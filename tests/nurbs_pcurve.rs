use hagane::*;
fn curve(z: f64) -> NurbsCurve {
    NurbsCurve::new(
        2,
        vec![2., 2., 2., 5., 5., 5.],
        vec![
            Point3::new(1., 0., z),
            Point3::new(1., 1., z),
            Point3::new(0., 1., z),
        ],
        vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
    )
    .unwrap()
}
#[test]
fn checked_rational_uv_preserves_basis_and_parameter() {
    let c = curve(0.);
    let uv = PCurve::nurbs(c.clone()).unwrap();
    let PCurve::Nurbs(retained) = &uv else {
        panic!()
    };
    assert_eq!(retained.degree(), 2);
    assert_eq!(retained.knots(), c.knots());
    assert_eq!(retained.weights(), c.weights());
    for i in 0..=100 {
        let t = 2. + 3. * i as f64 / 100.;
        let p = c.evaluate(t).unwrap();
        assert_eq!(uv.try_evaluate(t).unwrap(), [p.x, p.y]);
        assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-14);
    }
    for t in [1.999, 5.001, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(uv.try_evaluate(t).is_err());
        assert!(uv.evaluate(t).iter().all(|x| x.is_nan()));
    }
}
#[test]
fn non_uv_controls_reject_checked_and_public_variant() {
    for z in [1., f64::MIN_POSITIVE] {
        assert!(PCurve::nurbs(curve(z)).is_err());
        let unchecked = PCurve::Nurbs(Box::new(curve(z)));
        assert!(unchecked.try_evaluate(3.).is_err());
        assert!(unchecked.evaluate(3.).iter().all(|x| x.is_nan()));
    }
    for weights in [vec![1., 0., 1.], vec![1., -1., 1.], vec![1., f64::NAN, 1.]] {
        assert!(NurbsCurve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            vec![Point3::new(0., 0., 0.); 3],
            weights
        )
        .is_err());
    }
}
#[test]
fn analytic_arithmetic_and_checked_nonfinite_behavior() {
    let p = PCurve::Affine {
        origin: [2., 3.],
        direction: [4., 5.],
    };
    assert_eq!(p.evaluate(0.25), [3., 4.25]);
    assert_eq!(p.try_evaluate(0.25).unwrap(), p.evaluate(0.25));
    assert!(p.try_evaluate(f64::NAN).is_err());
}
#[test]
fn analytic_solid_operations_reject_even_equivalent_rational_trim() {
    let t = Tolerance::default();
    let mut s = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(2., 3., 4.),
        },
        t,
    )
    .unwrap();
    let c = &mut s.shell.faces[0].wires[0].coedges[0];
    let PCurve::Affine { origin, direction } = c.pcurve else {
        panic!()
    };
    c.pcurve = PCurve::nurbs(
        NurbsCurve::new(
            1,
            vec![0., 0., 1., 1.],
            vec![
                Point3::new(origin[0], origin[1], 0.),
                Point3::new(origin[0] + direction[0], origin[1] + direction[1], 0.),
            ],
            vec![1., 1.],
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(s.validate(t), Err(Error::Unsupported(_))));
    assert!(matches!(s.volume(), Err(Error::Unsupported(_))));
    assert!(matches!(export_step_mm(&s, t), Err(Error::Unsupported(_))));
    assert!(matches!(
        classify_point_in_solid(&s, Point3::new(1., 1., 1.), GeometryTolerance::default()),
        Err(Error::Unsupported(_))
    ));
}
