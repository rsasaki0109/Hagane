use hagane::{KnotSide, NurbsSurface, NurbsSurfaceEdge, PCurve, Point3, Tolerance};

fn rational() -> NurbsSurface {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                i as f64 * 0.5,
                j as f64 * 0.5,
                (i * j) as f64 * 0.2 + (i * i) as f64 * 0.1,
            ));
            weights.push(1. + ((i * 3 + j) % 5) as f64 * 0.2);
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
}

#[test]
fn nonunit_rational_uv_line_matches_independent_tensor_formula_and_chain_rule() {
    let surface = rational();
    for (start, end) in [
        ([2.4, -2.6], [5.6, 4.6]),
        ([5.6, 4.6], [2.4, -2.6]),
        ([3.2, -2.], [3.2, 4.]),
    ] {
        let curve = surface.parameter_curve(start, end).unwrap();
        assert_eq!(curve.domain(), [0., 1.]);
        assert_eq!(curve.degree(), if start[0] == end[0] { 2 } else { 4 });
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let uv =
                std::array::from_fn::<_, 2, _>(|axis| start[axis] + (end[axis] - start[axis]) * t);
            let u = (uv[0] - 2.) / 4.;
            let v = (uv[1] + 3.) / 8.;
            let bu = [(1. - u).powi(2), 2. * u * (1. - u), u * u];
            let bv = [(1. - v).powi(2), 2. * v * (1. - v), v * v];
            let mut num = Point3::new(0., 0., 0.);
            let mut den = 0.;
            for (a, &x) in bu.iter().enumerate() {
                for (b, &y) in bv.iter().enumerate() {
                    let index = a * 3 + b;
                    let w = x * y * surface.weights()[index];
                    num = num + surface.control_points()[index] * w;
                    den += w;
                }
            }
            let (point, tangent) = curve.evaluate_with_derivative(t, KnotSide::Right).unwrap();
            assert!((point - num * (1. / den)).norm() < 2e-12);
            let jet = surface.partials(uv[0], uv[1]).unwrap();
            assert!(
                (tangent - jet.du * (end[0] - start[0]) - jet.dv * (end[1] - start[1])).norm()
                    < 2e-11
            );
        }
        let mesh = curve.tessellate_bounded(0.001, 4096).unwrap();
        for (k, bound) in mesh.error_bounds.iter().enumerate() {
            let a = mesh.points[k];
            let b = mesh.points[k + 1];
            let direction = (b - a).normalized().unwrap();
            let length = (b - a).norm();
            for j in 0..=20 {
                let t = mesh.parameters[k]
                    + (mesh.parameters[k + 1] - mesh.parameters[k]) * j as f64 / 20.;
                let uv = [
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ];
                let p = surface.evaluate(uv[0], uv[1]).unwrap();
                let closest = a + direction * (p - a).dot(direction).clamp(0., length);
                assert!((p - closest).norm() <= *bound + 2e-12);
            }
        }
    }
}

#[test]
fn simultaneous_c0_crossings_reverse_source_derivative_limits() {
    let mut points = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                [-2., 0., 4.][i],
                [-2., 0., 2.][j],
                [0., 1., 0.][i] + [0., 1., 0.][j],
            ));
        }
    }
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![-2., -2., 1., 5., 5.], vec![3., 3., 6., 9., 9.]],
        [3, 3],
        points,
        vec![1.; 9],
    )
    .unwrap();
    for (start, end, reverse) in [([-1., 4.], [3., 8.], false), ([3., 8.], [-1., 4.], true)] {
        let curve = surface.parameter_curve(start, end).unwrap();
        assert_eq!(curve.degree(), 2);
        assert_eq!(curve.knots().iter().filter(|u| **u == 0.5).count(), 2);
        assert!(curve.derivative(0.5).is_err());
        for side in [KnotSide::Left, KnotSide::Right] {
            let source_side = if reverse {
                match side {
                    KnotSide::Left => KnotSide::Right,
                    KnotSide::Right => KnotSide::Left,
                }
            } else {
                side
            };
            let expected = surface
                .evaluate_with_partials(1., 6., [source_side; 2])
                .unwrap();
            let (p, d) = curve.evaluate_with_derivative(0.5, side).unwrap();
            assert!((p - expected.point).norm() < 1e-13);
            assert!(
                (d - expected.du * (end[0] - start[0]) - expected.dv * (end[1] - start[1])).norm()
                    < 1e-12
            );
        }
    }
}

#[test]
fn surface_edge_rejects_dirty_boundary_and_invalid_uv_paths() {
    let source = rational();
    let t = Tolerance::default();
    let start = [2.4, -2.6];
    let end = [5.6, 4.6];
    let edge = NurbsSurfaceEdge::new(source.clone(), start, end, t).unwrap();
    edge.validate(t).unwrap();
    for k in 0..=20 {
        let parameter = k as f64 / 20.;
        let uv = edge.pcurve.evaluate(parameter);
        assert_eq!(
            uv,
            [
                start[0] + (end[0] - start[0]) * parameter,
                start[1] + (end[1] - start[1]) * parameter
            ]
        );
        assert!(
            (edge.edge.curve.try_evaluate(parameter).unwrap()
                - source.evaluate(uv[0], uv[1]).unwrap())
            .norm()
                < 2e-12
        );
    }
    let mut invalid = edge.clone();
    invalid.edge.vertices = [0, 0];
    assert!(invalid.validate(t).is_err());
    let mut invalid = edge.clone();
    invalid.vertices[0].point.x += 0.01;
    assert!(invalid.validate(t).is_err());
    let mut invalid = edge.clone();
    let PCurve::Affine { origin, .. } = &mut invalid.pcurve else {
        panic!()
    };
    origin[0] += 0.01;
    assert!(invalid.validate(t).is_err());
    for (a, b) in [
        (start, start),
        ([1., 0.], end),
        (start, [6.1, 0.]),
        ([f64::NAN, 0.], end),
        (start, [3., f64::INFINITY]),
    ] {
        assert!(source.parameter_curve(a, b).is_err());
        assert!(NurbsSurfaceEdge::new(source.clone(), a, b, t).is_err());
    }
}

#[test]
fn rounded_affine_uv_evaluation_cannot_masquerade_as_matching_edge() {
    let a = 1e16;
    let b = a + 8.;
    let source = NurbsSurface::new(
        [1, 1],
        [vec![a, a, b, b], vec![0., 0., 1., 1.]],
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
    // At t=1/8, a+8*t rounds back to a, despite a whole spatial unit of travel.
    let pcurve = PCurve::Affine {
        origin: [a, 0.25],
        direction: [8., 0.5],
    };
    assert_eq!(pcurve.evaluate(0.125)[0], a);
    assert_eq!(source.evaluate(a, 0.3125).unwrap().x, 0.);
    assert!(source.parameter_curve([a, 0.25], [b, 0.75]).is_err());
    assert!(NurbsSurfaceEdge::new(source, [a, 0.25], [b, 0.75], Tolerance::default()).is_err());
}

#[test]
fn sufficiently_long_uv_direction_still_needs_spatial_consistency_guard() {
    let a = 1e16;
    let b = a + 1024.;
    let surface = NurbsSurface::new(
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
    // Relative UV resolution passes the composition guard, but a spatial unit
    // disappears in the affine UV evaluation at t=1/1024.
    let uv = PCurve::Affine {
        origin: [a, 0.25],
        direction: [1024., 0.5],
    }
    .evaluate(1. / 1024.);
    assert_eq!(uv[0], a);
    assert_eq!(surface.evaluate(uv[0], uv[1]).unwrap().x, 0.);
    assert!(NurbsSurfaceEdge::new(surface, [a, 0.25], [b, 0.75], Tolerance::default()).is_err());
}

#[test]
fn canonical_control_deviation_and_uv_roundoff_share_one_physical_budget() {
    use hagane::{Curve, NurbsCurve};
    let a = 1e16;
    let b = a + 1024.;
    let tolerance = Tolerance::default();
    let surface = NurbsSurface::new(
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
    let mut edge = NurbsSurfaceEdge::new(surface.clone(), [a, 0.25], [b, 0.75], tolerance).unwrap();
    let Curve::Nurbs(curve) = &edge.edge.curve else {
        panic!()
    };
    let shifted = curve
        .control_points()
        .iter()
        .map(|p| *p + Point3::new(0., 0., tolerance.linear))
        .collect();
    edge.edge.curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            shifted,
            curve.weights().to_vec(),
        )
        .unwrap(),
    ));
    let uv = edge.pcurve.evaluate(1. / 1024.);
    let gap = (edge.edge.curve.try_evaluate(1. / 1024.).unwrap()
        - surface.evaluate(uv[0], uv[1]).unwrap())
    .norm();
    assert!(gap > tolerance.linear);
    assert!(edge.validate(tolerance).is_err());
    assert!(edge.tessellate_bounded(0.001, 16, tolerance).is_err());
}

#[test]
fn world_coordinate_evaluation_roundoff_is_part_of_physical_identity_budget() {
    let offset = 1e12;
    let points = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                Point3::new(
                    offset + i as f64,
                    offset + j as f64,
                    offset + (i * j) as f64,
                )
            })
        })
        .collect();
    let knots = vec![0., 0., 0., 1., 1., 1.];
    let source =
        NurbsSurface::new([2, 2], [knots.clone(), knots], [3, 3], points, vec![1.; 9]).unwrap();
    // Original UV units are benign, but the supporting surface and the composed
    // rational curve take different rounded evaluation paths at a remote origin.
    assert!(NurbsSurfaceEdge::new(source, [0., 0.], [1., 1.], Tolerance::default()).is_err());
}
