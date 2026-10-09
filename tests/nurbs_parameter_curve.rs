use hagane::{Curve, KnotSide, NurbsSurface, NurbsSurfaceEdge, Point3, Tolerance};
fn patch() -> NurbsSurface {
    let k = vec![2., 2., 2., 6., 6., 6.];
    let v = vec![-3., -3., -3., 5., 5., 5.];
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(i as f64, j as f64, (i * j + i + j) as f64));
            weights.push(1. + (3 * i + j) as f64 * 0.2);
        }
    }
    NurbsSurface::new([2, 2], [k, v], [3, 3], points, weights).unwrap()
}
fn verify(surface: &NurbsSurface, start: [f64; 2], end: [f64; 2]) {
    let c = surface.parameter_curve(start, end).unwrap();
    assert_eq!(c.domain(), [0., 1.]);
    let d = [end[0] - start[0], end[1] - start[1]];
    for i in 0..=200 {
        let t = i as f64 / 200.;
        let uv = [start[0] + d[0] * t, start[1] + d[1] * t];
        let e = surface
            .evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])
            .unwrap();
        let (p, tangent) = c.evaluate_with_derivative(t, KnotSide::Right).unwrap();
        assert!((p - e.point).norm() < 2e-11);
        assert!((tangent - e.du * d[0] - e.dv * d[1]).norm() < 2e-10);
    }
}
#[test]
fn rational_diagonal_reverse_and_isoparametric_composition() {
    let s = patch();
    for (a, b) in [
        ([2.25, -2.5], [5.75, 4.5]),
        ([5.75, 4.5], [2.25, -2.5]),
        ([4., -2.5], [4., 4.5]),
        ([2.25, 1.], [5.75, 1.]),
    ] {
        verify(&s, a, b);
    }
    let c = s.parameter_curve([2., -3.], [6., 5.]).unwrap();
    assert_eq!(c.degree(), 4);
}
#[test]
fn multispan_and_true_simultaneous_crossings_preserve_curve() {
    let s = patch()
        .insert_knot(0, 3.5, 1)
        .unwrap()
        .insert_knot(1, 0., 1)
        .unwrap();
    verify(&s, [2., -3.], [6., 5.]);
    verify(&s, [6., 5.], [2., -3.]);
    let c = s.parameter_curve([2., -3.], [6., 5.]).unwrap();
    assert_eq!(c.bezier_spans().unwrap().len(), 2);
}
#[test]
fn c0_crossing_derivative_limits_follow_segment_traversal() {
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 1., 1.]],
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
    for (start, end) in [([0., 0.1], [1., 0.9]), ([1., 0.9], [0., 0.1])] {
        let curve = s.parameter_curve(start, end).unwrap();
        assert!(curve.derivative(0.5).is_err());
        for side in [KnotSide::Left, KnotSide::Right] {
            let u_side = if start[0] < end[0] {
                side
            } else {
                match side {
                    KnotSide::Left => KnotSide::Right,
                    KnotSide::Right => KnotSide::Left,
                }
            };
            let e = s
                .evaluate_with_partials(0.5, 0.5, [u_side, KnotSide::Right])
                .unwrap();
            let (_, derivative) = curve.evaluate_with_derivative(0.5, side).unwrap();
            assert!(
                (derivative - e.du * (end[0] - start[0]) - e.dv * (end[1] - start[1])).norm()
                    < 1e-12
            );
        }
    }
}
#[test]
fn retained_edge_uses_affine_pcurve_and_rejects_modified_topology() {
    let t = Tolerance::default();
    let edge = NurbsSurfaceEdge::new(patch(), [2.25, -2.5], [5.75, 4.5], t).unwrap();
    edge.validate(t).unwrap();
    let curve = edge.tessellate_bounded(0.001, 4096, t).unwrap();
    assert!(curve.error_bounds.iter().all(|b| *b <= 0.001));
    for (point, p) in curve.points.iter().zip(curve.parameters) {
        let uv = edge.pcurve.evaluate(p);
        assert!((*point - edge.surface.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-11);
    }
    let mut dirty = edge.clone();
    dirty.edge.vertices = [1, 0];
    assert!(dirty.validate(t).is_err());
    let mut dirty = edge;
    let Curve::Nurbs(c) = &dirty.edge.curve else {
        panic!()
    };
    let shifted = hagane::NurbsCurve::new(
        c.degree(),
        c.knots().to_vec(),
        c.control_points()
            .iter()
            .map(|p| *p - Point3::new(t.linear * 0.75, 0., 0.))
            .collect(),
        c.weights().to_vec(),
    )
    .unwrap();
    dirty.edge.curve = Curve::Nurbs(Box::new(shifted));
    for v in &mut dirty.vertices {
        v.point = v.point + Point3::new(t.linear * 0.75, 0., 0.);
    }
    assert!(dirty.validate(t).is_err());
}
#[test]
fn rejects_invalid_unresolved_and_excessive_degree() {
    let s = patch();
    for (a, b) in [
        ([2., -3.], [2., -3.]),
        ([1., 0.], [5., 4.]),
        ([2., -3.], [f64::NAN, 4.]),
    ] {
        assert!(s.parameter_curve(a, b).is_err());
    }
    let s = patch()
        .insert_knot(0, 4., 1)
        .unwrap()
        .insert_knot(1, 1. + 1e-14, 1)
        .unwrap();
    assert!(s.parameter_curve([2., -3.], [6., 5.]).is_err());
    let p = 9;
    let mut k = vec![0.; p + 1];
    k.extend(vec![1.; p + 1]);
    let points = (0..=p)
        .flat_map(|i| (0..=p).map(move |j| Point3::new(i as f64, j as f64, 0.)))
        .collect();
    let s = NurbsSurface::new(
        [p, p],
        [k.clone(), k],
        [p + 1, p + 1],
        points,
        vec![1.; (p + 1) * (p + 1)],
    )
    .unwrap();
    assert!(s.parameter_curve([0., 0.], [1., 1.]).is_err());
    assert_eq!(s.parameter_curve([0.5, 0.], [0.5, 1.]).unwrap().degree(), p);
}

#[test]
fn independent_rational_bilinear_formula_and_chain_rule() {
    let s = NurbsSurface::new(
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
    let c = s.parameter_curve([2.25, -2.5], [5.75, 4.5]).unwrap();
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let u = (0.25 + 3.5 * t) / 4.;
        let v = (0.5 + 7. * t) / 8.;
        let du = 3.5 / 4.;
        let dv = 7. / 8.;
        let basis = [(1. - u) * (1. - v), (1. - u) * v, u * (1. - v), u * v];
        let derivatives = [
            -du * (1. - v) - (1. - u) * dv,
            -du * v + (1. - u) * dv,
            du * (1. - v) - u * dv,
            du * v + u * dv,
        ];
        let mut n = Point3::new(0., 0., 0.);
        let mut dn = n;
        let mut w = 0.;
        let mut dw = 0.;
        for k in 0..4 {
            let weight = s.weights()[k];
            n = n + s.control_points()[k] * (basis[k] * weight);
            dn = dn + s.control_points()[k] * (derivatives[k] * weight);
            w += basis[k] * weight;
            dw += derivatives[k] * weight;
        }
        let p = n * (1. / w);
        let expected = (dn - p * dw) * (1. / w);
        let (actual, d) = c.evaluate_with_derivative(t, KnotSide::Right).unwrap();
        assert!((actual - p).norm() < 1e-12);
        assert!((d - expected).norm() < 1e-12);
    }
}
#[test]
fn local_homogeneous_underflow_large_uv_origins_and_span_resources_reject() {
    let base = 1e-300;
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(base, 0., 0.),
            Point3::new(base, 0., 0.),
            Point3::new(base + 1e-310, 0., 0.),
            Point3::new(base + 1e-310, 0., 0.),
        ],
        vec![1., 1., 1e-20, 1e-20],
    )
    .unwrap();
    assert!(matches!(
        s.parameter_curve([0., 0.], [1., 1.]),
        Err(hagane::Error::InvalidInput(
            "surface parameter curve weighted local control underflows"
        ))
    ));
    let a = 1e16;
    let s = NurbsSurface::new(
        [1, 1],
        [vec![a, a, a + 8., a + 8.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    assert!(s.parameter_curve([a, 0.], [a + 8., 1.]).is_err());
    let n = 4100;
    let mut k = vec![0., 0.];
    k.extend((1..n - 1).map(|i| i as f64));
    k.extend([n as f64, n as f64]);
    let s = NurbsSurface::new(
        [1, 1],
        [k, vec![0., 0., 1., 1.]],
        [n, 2],
        vec![Point3::new(0., 0., 0.); n * 2],
        vec![1.; n * 2],
    )
    .unwrap();
    assert!(matches!(
        s.parameter_curve([0., 0.], [n as f64, 0.]),
        Err(hagane::Error::Unsupported(
            "surface parameter curve exceeds span or control limit"
        ))
    ));
}

#[test]
fn pcurve_roundoff_is_checked_in_physical_tolerance_units() {
    let a = 1e16;
    let b = a + 1024.;
    let source = NurbsSurface::new(
        [1, 1],
        [vec![a, a, b, b], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1024., 0., 0.),
            Point3::new(1024., 1., 0.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    // Mathematical Bernstein composition is well defined, but the floating UV
    // map loses a unit of physical distance at t=1/1024. It cannot be a checked edge.
    let c = source.parameter_curve([a, 0.25], [b, 0.75]).unwrap();
    let t = 1. / 1024.;
    let uv = [a + (b - a) * t, 0.25 + 0.5 * t];
    assert!((c.evaluate(t).unwrap() - source.evaluate(uv[0], uv[1]).unwrap()).norm() > 0.9);
    assert!(matches!(
        NurbsSurfaceEdge::new(source, [a, 0.25], [b, 0.75], Tolerance::default()),
        Err(hagane::Error::Unsupported(
            "affine UV roundoff cannot preserve surface edge identity within physical tolerance"
        ))
    ));
}

#[test]
fn canonical_curve_displacement_reserves_uv_identity_budget() {
    let a = 1e16;
    let b = a + 1024.;
    let source = NurbsSurface::new(
        [1, 1],
        [vec![a, a, b, b], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1e-8, 0.),
            Point3::new(1e-8, 0., 0.),
            Point3::new(1e-8, 1e-8, 0.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    let tol = Tolerance::default();
    let mut edge = NurbsSurfaceEdge::new(source, [a, 0.25], [b, 0.75], tol).unwrap();
    let Curve::Nurbs(curve) = &edge.edge.curve else {
        panic!()
    };
    let displaced = hagane::NurbsCurve::new(
        curve.degree(),
        curve.knots().to_vec(),
        curve
            .control_points()
            .iter()
            .map(|point| *point + Point3::new(0., 0., tol.linear))
            .collect(),
        curve.weights().to_vec(),
    )
    .unwrap();
    edge.edge.curve = Curve::Nurbs(Box::new(displaced));
    assert!(matches!(
        edge.validate(tol),
        Err(hagane::Error::InvalidTopology(
            "surface edge control displacement exhausts parameter identity tolerance"
        ))
    ));
}

#[test]
fn world_coordinate_homogeneous_arithmetic_reserves_physical_identity() {
    let k = vec![0., 0., 0., 1., 1., 1.];
    let mut controls = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            controls.push(Point3::new(
                1e12 + i as f64,
                1e12 + j as f64,
                1e12 + (i * j) as f64,
            ));
        }
    }
    let source = NurbsSurface::new([2, 2], [k.clone(), k], [3, 3], controls, vec![1.; 9]).unwrap();
    let curve = source.parameter_curve([0., 0.], [1., 1.]).unwrap();
    let error = (0..=100)
        .map(|i| {
            let t = i as f64 / 100.;
            (curve.evaluate(t).unwrap() - source.evaluate(t, t).unwrap()).norm()
        })
        .fold(0f64, f64::max);
    assert!(error > 1e-4);
    assert!(NurbsSurfaceEdge::new(source, [0., 0.], [1., 1.], Tolerance::default()).is_err());
}
