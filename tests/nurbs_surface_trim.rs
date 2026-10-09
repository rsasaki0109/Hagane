use hagane::{KnotSide, NurbsFace, NurbsSurface, Point3, Surface, Tolerance, Transform};
fn source() -> NurbsSurface {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(i as f64, j as f64, (i * i + i * j + j) as f64));
            weights.push(1. + (3 * i + j) as f64 * 0.2);
        }
    }
    NurbsSurface::new(
        [2, 2],
        [
            vec![2., 2., 2., 6., 6., 6.],
            vec![-3., -3., -3., 5., 5., 5.],
        ],
        [3, 3],
        points,
        weights,
    )
    .unwrap()
    .insert_knot(0, 3.25, 1)
    .unwrap()
    .insert_knot(0, 5., 1)
    .unwrap()
    .insert_knot(1, 0.25, 1)
    .unwrap()
}
#[test]
fn rectangular_restriction_preserves_nonuniform_surface_and_partials() {
    let original = source();
    for ranges in [
        [[2.5, 5.5], [-2., 4.]],
        [[3.25, 5.], [0.25, 5.]],
        [[2., 6.], [-3., 5.]],
        [[2., 5.], [0.25, 4.]],
    ] {
        let trimmed = original.restricted(ranges).unwrap();
        assert_eq!(trimmed.domain(), ranges);
        assert_eq!(trimmed.degrees(), original.degrees());
        for i in 0..=20 {
            for j in 0..=20 {
                let u = ranges[0][0] + (ranges[0][1] - ranges[0][0]) * i as f64 / 20.;
                let v = ranges[1][0] + (ranges[1][1] - ranges[1][0]) * j as f64 / 20.;
                let a = original.partials(u, v).unwrap();
                let b = trimmed.partials(u, v).unwrap();
                assert!((a.point - b.point).norm() < 1e-11);
                assert!((a.du - b.du).norm() < 1e-11);
                assert!((a.dv - b.dv).norm() < 1e-11);
            }
        }
    }
}
#[test]
fn c0_cut_endpoints_retain_inward_derivative_limits() {
    let original = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![2., 2., 5., 5.]],
        [3, 2],
        vec![
            Point3::new(-1., 0., 0.),
            Point3::new(-1., 1., 0.),
            Point3::new(0., 0., 1.),
            Point3::new(0., 1., 1.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
        ],
        vec![1.; 6],
    )
    .unwrap();
    for (range, side) in [([0., 0.5], KnotSide::Left), ([0.5, 1.], KnotSide::Right)] {
        let trimmed = original.restricted([range, [2.5, 4.5]]).unwrap();
        let expected = original
            .evaluate_with_partials(0.5, 3., [side, KnotSide::Right])
            .unwrap();
        let actual = trimmed.partials(0.5, 3.).unwrap();
        assert!((expected.point - actual.point).norm() < 1e-14);
        assert!((expected.du - actual.du).norm() < 1e-14);
        assert!((expected.normal().unwrap() - actual.normal().unwrap()).norm() < 1e-14);
    }
}
#[test]
fn trimmed_face_regenerates_exact_boundary_and_preserves_orientation_and_placement() {
    let t = Tolerance::default();
    let face = NurbsFace::new(source(), -1, t).unwrap();
    let ranges = [[2.5, 5.5], [-2., 4.]];
    let trimmed = face.trimmed(ranges, t).unwrap();
    trimmed.validate_boundary(t).unwrap();
    assert_eq!(trimmed.face.orientation, -1);
    let Surface::Nurbs(surface) = &trimmed.face.surface else {
        panic!()
    };
    assert_eq!(surface.domain(), ranges);
    for edge in &trimmed.face.wires[0].coedges {
        let curve = &trimmed.edges[edge.edge].curve;
        let [a, b] = curve.range();
        for i in 0..=12 {
            let parameter = a + (b - a) * i as f64 / 12.;
            let uv = edge.pcurve.evaluate(parameter);
            assert!(
                (surface.evaluate(uv[0], uv[1]).unwrap() - curve.try_evaluate(parameter).unwrap())
                    .norm()
                    < 1e-12
            );
        }
    }
    let transform = Transform::translation(Point3::new(10., -20., 30.)).unwrap();
    let moved = face
        .transformed(transform, t)
        .unwrap()
        .trimmed(ranges, t)
        .unwrap();
    let direct = trimmed.transformed(transform, t).unwrap();
    for (a, b) in moved.vertices.iter().zip(&direct.vertices) {
        assert!((a.point - b.point).norm() < 1e-12);
    }
    let mesh = trimmed.tessellate_bounded(0.05, 4096, t).unwrap();
    assert!(mesh.error_bounds.iter().all(|b| *b <= 0.05));
    let mut invalid = face.clone();
    invalid.face.wires[0].coedges[0].edge = 3;
    assert!(invalid.trimmed(ranges, t).is_err());
}
#[test]
fn restriction_rejects_invalid_ranges_and_preflights_resources() {
    let s = source();
    for ranges in [
        [[2., 2.], [-3., 5.]],
        [[6., 2.], [-3., 5.]],
        [[1., 5.], [-3., 5.]],
        [[2., 7.], [-3., 5.]],
        [[f64::NAN, 5.], [-3., 5.]],
        [[2., f64::INFINITY], [-3., 5.]],
        [[2., 6.], [-4., 5.]],
    ] {
        assert!(s.restricted(ranges).is_err());
    }
    let count = 256;
    let mut k = vec![0., 0.];
    k.extend((1..count - 1).map(|i| i as f64));
    k.extend([count as f64, count as f64]);
    let huge = NurbsSurface::new(
        [1, 1],
        [k.clone(), k],
        [count, count],
        vec![Point3::new(0., 0., 0.); count * count],
        vec![1.; count * count],
    )
    .unwrap();
    assert!(matches!(
        huge.restricted([[0.25, 0.75], [0., 256.]]),
        Err(hagane::Error::Unsupported(
            "surface restriction exceeds refinement control limit"
        ))
    ));
}
#[test]
fn degree_sixteen_restriction_and_large_common_weight_scaling() {
    let p = 16;
    let mut k = vec![0.; p + 1];
    k.extend(vec![1.; p + 1]);
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..=p {
        for j in 0..2 {
            points.push(Point3::new(i as f64, j as f64, (i * i + j) as f64) * 1e-100);
            weights.push((1. + (i + j) as f64 / 10.) * 1e200);
        }
    }
    let s = NurbsSurface::new(
        [p, 1],
        [k, vec![2., 2., 4., 4.]],
        [p + 1, 2],
        points,
        weights,
    )
    .unwrap();
    let t = s.restricted([[0.2, 0.8], [2.25, 3.75]]).unwrap();
    assert_eq!(t.control_counts(), [p + 1, 2]);
    for i in 0..=16 {
        let u = 0.2 + 0.6 * i as f64 / 16.;
        let a = s.partials(u, 3.).unwrap();
        let b = t.partials(u, 3.).unwrap();
        assert!((a.point - b.point).norm() < 1e-110);
        assert!((a.du - b.du).norm() < 1e-109);
        assert!((a.dv - b.dv).norm() < 1e-110);
    }
}
#[test]
fn restriction_preflights_cumulative_work() {
    let p = 16;
    let nv = 1300;
    let mut u = vec![0.; p + 1];
    u.extend(vec![1.; p + 1]);
    let mut v = vec![0., 0.];
    v.extend((1..nv - 1).map(|i| i as f64));
    v.extend([nv as f64, nv as f64]);
    let surface = NurbsSurface::new(
        [p, 1],
        [u, v],
        [p + 1, nv],
        vec![Point3::new(0., 0., 0.); (p + 1) * nv],
        vec![1.; (p + 1) * nv],
    )
    .unwrap();
    assert!(matches!(
        surface.restricted([[0.2, 0.8], [0., nv as f64]]),
        Err(hagane::Error::Unsupported(
            "surface restriction exceeds refinement work limit"
        ))
    ));
}

#[test]
fn nested_restriction_matches_direct_and_rejects_outside_current_domain() {
    let source = source();
    let outer = source.restricted([[2.5, 5.5], [-2., 4.]]).unwrap();
    let ranges = [[3., 4.75], [-1., 3.]];
    let nested = outer.restricted(ranges).unwrap();
    let direct = source.restricted(ranges).unwrap();
    for i in 0..=20 {
        for j in 0..=20 {
            let u = 3. + 1.75 * i as f64 / 20.;
            let v = -1. + 4. * j as f64 / 20.;
            let a = nested.partials(u, v).unwrap();
            let b = direct.partials(u, v).unwrap();
            assert!((a.point - b.point).norm() < 1e-11);
            assert!((a.du - b.du).norm() < 1e-11);
            assert!((a.dv - b.dv).norm() < 1e-11);
        }
    }
    assert!(outer.restricted([[2.25, 5.], [-1., 3.]]).is_err());
}
#[test]
fn restricted_rational_bilinear_matches_independent_quotient_formula() {
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![2., 2., 6., 6.], vec![-3., -3., 5., 5.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 2., 1.),
            Point3::new(3., 0., 2.),
            Point3::new(3., 2., 5.),
        ],
        vec![1., 2., 5., 11.],
    )
    .unwrap();
    let cropped = surface.restricted([[2.5, 5.5], [-2., 4.]]).unwrap();
    for i in 0..=20 {
        for j in 0..=20 {
            let u = 2.5 + 3. * i as f64 / 20.;
            let v = -2. + 6. * j as f64 / 20.;
            let s = (u - 2.) / 4.;
            let t = (v + 3.) / 8.;
            let basis = [(1. - s) * (1. - t), (1. - s) * t, s * (1. - t), s * t];
            let bu = [-(1. - t) / 4., -t / 4., (1. - t) / 4., t / 4.];
            let bv = [-(1. - s) / 8., (1. - s) / 8., -s / 8., s / 8.];
            let mut n = Point3::new(0., 0., 0.);
            let mut du = n;
            let mut dv = n;
            let mut w = 0.;
            let mut wu = 0.;
            let mut wv = 0.;
            for k in 0..4 {
                let weight = surface.weights()[k];
                let point = surface.control_points()[k];
                n = n + point * (basis[k] * weight);
                w += basis[k] * weight;
                du = du + point * (bu[k] * weight);
                wu += bu[k] * weight;
                dv = dv + point * (bv[k] * weight);
                wv += bv[k] * weight;
            }
            let expected = n * (1. / w);
            let e = cropped.partials(u, v).unwrap();
            assert!((e.point - expected).norm() < 1e-12);
            assert!((e.du - (du - expected * wu) * (1. / w)).norm() < 1e-12);
            assert!((e.dv - (dv - expected * wv) * (1. / w)).norm() < 1e-12);
        }
    }
}
#[test]
fn adjacent_large_parameters_and_signed_zero_cut_sides_are_retained() {
    let a = 1e16;
    let b = a + 8.;
    let source = NurbsSurface::new(
        [1, 1],
        [vec![a, a, b, b], vec![2., 2., 4., 4.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(8., 0., 0.),
            Point3::new(8., 1., 0.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    let ranges = [[a + 2., a + 4.], [2.25, 3.75]];
    let cut = source.restricted(ranges).unwrap();
    assert_eq!(cut.domain(), ranges);
    for u in ranges[0] {
        let e = cut.partials(u, 3.).unwrap();
        assert_eq!(e.point, Point3::new(u - a, 0.5, 0.));
        assert_eq!(e.du, Point3::new(1., 0., 0.));
    }
    assert!(cut.tessellate_bounded(0.01, 1).is_ok());
    assert!(source.restricted([[a + 2., a + 2.], [2.25, 3.75]]).is_err());
    let mut points = Vec::new();
    for (x, z) in [(-1., 0.), (-0.5, 0.5), (0., 1.), (0.5, 0.5), (1., 0.)] {
        for y in [0., 1.] {
            points.push(Point3::new(x, y, z));
        }
    }
    let source = NurbsSurface::new(
        [2, 1],
        [
            vec![-1., -1., -1., -0.0, 0.0, 1., 1., 1.],
            vec![0., 0., 1., 1.],
        ],
        [5, 2],
        points,
        vec![1.; 10],
    )
    .unwrap();
    for (range, side) in [
        ([-0.75, -0.0], KnotSide::Left),
        ([0.0, 0.75], KnotSide::Right),
    ] {
        let cut = source.restricted([range, [0.2, 0.8]]).unwrap();
        assert_eq!(cut.domain()[0][0].to_bits(), range[0].to_bits());
        assert_eq!(cut.domain()[0][1].to_bits(), range[1].to_bits());
        let expected = source
            .evaluate_with_partials(0., 0.5, [side, KnotSide::Right])
            .unwrap();
        let actual = cut.partials(0., 0.5).unwrap();
        assert!((expected.point - actual.point).norm() < 1e-12);
        assert!((expected.du - actual.du).norm() < 1e-12);
        assert!((expected.normal().unwrap() - actual.normal().unwrap()).norm() < 1e-12);
    }
}
