use hagane::*;
fn bezier(points: Vec<Point3>, weights: Vec<f64>, domain: [f64; 2]) -> NurbsCurve {
    let degree = points.len() - 1;
    NurbsCurve::new(
        degree,
        [vec![domain[0]; degree + 1], vec![domain[1]; degree + 1]].concat(),
        points,
        weights,
    )
    .unwrap()
}
fn quadratic(points: &[Point3], weights: &[f64], q: f64) -> Point3 {
    let b = [(1. - q).powi(2), 2. * q * (1. - q), q * q];
    let d = (0..3).map(|i| b[i] * weights[i]).sum::<f64>();
    (0..3).fold(Point3::new(0., 0., 0.), |s, i| {
        s + points[i] * (b[i] * weights[i])
    }) * (1. / d)
}
#[test]
fn rational_quarter_circle_on_polynomial_roof_has_degree_eight_and_fourth_power_denominator() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([20., 12., 3.], 4., tol).unwrap();
    let Surface::Nurbs(surface) = &graph.brep().shell.faces[1].surface else {
        panic!()
    };
    let w = 0.5f64.sqrt();
    let uv = bezier(
        vec![
            Point3::new(0.6, 0.5, 0.),
            Point3::new(0.6, 0.5 + 1. / 6., 0.),
            Point3::new(0.5, 0.5 + 1. / 6., 0.),
        ],
        vec![1., w, 1.],
        [7., 11.],
    );
    let curve = surface.parameter_curve_nurbs(&uv, tol).unwrap();
    assert_eq!(curve.degree(), 8);
    assert_eq!(curve.knots(), [vec![7.; 9], vec![11.; 9]].concat());
    assert!(curve.weights().iter().any(|v| *v != curve.weights()[0]));
    for i in 0..=256 {
        let q = i as f64 / 256.;
        let a = (1. - q).powi(2);
        let b = 2. * w * q * (1. - q);
        let c = q * q;
        let d = a + b + c;
        let u = 0.5 + 0.1 * (a + b) / d;
        let v = 0.5 + (b + c) / (6. * d);
        let point = curve.evaluate(7. + 4. * q).unwrap();
        let expected = Point3::new(20. * u, 12. * v, 3. + 16. * u * (1. - u) * v * (1. - v));
        assert!((point - expected).norm() < tol.linear / 10.);
        assert!(((point.x - 10.).powi(2) + (point.y - 6.).powi(2) - 4.).abs() < 1e-11);
        let denominator = curve
            .weights()
            .iter()
            .enumerate()
            .map(|(k, w)| {
                let binomial = [1., 8., 28., 56., 70., 56., 28., 8., 1.][k];
                w * binomial * q.powi(k as i32) * (1. - q).powi(8 - k as i32)
            })
            .sum::<f64>()
            / curve.weights()[0];
        assert!((denominator - d.powi(4)).abs() < 2e-13);
    }
}
#[test]
fn nonunit_rational_surface_and_uv_weights_match_independent_tensor_formula() {
    let controls = vec![
        Point3::new(1., 2., 0.),
        Point3::new(1., 8., 1.),
        Point3::new(9., 2., 3.),
        Point3::new(9., 8., 6.),
    ];
    let weights = vec![2., 3., 5., 7.];
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![2., 2., 6., 6.], vec![-3., -3., 5., 5.]],
        [2, 2],
        controls.clone(),
        weights.clone(),
    )
    .unwrap();
    assert_eq!(surface.weights(), weights);
    let points = vec![
        Point3::new(2.5, -2., 0.),
        Point3::new(4., -1., 0.),
        Point3::new(5.5, 4., 0.),
    ];
    let uv_weights = vec![3., 0.7, 2.];
    let uv = bezier(points.clone(), uv_weights.clone(), [-7., -3.]);
    let curve = surface
        .parameter_curve_nurbs(&uv, Tolerance::default())
        .unwrap();
    assert_eq!(curve.degree(), 4);
    assert_eq!(curve.domain(), [-7., -3.]);
    for i in 0..=256 {
        let q = i as f64 / 256.;
        let p = quadratic(&points, &uv_weights, q);
        let u = (p.x - 2.) / 4.;
        let v = (p.y + 3.) / 8.;
        let basis = [(1. - u) * (1. - v), (1. - u) * v, u * (1. - v), u * v];
        let d = (0..4).map(|i| basis[i] * weights[i]).sum::<f64>();
        let expected = (0..4).fold(Point3::new(0., 0., 0.), |s, i| {
            s + controls[i] * (basis[i] * weights[i])
        }) * (1. / d);
        assert!((curve.evaluate(-7. + 4. * q).unwrap() - expected).norm() < 1e-10);
    }
}
#[test]
fn composition_scaling_and_rigid_coordinates_preserve_the_actual_path() {
    for scale in [1e-6, 1., 1e6] {
        let tol = Tolerance::new(scale * 1e-7).unwrap();
        let frame = Transform::translation(Vec3::new(4. * scale, -3. * scale, scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
            .unwrap();
        let controls = vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., scale, 0.),
            Point3::new(scale, 0., 0.),
            Point3::new(scale, scale, scale),
        ]
        .into_iter()
        .map(|p| frame.point(p))
        .collect();
        let surface = NurbsSurface::new(
            [1, 1],
            [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
            [2, 2],
            controls,
            vec![1.; 4],
        )
        .unwrap();
        let points = vec![
            Point3::new(0.125, 0.25, 0.),
            Point3::new(0.5, 0.75, 0.),
            Point3::new(0.875, 0.5, 0.),
        ];
        let weights = vec![1., 0.8, 1.];
        let uv = bezier(points.clone(), weights.clone(), [0., 1.]);
        let curve = surface.parameter_curve_nurbs(&uv, tol).unwrap();
        for i in 0..=128 {
            let q = i as f64 / 128.;
            let p = quadratic(&points, &weights, q);
            let expected = frame.point(Point3::new(scale * p.x, scale * p.y, scale * p.x * p.y));
            assert!((curve.evaluate(q).unwrap() - expected).norm() < tol.linear / 10.);
        }
    }
}

#[test]
fn checked_pcurves_and_unsupported_compositions_never_return_false_success() {
    let tol = Tolerance::default();
    let controls = vec![
        Point3::new(0., 0., 0.),
        Point3::new(0., 1., 0.),
        Point3::new(1., 0., 0.),
        Point3::new(1., 1., 1.),
    ];
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        controls.clone(),
        vec![1.; 4],
    )
    .unwrap();
    let points = vec![
        Point3::new(0.125, 0.25, 0.),
        Point3::new(0.5, 0.75, 0.),
        Point3::new(0.875, 0.5, 0.),
    ];
    let uv = bezier(points.clone(), vec![1.; 3], [2., 6.]);
    let pcurve = PCurve::nurbs(uv.clone()).unwrap();
    for t in [2., 3., 4., 5., 6.] {
        let p = uv.evaluate(t).unwrap();
        assert_eq!(pcurve.try_evaluate(t).unwrap(), [p.x, p.y]);
    }
    assert!(pcurve.try_evaluate(f64::NAN).is_err());
    assert!(surface
        .parameter_curve_nurbs(&uv, Tolerance { linear: f64::NAN })
        .is_err());
    let mut nonplanar = points.clone();
    nonplanar[1].z = 1e-20;
    let nonplanar = bezier(nonplanar, vec![1.; 3], [0., 1.]);
    assert!(PCurve::nurbs(nonplanar.clone()).is_err());
    assert!(surface.parameter_curve_nurbs(&nonplanar, tol).is_err());
    let mut outside = points.clone();
    outside[1].x = 1.01;
    assert!(surface
        .parameter_curve_nurbs(&bezier(outside, vec![1.; 3], [0., 1.]), tol)
        .is_err());
    assert!(surface
        .parameter_curve_nurbs(&bezier(points.clone(), vec![1e-200, 1., 1.], [0., 1.]), tol)
        .is_err());
    let rational = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        controls,
        vec![1e-200, 1., 1., 1.],
    )
    .unwrap();
    assert!(rational.parameter_curve_nurbs(&uv, tol).is_err());
    let multi = NurbsCurve::new(1, vec![0., 0., 0.5, 1., 1.], points.clone(), vec![1.; 3]).unwrap();
    assert!(surface.parameter_curve_nurbs(&multi, tol).is_err());
    let source = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        (0..3)
            .flat_map(|i| (0..2).map(move |j| Point3::new(i as f64 / 2., j as f64, 0.)))
            .collect(),
        vec![1.; 6],
    )
    .unwrap();
    assert!(source.parameter_curve_nurbs(&uv, tol).is_err());
    let high = NurbsSurface::new(
        [5, 4],
        [
            [vec![0.; 6], vec![1.; 6]].concat(),
            [vec![0.; 5], vec![1.; 5]].concat(),
        ],
        [6, 5],
        (0..6)
            .flat_map(|i| (0..5).map(move |j| Point3::new(i as f64 / 5., j as f64 / 4., 0.)))
            .collect(),
        vec![1.; 30],
    )
    .unwrap();
    assert!(high.parameter_curve_nurbs(&uv, tol).is_err());
}
