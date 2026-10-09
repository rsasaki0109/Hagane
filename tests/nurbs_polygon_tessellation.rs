use hagane::*;
use std::collections::HashMap;
fn face(orientation: i8) -> NurbsPolygonFace {
    let s = NurbsSurface::new(
        [1, 1],
        [vec![2., 2., 6., 6.], vec![-3., -3., 5., 5.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 10., 0.),
            Point3::new(10., 0., 0.),
            Point3::new(10., 10., 8.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    NurbsPolygonFace::new(
        s,
        vec![[2.5, -2.], [5.5, -1.], [5., 3.], [3., 4.]],
        orientation,
        Tolerance::default(),
    )
    .unwrap()
}
#[test]
fn curved_triangles_respect_independent_bilinear_formula_and_bounds() {
    let f = face(1);
    let display = f
        .tessellate_bilinear_bounded(0.01, 65536, Tolerance::default())
        .unwrap();
    assert!(display.mesh.triangles.len() > 2);
    assert_eq!(display.error_bounds.len(), display.mesh.triangles.len());
    for (i, t) in display.mesh.triangles.iter().enumerate() {
        assert!(display.error_bounds[i] <= 0.01);
        for a in 0..=8 {
            for b in 0..=8 - a {
                let w = [a as f64 / 8., b as f64 / 8., (8 - a - b) as f64 / 8.];
                let mut uv = [0.; 2];
                let mut point = Vec3::new(0., 0., 0.);
                for j in 0..3 {
                    for (axis, value) in uv.iter_mut().enumerate() {
                        *value += display.vertex_uv[t[j]][axis] * w[j];
                    }
                    point = point + display.mesh.positions[t[j]] * w[j];
                }
                let u = (uv[0] - 2.) / 4.;
                let v = (uv[1] + 3.) / 8.;
                let expected = Point3::new(10. * u, 10. * v, 8. * u * v);
                assert!((expected - point).norm() <= display.error_bounds[i]);
            }
        }
    }
}
#[test]
fn subdivision_is_conforming_covers_uv_polygon_and_preserves_orientation() {
    let a = face(1)
        .tessellate_bilinear_bounded(0.03, 65536, Tolerance::default())
        .unwrap();
    let b = face(-1)
        .tessellate_bilinear_bounded(0.03, 65536, Tolerance::default())
        .unwrap();
    assert_eq!(a.vertex_uv, b.vertex_uv);
    let mut uses = HashMap::new();
    let mut area = 0.;
    for (i, t) in a.mesh.triangles.iter().enumerate() {
        assert_eq!(b.mesh.triangles[i], [t[0], t[2], t[1]]);
        let [p, q, r] = t.map(|id| a.vertex_uv[id]);
        area += ((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])) / 2.;
        for (x, y) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if x < y { [x, y] } else { [y, x] };
            let entry = uses.entry(key).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += if x < y { 1 } else { -1 };
        }
    }
    let uv = face(1).boundary.uv_corners().to_vec();
    let expected: f64 = (0..uv.len())
        .map(|i| {
            let j = (i + 1) % uv.len();
            (uv[i][0] * uv[j][1] - uv[j][0] * uv[i][1]) / 2.
        })
        .sum();
    assert!((area - expected).abs() < 1e-12);
    for (edge, (count, sign)) in uses {
        assert!(count == 1 || (count == 2 && sign == 0));
        if count == 1 {
            let p = a.vertex_uv[edge[0]];
            let q = a.vertex_uv[edge[1]];
            assert!(
                (0..uv.len()).any(|i| orient2d(uv[i], uv[(i + 1) % uv.len()], p).unwrap()
                    == Orientation::Collinear
                    && orient2d(uv[i], uv[(i + 1) % uv.len()], q).unwrap()
                        == Orientation::Collinear)
            );
        }
    }
    for (x, y) in a.mesh.normals.iter().zip(b.mesh.normals) {
        assert!((*x + y).norm() < 1e-12);
    }
}
#[test]
fn precision_budget_and_unsupported_multispan_patch_are_explicit() {
    let f = face(1);
    assert!(f
        .tessellate_bilinear_bounded(0.001, 2, Tolerance::default())
        .is_err());
    assert!(f
        .tessellate_bilinear_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
    assert!(f
        .tessellate_bilinear_bounded(f64::NAN, 65536, Tolerance::default())
        .is_err());
    let s = &f.boundary.surface;
    let rational = NurbsSurface::new(
        [1, 1],
        [s.knots(0).unwrap().to_vec(), s.knots(1).unwrap().to_vec()],
        [2, 2],
        s.control_points().to_vec(),
        vec![1., 2., 1., 1.],
    )
    .unwrap();
    let rational = rational.insert_knot(0, 4., 1).unwrap();
    let f = NurbsPolygonFace::new(
        rational,
        f.boundary.uv_corners().to_vec(),
        1,
        Tolerance::default(),
    )
    .unwrap();
    assert!(matches!(
        f.tessellate_bilinear_bounded(0.1, 65536, Tolerance::default()),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn rational_triangle_bounds_match_independent_weighted_formula() {
    let original = face(1);
    let weights = [1., 1.25, 1.5, 1.];
    for scale in [1., 1e100, 1e-100] {
        let s = &original.boundary.surface;
        let rational = NurbsSurface::new(
            [1, 1],
            [s.knots(0).unwrap().to_vec(), s.knots(1).unwrap().to_vec()],
            [2, 2],
            s.control_points().to_vec(),
            weights.map(|w| w * scale).to_vec(),
        )
        .unwrap();
        let face = NurbsPolygonFace::new(
            rational,
            original.boundary.uv_corners().to_vec(),
            1,
            Tolerance::default(),
        )
        .unwrap();
        let display = face
            .tessellate_bilinear_bounded(0.1, 65536, Tolerance::default())
            .unwrap();
        for (i, t) in display.mesh.triangles.iter().enumerate() {
            assert!(display.error_bounds[i] <= 0.1);
            for a in 0..=4 {
                for b in 0..=4 - a {
                    let bary = [a as f64 / 4., b as f64 / 4., (4 - a - b) as f64 / 4.];
                    let mut uv = [0.; 2];
                    let mut chord = Vec3::new(0., 0., 0.);
                    for j in 0..3 {
                        for (axis, v) in uv.iter_mut().enumerate() {
                            *v += display.vertex_uv[t[j]][axis] * bary[j];
                        }
                        chord = chord + display.mesh.positions[t[j]] * bary[j];
                    }
                    let u = (uv[0] - 2.) / 4.;
                    let v = (uv[1] + 3.) / 8.;
                    let basis = [(1. - u) * (1. - v), (1. - u) * v, u * (1. - v), u * v];
                    let denominator: f64 = (0..4).map(|j| basis[j] * weights[j]).sum();
                    let expected = Point3::new(
                        10. * (basis[2] * weights[2] + basis[3] * weights[3]) / denominator,
                        10. * (basis[1] * weights[1] + basis[3] * weights[3]) / denominator,
                        8. * basis[3] * weights[3] / denominator,
                    );
                    assert!((expected - chord).norm() <= display.error_bounds[i]);
                }
            }
        }
    }
}

#[test]
fn higher_degree_rational_bounds_match_independent_bernstein_surface() {
    fn bernstein(n: usize, i: usize, t: f64) -> f64 {
        let mut choose = 1.;
        for j in 0..i {
            choose *= (n - j) as f64 / (j + 1) as f64;
        }
        choose * t.powi(i as i32) * (1. - t).powi((n - i) as i32)
    }
    for degrees in [[2, 2], [3, 1], [1, 3]] {
        let [p, q] = degrees;
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for i in 0..=p {
            for j in 0..=q {
                points.push(Point3::new(
                    i as f64 * 6. / p as f64,
                    j as f64 * 6. / q as f64,
                    (i * j) as f64 + if i == 1 && j == 1 { 2. } else { 0. },
                ));
                weights.push(1. + 0.05 * (i + j) as f64);
            }
        }
        let domains = [[2., 6.], [-3., 5.]];
        let knots = |degree: usize, domain: [f64; 2]| {
            let mut k = vec![domain[0]; degree + 1];
            k.extend(vec![domain[1]; degree + 1]);
            k
        };
        let s = NurbsSurface::new(
            degrees,
            [knots(p, domains[0]), knots(q, domains[1])],
            [p + 1, q + 1],
            points.clone(),
            weights.clone(),
        )
        .unwrap();
        let face = NurbsPolygonFace::new(
            s,
            vec![[2.5, -2.], [5.5, -1.], [3., 4.]],
            1,
            Tolerance::default(),
        )
        .unwrap();
        assert!(matches!(
            face.tessellate_bilinear_bounded(0.1, 65536, Tolerance::default()),
            Err(Error::Unsupported(_))
        ));
        let display = face
            .tessellate_bounded(0.1, 65536, Tolerance::default())
            .unwrap();
        for (index, t) in display.mesh.triangles.iter().enumerate() {
            assert!(display.error_bounds[index] <= 0.1);
            for a in 0..=4 {
                for b in 0..=4 - a {
                    let bary = [a as f64 / 4., b as f64 / 4., (4 - a - b) as f64 / 4.];
                    let mut uv = [0.; 2];
                    let mut chord = Vec3::new(0., 0., 0.);
                    for j in 0..3 {
                        for (axis, v) in uv.iter_mut().enumerate() {
                            *v += display.vertex_uv[t[j]][axis] * bary[j];
                        }
                        chord = chord + display.mesh.positions[t[j]] * bary[j];
                    }
                    let mut numerator = Vec3::new(0., 0., 0.);
                    let mut denominator = 0.;
                    for i in 0..=p {
                        for j in 0..=q {
                            let id = i * (q + 1) + j;
                            let w = bernstein(p, i, (uv[0] - 2.) / 4.)
                                * bernstein(q, j, (uv[1] + 3.) / 8.)
                                * weights[id];
                            numerator = numerator + points[id] * w;
                            denominator += w;
                        }
                    }
                    assert!(
                        (numerator * (1. / denominator) - chord).norm()
                            <= display.error_bounds[index]
                    );
                }
            }
        }
    }
}
