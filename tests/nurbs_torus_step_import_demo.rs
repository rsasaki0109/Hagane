use hagane::*;
use serde_json::Value;
#[test]
fn imported_actual_cardinal_body_is_transactional_and_invalidates_children_only_on_success() {
    let policy = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let frame = Frame3::new_with_tolerance(
        Point3::new(12., -5., 8.),
        [
            Vec3::new(0., 1., 0.),
            Vec3::new(0., 0., 1.),
            Vec3::new(1., 0., 0.),
        ],
        policy,
    )
    .unwrap();
    let source = NurbsTorusSolid::new(frame, [16., 4.], policy).unwrap();
    let text = source.export_step_mm(policy).unwrap();
    let mut s = NurbsTorusDemoSession::new();
    s.apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1]).unwrap();
    s.split_horizontal(1., 0.1).unwrap();
    let old = s.accepted_body().unwrap() as *const _;
    let pair = s.accepted_horizontal_split().unwrap() as *const _;
    let query = s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .unwrap();
    for (bad, chord) in [
        ("invalid STEP", 0.1),
        (text.as_str(), 1e-14),
        (text.as_str(), f64::NAN),
    ] {
        assert!(s.import_step_cardinal(bad, chord, policy).is_err());
        assert_eq!(s.accepted_body().unwrap() as *const _, old);
        assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, pair);
        assert_eq!(
            s.query_horizontal_point(true, Point3::new(16., 0., 0.))
                .unwrap(),
            query
        );
    }
    let units = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    assert_ne!(units, text);
    assert!(s.import_step_cardinal(&units, 0.1, policy).is_err());
    assert_eq!(s.accepted_body().unwrap() as *const _, old);
    let result: Value =
        serde_json::from_str(&s.import_step_cardinal(&text, 0.1, policy).unwrap()).unwrap();
    assert!(s.accepted_horizontal_split().is_none());
    assert_eq!(result["radii"], serde_json::json!([16., 4.]));
    assert_eq!(
        result["placement"]["translation"],
        serde_json::json!([12., -5., 8.])
    );
    assert_eq!(
        result["placement"]["axes"],
        serde_json::json!([[0., 1., 0.], [0., 0., 1.], [1., 0., 0.]])
    );
    assert!(
        (result["volume"].as_f64().unwrap() - 512. * std::f64::consts::PI.powi(2)).abs() < 1e-8
    );
    let imported = s.accepted_body().unwrap();
    for (a, b) in imported
        .solid()
        .shell
        .faces
        .iter()
        .zip(&source.solid().shell.faces)
    {
        match (&a.surface, &b.surface) {
            (Surface::Nurbs(a), Surface::Nurbs(b)) => {
                assert_eq!(a.control_points(), b.control_points());
                assert_eq!(a.weights(), b.weights());
                for axis in 0..2 {
                    assert_eq!(a.knots(axis).unwrap(), b.knots(axis).unwrap());
                }
            }
            _ => panic!("actual torus surfaces must remain rational"),
        }
    }
    assert_eq!(
        serde_json::from_str::<Value>(
            &s.query_point(frame.point(Point3::new(16., 0., 0.)))
                .unwrap()
        )
        .unwrap()["location"],
        "inside"
    );
    assert!(s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .is_err());
    let reparsed =
        import_step_nurbs_torus_cardinal_mm(result["step"].as_str().unwrap(), policy).unwrap();
    assert_eq!(reparsed.radii(), imported.radii());
    assert_eq!(
        reparsed.export_step_mm(policy).unwrap(),
        result["step"].as_str().unwrap()
    );
}
