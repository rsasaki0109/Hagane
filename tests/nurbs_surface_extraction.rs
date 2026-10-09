use hagane::{KnotSide, NurbsSurface, Point3};
fn knots(p: usize, a: f64, b: f64) -> Vec<f64> {
    let mut k = vec![a; p + 1];
    k.extend(vec![b; p + 1]);
    k
}
fn fixture(scale: f64, weight_scale: f64) -> NurbsSurface {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(i as f64, j as f64, (i * i + i * j + j) as f64) * scale);
            weights.push((1. + (3 * i + j) as f64 * 0.2) * weight_scale);
        }
    }
    NurbsSurface::new(
        [2, 2],
        [knots(2, 2., 6.), knots(2, -3., 5.)],
        [3, 3],
        points,
        weights,
    )
    .unwrap()
}
#[test]
fn extracted_nonuniform_patches_preserve_geometry_and_partials() {
    for (scale, weight_scale) in [(1., 1.), (1., 1e200), (1e-100, 1e200)] {
        let surface = fixture(scale, weight_scale)
            .insert_knot(0, 2.75, 1)
            .unwrap()
            .insert_knot(0, 5., 1)
            .unwrap()
            .insert_knot(1, 0.25, 1)
            .unwrap();
        let patches = surface.bezier_patches().unwrap();
        assert_eq!(patches.len(), 6);
        let mut area = 0.;
        for patch in patches {
            assert_eq!(patch.surface.control_counts(), [3, 3]);
            assert_eq!(patch.surface.domain(), patch.parameter_ranges);
            let [[a, b], [c, d]] = patch.parameter_ranges;
            area += (b - a) * (d - c);
            for i in 0..=12 {
                for j in 0..=12 {
                    let u = a + (b - a) * i as f64 / 12.;
                    let v = c + (d - c) * j as f64 / 12.;
                    let actual = surface
                        .evaluate_with_partials(u, v, [KnotSide::Right; 2])
                        .unwrap();
                    let expected = patch.surface.partials(u, v).unwrap();
                    assert!((actual.point - expected.point).norm() < scale * 1e-11);
                    assert!((actual.du - expected.du).norm() < scale * 1e-11);
                    assert!((actual.dv - expected.dv).norm() < scale * 1e-11);
                }
            }
        }
        assert_eq!(area, 32.);
    }
}
#[test]
fn c0_patch_extraction_retains_one_sided_partial_limits() {
    let surface = fixture(1., 1.).insert_knot(0, 4., 2).unwrap();
    let patches = surface.bezier_patches().unwrap();
    assert_eq!(patches.len(), 2);
    for (i, patch) in patches.iter().enumerate() {
        let side = if i == 0 {
            KnotSide::Left
        } else {
            KnotSide::Right
        };
        let a = surface
            .evaluate_with_partials(4., 1., [side, KnotSide::Right])
            .unwrap();
        let b = patch.surface.partials(4., 1.).unwrap();
        assert!((a.point - b.point).norm() < 1e-12);
        assert!((a.du - b.du).norm() < 1e-12);
    }
}
#[test]
fn degree_one_existing_full_multiplicity_skips_refinement() {
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 3., 3.], vec![2., 2., 5., 5.]],
        [3, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1., 0., 1.),
            Point3::new(1., 1., 1.),
            Point3::new(3., 0., 0.),
            Point3::new(3., 1., 0.),
        ],
        vec![1., 2., 3., 4., 5., 6.],
    )
    .unwrap();
    let patches = surface.bezier_patches().unwrap();
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].surface.weights(), &surface.weights()[..4]);
    assert_eq!(patches[1].surface.weights(), &surface.weights()[2..]);
}
#[test]
fn degree_sixteen_and_invalid_resource_requests() {
    let degree = 16;
    let mut points = Vec::new();
    for i in 0..=degree {
        for j in 0..2 {
            points.push(Point3::new(i as f64, j as f64, (i * i + j) as f64));
        }
    }
    let surface = NurbsSurface::new(
        [degree, 1],
        [knots(degree, 0., 1.), knots(1, 0., 1.)],
        [degree + 1, 2],
        points,
        vec![1.; (degree + 1) * 2],
    )
    .unwrap()
    .insert_knot(0, 0.35, 1)
    .unwrap();
    let patches = surface.bezier_patches().unwrap();
    assert_eq!(patches.len(), 2);
    for patch in &patches {
        let [a, b] = patch.parameter_ranges[0];
        for i in 0..=10 {
            let u = a + (b - a) * i as f64 / 10.;
            assert!(
                (surface.evaluate(u, 0.3).unwrap() - patch.surface.evaluate(u, 0.3).unwrap())
                    .norm()
                    < 1e-10
            );
        }
    }
    let count = 250;
    let mut axis = vec![0.; 3];
    axis.extend((1..count - 2).map(|i| i as f64));
    axis.extend(vec![count as f64; 3]);
    let huge = NurbsSurface::new(
        [2, 2],
        [axis.clone(), axis],
        [count, count],
        vec![Point3::new(0., 0., 0.); count * count],
        vec![1.; count * count],
    )
    .unwrap();
    assert!(matches!(
        huge.bezier_patches(),
        Err(hagane::Error::Unsupported(
            "surface Bezier extraction exceeds 65536 controls"
        ))
    ));
}
#[test]
fn extraction_preflights_cumulative_refinement_work() {
    let p = 16;
    let count = 500;
    let mut u = vec![0.; p + 1];
    u.extend((1..count - p).map(|i| i as f64));
    u.extend(vec![count as f64; p + 1]);
    let surface = NurbsSurface::new(
        [p, 1],
        [u, knots(1, 0., 1.)],
        [count, 2],
        vec![Point3::new(0., 0., 0.); count * 2],
        vec![1.; count * 2],
    )
    .unwrap();
    assert!(matches!(
        surface.bezier_patches(),
        Err(hagane::Error::Unsupported(
            "surface Bezier extraction exceeds work limit"
        ))
    ));
}
