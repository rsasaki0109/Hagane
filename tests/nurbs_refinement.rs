use hagane::{KnotSide, NurbsCurve, Point3};
fn circle() -> NurbsCurve {
    NurbsCurve::new(
        2,
        vec![2., 2., 2., 6., 6., 6.],
        vec![
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
            Point3::new(0., 1., 0.),
        ],
        vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
    )
    .unwrap()
}
#[test]
fn insertion_preserves_shape_and_derivative() {
    let original = circle();
    let refined = original
        .insert_knot(3.25, 2)
        .unwrap()
        .insert_knot(5., 1)
        .unwrap();
    assert_eq!(refined.domain(), [2., 6.]);
    for i in 0..=400 {
        let u = 2. + i as f64 / 100.;
        for side in [KnotSide::Left, KnotSide::Right] {
            let (p, d) = original.evaluate_with_derivative(u, side).unwrap();
            let (q, e) = refined.evaluate_with_derivative(u, side).unwrap();
            assert!((p - q).norm() < 2e-14);
            assert!((d - e).norm() < 2e-14);
        }
    }
    let spans = refined.bezier_spans().unwrap();
    assert_eq!(spans.len(), 3);
    for span in &spans {
        assert_eq!(span.control_points.len(), 3);
        assert!(span.weights.iter().all(|w| *w > 0.));
        for i in 0..=100 {
            let u = span.parameter_range[0]
                + (span.parameter_range[1] - span.parameter_range[0]) * i as f64 / 100.;
            assert!((span.evaluate(u).unwrap() - original.evaluate(u).unwrap()).norm() < 2e-14);
        }
    }
    for pair in spans.windows(2) {
        assert_eq!(pair[0].parameter_range[1], pair[1].parameter_range[0]);
        assert!(
            (pair[0].evaluate(pair[0].parameter_range[1]).unwrap()
                - pair[1].evaluate(pair[1].parameter_range[0]).unwrap())
            .norm()
                < 1e-14
        );
    }
}
#[test]
fn adaptive_chords_independently_bound_circle() {
    let curve = circle().insert_knot(4., 2).unwrap();
    let error = 1e-4;
    let mesh = curve.tessellate_bounded(error, 1000).unwrap();
    assert_eq!(mesh.points.len(), mesh.parameters.len());
    assert_eq!(mesh.error_bounds.len() + 1, mesh.points.len());
    assert_eq!(mesh.parameters.first(), Some(&2.));
    assert_eq!(mesh.parameters.last(), Some(&6.));
    for i in 0..mesh.error_bounds.len() {
        assert!(mesh.error_bounds[i] <= error);
        let a = mesh.points[i];
        let b = mesh.points[i + 1];
        let d = b - a;
        let l = d.dot(d);
        for j in 0..=64 {
            let u =
                mesh.parameters[i] + (mesh.parameters[i + 1] - mesh.parameters[i]) * j as f64 / 64.;
            let p = curve.evaluate(u).unwrap();
            let t = (p - a).dot(d) / l;
            let gap = (p - a - d * t.clamp(0., 1.)).norm();
            assert!(gap <= mesh.error_bounds[i] + 1e-14);
            assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-14);
        }
    }
    assert!(curve.tessellate_bounded(error, 1).is_err());
    assert!(curve.tessellate_bounded(1e-16, 1000).is_err());
}
#[test]
fn c0_and_degenerate_chords() {
    let curve = NurbsCurve::new(
        1,
        vec![0., 0., 1., 2., 2.],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1., 2., 0.),
            Point3::new(0., 0., 0.),
        ],
        vec![1.; 3],
    )
    .unwrap();
    let mesh = curve.tessellate_bounded(1e-8, 2).unwrap();
    assert_eq!(mesh.parameters, vec![0., 1., 2.]);
    assert_eq!(mesh.points, curve.control_points());
    let loop_curve = NurbsCurve::new(
        2,
        vec![0., 0., 0., 1., 1., 1.],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1., 2., 0.),
            Point3::new(0., 0., 0.),
        ],
        vec![1.; 3],
    )
    .unwrap();
    assert!(
        loop_curve
            .tessellate_bounded(1e-3, 1000)
            .unwrap()
            .points
            .len()
            > 2
    );
}
#[test]
fn invalid_requests_and_large_placement() {
    let curve = circle();
    for u in [f64::NAN, f64::INFINITY, 1., 7.] {
        assert!(curve.insert_knot(u, 0).is_err());
    }
    assert!(curve.insert_knot(2., 1).is_err());
    assert!(curve.insert_knot(6., 1).is_err());
    assert!(curve.insert_knot(4., 3).is_err());
    assert!(curve
        .insert_knot(4., 2)
        .unwrap()
        .insert_knot(4., 1)
        .is_err());
    assert_eq!(
        curve.insert_knot(2., 0).unwrap().control_points(),
        curve.control_points()
    );
    for e in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(curve.tessellate_bounded(e, 100).is_err());
    }
    assert!(curve.tessellate_bounded(0.1, 0).is_err());
    assert!(curve.tessellate_bounded(0.1, 1_000_001).is_err());
    let shifted = NurbsCurve::new(
        2,
        curve.knots().to_vec(),
        curve
            .control_points()
            .iter()
            .map(|p| *p + Point3::new(1e15, 0., 0.))
            .collect(),
        curve.weights().to_vec(),
    )
    .unwrap();
    assert!(shifted.tessellate_bounded(0.01, 1000).is_err());
}
#[test]
fn malformed_public_spans_are_errors() {
    let singleton = hagane::RationalBezierSpan {
        parameter_range: [f64::NAN, 6.],
        control_points: vec![Point3::new(0., 0., 0.)],
        weights: vec![1.],
    };
    assert!(singleton.evaluate(3.).is_err());
    use hagane::RationalBezierSpan;
    let mut span = circle().bezier_spans().unwrap().remove(0);
    span.control_points.clear();
    assert!(span.evaluate(3.).is_err());
    let span = RationalBezierSpan {
        parameter_range: [f64::NAN, 6.],
        control_points: vec![Point3::new(0., 0., 0.); 3],
        weights: vec![1.; 3],
    };
    assert!(span.evaluate(3.).is_err());
    let mut span = circle().bezier_spans().unwrap().remove(0);
    span.weights.pop();
    assert!(span.evaluate(3.).is_err());
    let mut span = circle().bezier_spans().unwrap().remove(0);
    span.weights[1] = 0.;
    assert!(span.evaluate(3.).is_err());
}
#[test]
fn small_dimensions_weight_scaling_and_high_degree() {
    let c = circle();
    let tiny = NurbsCurve::new(
        2,
        c.knots().to_vec(),
        c.control_points().iter().map(|p| *p * 1e-100).collect(),
        c.weights().iter().map(|w| w * 1e200).collect(),
    )
    .unwrap();
    let mesh = tiny.tessellate_bounded(1e-104, 1000).unwrap();
    assert!(mesh.error_bounds.iter().all(|b| *b <= 1e-104));
    let p = 16;
    let mut knots = vec![0.; p + 1];
    knots.extend(vec![1.; p + 1]);
    let controls = (0..=p)
        .map(|i| Point3::new(i as f64, (i * i) as f64, 0.))
        .collect();
    let curve = NurbsCurve::new(p, knots, controls, vec![1.; p + 1]).unwrap();
    let refined = curve.insert_knot(0.375, p).unwrap();
    for i in 0..=100 {
        let u = i as f64 / 100.;
        let (a, d) = curve.evaluate_with_derivative(u, KnotSide::Right).unwrap();
        let (b, e) = refined
            .evaluate_with_derivative(u, KnotSide::Right)
            .unwrap();
        assert!((a - b).norm() < 1e-11);
        assert!((d - e).norm() < 1e-10);
    }
    let a: f64 = 1.;
    let b = f64::from_bits(a.to_bits() + 1);
    let narrow = NurbsCurve::new(
        1,
        vec![a, a, b, b],
        vec![Point3::new(0., 0., 0.), Point3::new(1., 0., 0.)],
        vec![1.; 2],
    )
    .unwrap();
    assert_eq!(
        narrow.tessellate_bounded(1e-8, 1).unwrap().parameters,
        vec![a, b]
    );
}
#[test]
fn subnormal_chord_does_not_hide_overflow() {
    let curve = NurbsCurve::new(
        2,
        vec![0., 0., 0., 1., 1., 1.],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1e-310, 2e-310, 0.),
            Point3::new(2e-310, 0., 0.),
        ],
        vec![1.; 3],
    )
    .unwrap();
    let mesh = curve.tessellate_bounded(1e-312, 1000).unwrap();
    assert!(mesh.points.len() > 2);
    for i in 0..mesh.error_bounds.len() {
        assert!(mesh.error_bounds[i] <= 1e-312);
        let a = mesh.points[i];
        let b = mesh.points[i + 1];
        let direction = (b - a).normalized().unwrap();
        let length = (b - a).norm();
        for j in 0..=16 {
            let u =
                mesh.parameters[i] + (mesh.parameters[i + 1] - mesh.parameters[i]) * j as f64 / 16.;
            let q = curve.evaluate(u).unwrap();
            let along = (q - a).dot(direction).clamp(0., length);
            assert!((q - a - direction * along).norm() <= mesh.error_bounds[i] + 1e-320);
        }
    }
}
#[test]
fn rounded_parameter_midpoint_matches_geometry() {
    let a = 1e16;
    let b = a + 6.;
    let curve = NurbsCurve::new(
        2,
        vec![a, a, a, b, b, b],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1., 1., 0.),
            Point3::new(2., 0., 0.),
        ],
        vec![1.; 3],
    )
    .unwrap();
    let mesh = curve.tessellate_bounded(0.3, 100).unwrap();
    for (p, u) in mesh.points.iter().zip(&mesh.parameters) {
        assert!((*p - curve.evaluate(*u).unwrap()).norm() < 1e-14);
    }
    for i in 0..mesh.error_bounds.len() {
        let start = mesh.points[i];
        let end = mesh.points[i + 1];
        let d = (end - start).normalized().unwrap();
        let length = (end - start).norm();
        for u in [a, a + 2., a + 4., b] {
            if u >= mesh.parameters[i] && u <= mesh.parameters[i + 1] {
                let p = curve.evaluate(u).unwrap();
                let distance = (p - start - d * (p - start).dot(d).clamp(0., length)).norm();
                assert!(distance <= mesh.error_bounds[i] + 1e-14);
            }
        }
    }
}
#[test]
fn extraction_preflights_large_refinement_resources() {
    let degree = 16;
    let count = 5000;
    let mut knots = vec![0.; degree + 1];
    knots.extend((1..count - degree).map(|i| i as f64));
    let end = (count - degree) as f64;
    knots.extend(vec![end; degree + 1]);
    let curve = NurbsCurve::new(
        degree,
        knots,
        (0..count).map(|i| Point3::new(i as f64, 0., 0.)).collect(),
        vec![1.; count],
    )
    .unwrap();
    assert!(matches!(
        curve.bezier_spans(),
        Err(hagane::Error::Unsupported(
            "Bezier extraction exceeds control limit"
        ))
    ));
    assert!(matches!(
        curve.tessellate_bounded(0.1, 1),
        Err(hagane::Error::Tessellation(
            "NURBS span count exceeds segment limit"
        ))
    ));
}
