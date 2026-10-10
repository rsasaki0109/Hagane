use hagane::*;
use serde_json::Value;

fn fixture(radii: [f64; 2], height: f64) -> (NurbsFrustumSolid, String) {
    let policy = GeometryTolerance::default();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, height, policy).unwrap();
    let step = body.export_step_mm(policy).unwrap();
    (body, step)
}

#[test]
fn imported_actual_geometry_drives_metrics_display_step_and_mesh_free_queries() {
    let policy = GeometryTolerance::default();
    for (radii, height) in [([16., 8.], 24.), ([8., 16.], 24.), ([8., 8.], 24.)] {
        let (source, step) = fixture(radii, height);
        let imported = import_step_nurbs_frustum_mm(&step, policy).unwrap();
        imported.validate(policy).unwrap();
        assert_eq!(
            format!("{:?}", imported.solid()),
            format!("{:?}", source.solid())
        );
        let actual: Value =
            serde_json::from_str(&nurbs_frustum_step_import_demo_json(&step, 0.5).unwrap())
                .unwrap();
        let expected: Value = serde_json::from_str(
            &nurbs_frustum_demo_json(&[
                radii[0],
                radii[1],
                height,
                0.,
                0.,
                0.,
                0.,
                policy.linear(),
                0.5,
            ])
            .unwrap(),
        )
        .unwrap();
        for field in [
            "radii", "height", "volume", "centroid", "inertia", "bounds", "mesh", "brep", "step",
        ] {
            assert_eq!(actual[field], expected[field], "{field}");
        }
        assert_eq!(actual["import"]["kind"], "nurbs_frustum");
        assert_eq!(actual["import"]["source_geometry_preserved"], true);
        assert_eq!(actual["import"]["pcurve_representation_verified"], true);
        assert_eq!(actual["import"]["validated_geometry"], true);
        assert_eq!(
            actual["placement"]["translation"],
            serde_json::json!([0., 0., 0.])
        );
        assert_eq!(
            imported
                .classify_point(Point3::new(0., 0., height / 2.), policy)
                .unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            imported
                .classify_point(Point3::new(0., 0., height), policy)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            imported
                .classify_point(
                    Point3::new(2. * radii[0].max(radii[1]), 0., height / 2.),
                    policy
                )
                .unwrap(),
            PointLocation::Outside
        );
        let mass = std::f64::consts::PI
            * height
            * (radii[0].powi(2) + radii[0] * radii[1] + radii[1].powi(2))
            / 3.;
        assert!((actual["volume"].as_f64().unwrap() - mass).abs() <= 1e-11 * mass);
    }
}

#[test]
fn display_and_import_failures_are_explicit_and_do_not_poison_subsequent_calls() {
    let (_, step) = fixture([16., 8.], 24.);
    let original = nurbs_frustum_step_import_demo_json(&step, 0.5).unwrap();
    for chord in [0., -1., f64::NAN, f64::INFINITY, 1e-12] {
        assert!(nurbs_frustum_step_import_demo_json(&step, chord).is_err());
    }
    for text in [
        String::new(),
        "not STEP".into(),
        step[..step.len() / 2].into(),
        format!("{step}garbage"),
        "x".repeat(2 * 1024 * 1024 + 1),
    ] {
        assert!(nurbs_frustum_step_import_demo_json(&text, 0.5).is_err());
    }
    let policy = GeometryTolerance::default();
    let placed = NurbsFrustumSolid::new(
        Transform::translation(Vec3::new(1., 0., 0.)).unwrap(),
        [16., 8.],
        24.,
        policy,
    )
    .unwrap();
    assert!(matches!(
        nurbs_frustum_step_import_demo_json(&placed.export_step_mm(policy).unwrap(), 0.5),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        nurbs_frustum_step_import_demo_json(&step, 0.5).unwrap(),
        original
    );
    assert!(matches!(
        import_step_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_bounded_analytic_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
}
