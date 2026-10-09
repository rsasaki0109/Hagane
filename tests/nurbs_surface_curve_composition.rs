use hagane::*;
fn quarter() -> NurbsCurve {
    NurbsCurve::new(
        2,
        vec![2., 2., 2., 5., 5., 5.],
        vec![
            Point3::new(0.8, 0.5, 0.),
            Point3::new(0.8, 0.9, 0.),
            Point3::new(0.5, 0.9, 0.),
        ],
        vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
    )
    .unwrap()
}
#[test]
fn quarter_ellipse_lifts_exactly_to_degree_eight_roof() {
    let tol = Tolerance::default();
    let body = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let Surface::Nurbs(s) = &body.brep().shell.faces[1].surface else {
        panic!()
    };
    let uv = quarter();
    let c = s.parameter_curve_nurbs(&uv, tol).unwrap();
    assert_eq!(c.degree(), 8);
    assert_eq!(c.domain(), [2., 5.]);
    let coedge = Coedge {
        edge: 0,
        forward: true,
        pcurve: PCurve::nurbs(uv.clone()).unwrap(),
    };
    assert!(c.weights().iter().any(|w| *w != 1.));
    let scaled = NurbsCurve::new(
        uv.degree(),
        uv.knots().to_vec(),
        uv.control_points().to_vec(),
        uv.weights().iter().map(|w| 2. * w).collect(),
    )
    .unwrap();
    let scaled = s.parameter_curve_nurbs(&scaled, tol).unwrap();
    assert_eq!(
        scaled.weights(),
        c.weights().iter().map(|w| 16. * w).collect::<Vec<_>>()
    );

    for i in 0..=500 {
        let t = 2. + 3. * i as f64 / 500.;
        let p = uv.evaluate(t).unwrap();
        assert_eq!(coedge.pcurve.try_evaluate(t).unwrap(), [p.x, p.y]);
        let expected = Point3::new(
            80. * p.x,
            60. * p.y,
            20. + 120. * p.x * (1. - p.x) * p.y * (1. - p.y),
        );
        let actual = c.evaluate(t).unwrap();
        assert!((actual - expected).norm() < 1e-10);
    }
}
#[test]
fn positive_rational_source_weights_and_nonunit_domains() {
    let k = vec![-2., -2., 3., 3.];
    let s = NurbsSurface::new(
        [1, 1],
        [k.clone(), k],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 2., 1.),
            Point3::new(3., 0., 2.),
            Point3::new(3., 2., 4.),
        ],
        vec![2., 3., 4., 5.],
    )
    .unwrap();
    let uv = NurbsCurve::new(
        2,
        vec![10., 10., 10., 20., 20., 20.],
        vec![
            Point3::new(-1., -1., 0.),
            Point3::new(2., 0., 0.),
            Point3::new(2., 2., 0.),
        ],
        vec![2., 0.8, 3.],
    )
    .unwrap();
    let c = s.parameter_curve_nurbs(&uv, Tolerance::default()).unwrap();
    for i in 0..=200 {
        let t = 10. + i as f64 / 20.;
        let p = uv.evaluate(t).unwrap();
        assert!((c.evaluate(t).unwrap() - s.evaluate(p.x, p.y).unwrap()).norm() < 1e-11);
    }
}
#[test]
fn rejects_wrong_scope_precision_and_raw_weight_overflow() {
    let t = Tolerance::default();
    let body = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    let Surface::Nurbs(s) = &body.brep().shell.faces[1].surface else {
        panic!()
    };
    let bad = NurbsCurve::new(
        1,
        vec![0., 0., 1., 1.],
        vec![Point3::new(0., 0., 1.), Point3::new(1., 1., 0.)],
        vec![1.; 2],
    )
    .unwrap();
    assert!(s.parameter_curve_nurbs(&bad, t).is_err());
    let refined = s.insert_knot(0, 0.5, 1).unwrap();
    assert!(refined.parameter_curve_nurbs(&quarter(), t).is_err());
    assert!(s
        .parameter_curve_nurbs(&quarter(), Tolerance::new(1e-20).unwrap())
        .is_err());
    let q = quarter();
    let huge = NurbsCurve::new(
        q.degree(),
        q.knots().to_vec(),
        q.control_points().to_vec(),
        vec![1e100; 3],
    )
    .unwrap();
    assert!(s.parameter_curve_nurbs(&huge, t).is_err());
}
#[test]
fn tiny_resolved_patch_and_rigidly_placed_source() {
    let t = Tolerance::new(1e-19).unwrap();
    let k = vec![0., 0., 1., 1.];
    let s = NurbsSurface::new(
        [1, 1],
        [k.clone(), k],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1e-12, 0.),
            Point3::new(1e-12, 0., 0.),
            Point3::new(1e-12, 1e-12, 1e-12),
        ],
        vec![1.; 4],
    )
    .unwrap();
    let uv = quarter();
    let c = s.parameter_curve_nurbs(&uv, t).unwrap();
    let p = uv.evaluate(3.5).unwrap();
    assert!((c.evaluate(3.5).unwrap() - s.evaluate(p.x, p.y).unwrap()).norm() < 1e-25);
    let placed = NurbsGraphSolid::new([80., 60., 20.], 30., Tolerance::default())
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(2., 3., 4.)).unwrap(),
            Tolerance::default(),
        )
        .unwrap();
    let Surface::Nurbs(s) = &placed.brep().shell.faces[1].surface else {
        panic!()
    };
    let c = s.parameter_curve_nurbs(&uv, Tolerance::default()).unwrap();
    assert!((c.evaluate(3.5).unwrap() - s.evaluate(p.x, p.y).unwrap()).norm() < 1e-10);
}
