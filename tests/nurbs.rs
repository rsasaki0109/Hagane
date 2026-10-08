use hagane::*;
fn p(x: f64, y: f64) -> Point3 {
    Point3::new(x, y, 0.0)
}
fn near(a: Vec3, b: Vec3) {
    assert!((a - b).norm() < 1e-9, "{a:?} != {b:?}");
}
fn quarter() -> NurbsCurve {
    NurbsCurve::new(
        2,
        vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
        vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0],
    )
    .unwrap()
}
#[test]
fn rational_quadratic_is_exact_quarter_circle() {
    let c = quarter();
    for i in 0..=100 {
        let u = i as f64 / 100.0;
        let point = c.evaluate(u).unwrap();
        assert!((point.x * point.x + point.y * point.y - 1.0).abs() < 1e-14);
        let d = c.derivative(u).unwrap();
        assert!(point.dot(d).abs() < 1e-13);
        assert!(d.norm() > 1.0);
    }
    near(
        c.evaluate(0.5).unwrap(),
        p(
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ),
    );
}
#[test]
fn line_and_parameter_units() {
    let c = NurbsCurve::new(
        1,
        vec![2.0, 2.0, 6.0, 6.0],
        vec![p(1.0, 2.0), p(9.0, 6.0)],
        vec![1.0, 1.0],
    )
    .unwrap();
    near(c.evaluate(4.0).unwrap(), p(5.0, 4.0));
    near(c.derivative(2.0).unwrap(), p(2.0, 1.0));
    near(c.derivative(6.0).unwrap(), p(2.0, 1.0));
    assert_eq!(c.domain(), [2.0, 6.0]);
}
#[test]
fn cubic_matches_bernstein_polynomial() {
    let points = vec![p(0.0, 0.0), p(1.0, 3.0), p(4.0, -2.0), p(6.0, 1.0)];
    let c = NurbsCurve::new(
        3,
        vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
        points.clone(),
        vec![1.0; 4],
    )
    .unwrap();
    for i in 0..=100 {
        let u = i as f64 / 100.0;
        let v = 1.0 - u;
        let expected = points[0] * v.powi(3)
            + points[1] * (3.0 * v * v * u)
            + points[2] * (3.0 * v * u * u)
            + points[3] * u.powi(3);
        let d = (points[1] - points[0]) * (3.0 * v * v)
            + (points[2] - points[1]) * (6.0 * v * u)
            + (points[3] - points[2]) * (3.0 * u * u);
        near(c.evaluate(u).unwrap(), expected);
        near(c.derivative(u).unwrap(), d);
    }
}
#[test]
fn derivatives_match_finite_differences_on_nonuniform_rational_spline() {
    let c = NurbsCurve::new(
        2,
        vec![2.0, 2.0, 2.0, 3.0, 5.0, 5.0, 5.0],
        vec![p(0.0, 0.0), p(1.0, 3.0), p(4.0, 2.0), p(5.0, 0.0)],
        vec![1.0, 2.0, 0.75, 1.0],
    )
    .unwrap();
    for u in [2.1, 2.5, 2.99, 3.0, 3.01, 3.5, 4.9] {
        let h = 1e-6;
        let fd = (c.evaluate(u + h).unwrap() - c.evaluate(u - h).unwrap()) * (0.5 / h);
        assert!((fd - c.derivative(u).unwrap()).norm() < 1e-5);
    }
}
#[test]
fn c0_knots_require_a_side() {
    let c = NurbsCurve::new(
        1,
        vec![0.0, 0.0, 0.5, 1.0, 1.0],
        vec![p(0.0, 0.0), p(1.0, 1.0), p(3.0, 1.0)],
        vec![1.0; 3],
    )
    .unwrap();
    assert!(matches!(c.derivative(0.5), Err(Error::Unsupported(_))));
    let (left, dl) = c.evaluate_with_derivative(0.5, KnotSide::Left).unwrap();
    let (right, dr) = c.evaluate_with_derivative(0.5, KnotSide::Right).unwrap();
    near(left, right);
    near(left, p(1.0, 1.0));
    near(dl, p(2.0, 2.0));
    near(dr, p(4.0, 0.0));
}
#[test]
fn repeated_quadratic_knot_is_continuous_with_distinct_tangents() {
    let c = NurbsCurve::new(
        2,
        vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0],
        vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 1.0),
            p(2.0, 1.0),
            p(2.0, 2.0),
        ],
        vec![1.0; 5],
    )
    .unwrap();
    let (a, dl) = c.evaluate_with_derivative(0.5, KnotSide::Left).unwrap();
    let (b, dr) = c.evaluate_with_derivative(0.5, KnotSide::Right).unwrap();
    near(a, b);
    near(a, p(1.0, 1.0));
    near(dl, p(0.0, 4.0));
    near(dr, p(4.0, 0.0));
}
#[test]
fn endpoints_use_inward_derivatives() {
    let c = quarter();
    for u in [0.0, 1.0] {
        let a = c.evaluate_with_derivative(u, KnotSide::Left).unwrap();
        let b = c.evaluate_with_derivative(u, KnotSide::Right).unwrap();
        near(a.0, b.0);
        near(a.1, b.1);
    }
    near(c.derivative(0.0).unwrap(), p(0.0, 2.0_f64.sqrt()));
    near(c.derivative(1.0).unwrap(), p(-2.0_f64.sqrt(), 0.0));
}
#[test]
fn common_weight_scaling_leaves_geometry_unchanged() {
    let original = quarter();
    for scale in [1e-200, 1e200] {
        let c = NurbsCurve::new(
            original.degree(),
            original.knots().to_vec(),
            original.control_points().to_vec(),
            original.weights().iter().map(|w| w * scale).collect(),
        )
        .unwrap();
        for u in [0.0, 0.2, 0.5, 0.9, 1.0] {
            near(c.evaluate(u).unwrap(), original.evaluate(u).unwrap());
            near(c.derivative(u).unwrap(), original.derivative(u).unwrap());
        }
    }
}
#[test]
fn invalid_data_and_unsupported_clamping_are_explicit() {
    let build = |degree, knots, points, weights| NurbsCurve::new(degree, knots, points, weights);
    assert!(build(0, vec![0.0, 1.0], vec![p(0.0, 0.0)], vec![1.0]).is_err());
    for weights in [
        vec![],
        vec![1.0, 0.0, 1.0],
        vec![1.0, -1.0, 1.0],
        vec![1.0, f64::NAN, 1.0],
        vec![f64::MAX, f64::MIN_POSITIVE, 1.0],
    ] {
        assert!(build(
            2,
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
            weights
        )
        .is_err());
    }
    for knots in [
        vec![0.0; 6],
        vec![0.0, 0.0, 0.0, 1.0, 0.9, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, f64::INFINITY, 1.0],
        vec![0.0, 0.0, 0.2, 0.8, 1.0, 1.0],
    ] {
        assert!(build(
            2,
            knots,
            vec![p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
            vec![1.0; 3]
        )
        .is_err());
    }
    assert!(build(
        1,
        vec![0.0, 0.0, 0.5, 0.5, 1.0, 1.0],
        vec![p(0.0, 0.0); 4],
        vec![1.0; 4]
    )
    .is_err());
}
#[test]
fn parameter_errors_do_not_extrapolate() {
    let c = quarter();
    for u in [-1e-12, 1.0 + 1e-12, f64::NAN, f64::INFINITY] {
        assert!(c.evaluate(u).is_err());
        assert!(c.derivative(u).is_err());
        assert!(c.evaluate_with_derivative(u, KnotSide::Left).is_err());
    }
}
#[test]
fn high_degree_endpoint_and_constant_curve() {
    let degree = 16;
    let mut knots = vec![0.0; degree + 1];
    knots.extend(vec![1.0; degree + 1]);
    let c = NurbsCurve::new(
        degree,
        knots,
        vec![Point3::new(2.0, -3.0, 4.0); degree + 1],
        vec![1.0; degree + 1],
    )
    .unwrap();
    for u in [0.0, 0.3, 0.5, 1.0] {
        near(c.evaluate(u).unwrap(), Point3::new(2.0, -3.0, 4.0));
        near(c.derivative(u).unwrap(), p(0.0, 0.0));
    }
}

fn basis(i: usize, degree: usize, u: f64, knots: &[f64]) -> f64 {
    if degree == 0 {
        return f64::from(knots[i] <= u && u < knots[i + 1]);
    }
    let a = knots[i + degree] - knots[i];
    let b = knots[i + degree + 1] - knots[i + 1];
    let left = if a == 0.0 {
        0.0
    } else {
        (u - knots[i]) / a * basis(i, degree - 1, u, knots)
    };
    let right = if b == 0.0 {
        0.0
    } else {
        (knots[i + degree + 1] - u) / b * basis(i + 1, degree - 1, u, knots)
    };
    left + right
}
#[test]
fn de_boor_matches_independent_cox_basis_on_nonuniform_3d_curve() {
    let c = NurbsCurve::new(
        3,
        vec![0.0, 0.0, 0.0, 0.0, 0.2, 0.5, 0.8, 1.0, 1.0, 1.0, 1.0],
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.5),
            Point3::new(2.0, -1.0, 1.0),
            Point3::new(3.0, 4.0, 2.0),
            Point3::new(4.0, -2.0, 3.0),
            Point3::new(5.0, 1.0, 3.5),
            Point3::new(6.0, 0.0, 4.0),
        ],
        vec![1.0, 0.5, 2.0, 1.5, 3.0, 0.75, 1.0],
    )
    .unwrap();
    for i in 0..100 {
        let u = i as f64 / 100.0;
        let mut numerator = Vec3::new(0.0, 0.0, 0.0);
        let mut denominator = 0.0;
        let mut unity = 0.0;
        for j in 0..c.control_points().len() {
            let n = basis(j, c.degree(), u, c.knots());
            unity += n;
            let weighted = n * c.weights()[j];
            numerator = numerator + c.control_points()[j] * weighted;
            denominator += weighted;
        }
        assert!((unity - 1.0).abs() < 1e-14);
        near(c.evaluate(u).unwrap(), numerator * (1.0 / denominator));
    }
}
#[test]
fn unusable_floating_point_ranges_are_errors() {
    let c = NurbsCurve::new(
        1,
        vec![0.0, 0.0, 1e-320, 1e-320],
        vec![p(0.0, 0.0), p(1.0, 1.0)],
        vec![1.0; 2],
    )
    .unwrap();
    near(c.evaluate(0.0).unwrap(), p(0.0, 0.0));
    assert!(c.derivative(0.0).is_err());
    assert!(NurbsCurve::new(
        1,
        vec![0.0, 0.0, 1.0, 1.0],
        vec![p(1e-50, 0.0), p(1.0, 1.0)],
        vec![1e-120, 1e180]
    )
    .is_err());
}
