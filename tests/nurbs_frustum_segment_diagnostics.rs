use hagane::*;
#[test]
fn distinct_user_scenarios_have_actionable_unsupported_reasons_and_recover() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let scenarios = [
        (
            [0., 0., 0.],
            [0., 0., 20.],
            "endpoint is within the boundary band",
        ),
        (
            [-20., 12., 12.],
            [20., 12., 12.],
            "tangency or double-root separation",
        ),
        ([20., 20., 0.], [21., 20., 0.], "cap-plane band"),
        (
            [-1e12, 0., 12.],
            [1e12, 0., 12.],
            "world-coordinate precision",
        ),
        ([0., 0., 12.], [1e-8, 0., 12.], "too short"),
    ];
    let point = |a: [f64; 3]| Point3::new(a[0], a[1], a[2]);
    for (a, b, phrase) in scenarios {
        let Error::Unsupported(reason) = body.intersect_segment(point(a), point(b), p).unwrap_err()
        else {
            panic!("unsupported category changed");
        };
        assert!(reason.contains(phrase), "{reason}");
        let recovered = body
            .intersect_segment(Point3::new(-20., 0., 12.), Point3::new(20., 0., 12.), p)
            .unwrap();
        assert_eq!(recovered.hits.len(), 2);
        assert!(recovered.material_interval.is_some());
    }
    let taper = NurbsFrustumSolid::new(Frame3::IDENTITY, [2., 4.], 4., p).unwrap();
    let Error::Unsupported(reason) = taper
        .intersect_segment(Point3::new(-4., 0., -1.), Point3::new(1e-12, 0., 7.), p)
        .unwrap_err()
    else {
        panic!("unsupported category changed");
    };
    assert!(reason.contains("near-linear lateral direction"), "{reason}");
    assert!(matches!(
        body.intersect_segment(Point3::new(0., 0., 1.), Point3::new(0., 0., 1.), p),
        Err(Error::InvalidInput(_))
    ));
}
