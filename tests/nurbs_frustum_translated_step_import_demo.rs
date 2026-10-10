use hagane::*;
use serde_json::Value;

fn source(radii: [f64; 2], height: f64, translation: [f64; 3]) -> NurbsFrustumSolid {
    NurbsFrustumSolid::new(
        Transform::translation(Vec3::new(translation[0], translation[1], translation[2])).unwrap(),
        radii,
        height,
        GeometryTolerance::default(),
    )
    .unwrap()
}
fn check(body: &NurbsFrustumSolid) {
    let policy = GeometryTolerance::default();
    let step = body.export_step_mm(policy).unwrap();
    let imported = import_step_nurbs_frustum_translated_mm(&step, policy).unwrap();
    assert_eq!(
        format!("{:?}", imported.solid()),
        format!("{:?}", body.solid())
    );
    let actual: Value =
        serde_json::from_str(&nurbs_frustum_translated_step_import_demo_json(&step, 0.1).unwrap())
            .unwrap();
    let origin = body.frame().origin();
    let radii = body.radii();
    let expected: Value = serde_json::from_str(
        &nurbs_frustum_demo_json(&[
            radii[0],
            radii[1],
            body.height(),
            0.,
            origin.x,
            origin.y,
            origin.z,
            policy.linear(),
            0.1,
        ])
        .unwrap(),
    )
    .unwrap();
    for field in [
        "radii",
        "height",
        "placement",
        "volume",
        "centroid",
        "inertia",
        "bounds",
        "mesh",
        "brep",
        "step",
    ] {
        assert_eq!(actual[field], expected[field], "{field}");
    }
    assert_eq!(actual["import"]["kind"], "nurbs_frustum_translated");
    assert_eq!(actual["import"]["source_geometry_preserved"], true);
    assert_eq!(actual["import"]["pcurve_representation_verified"], true);
    assert_eq!(actual["import"]["validated_geometry"], true);
    assert_eq!(
        imported
            .classify_point(
                body.frame().point(Point3::new(0., 0., body.height() / 2.)),
                policy
            )
            .unwrap(),
        PointLocation::Inside
    );
    // The additive translated entry point must not broaden the origin-only reader.
    if origin != Point3::new(0., 0., 0.) {
        assert!(matches!(
            import_step_nurbs_frustum_mm(&step, policy),
            Err(Error::Unsupported(_))
        ));
        assert!(nurbs_frustum_step_import_demo_json(&step, 0.1).is_err());
    }
}

#[test]
fn actual_translated_default_swapped_cylinder_and_upper_child_drive_full_json() {
    for radii in [[16., 8.], [8., 16.], [8., 8.]] {
        let body = source(radii, 24., [12., -5., 8.]);
        check(&body);
        check(
            &body
                .split_axial(10., GeometryTolerance::default())
                .unwrap()
                .upper,
        );
    }
    check(&source([16., 8.], 24., [0., 0., 0.]));
}

#[test]
fn translated_failures_are_explicit_stateless_and_old_domains_remain_strict() {
    let policy = GeometryTolerance::default();
    let body = source([16., 8.], 24., [12., -5., 8.]);
    let step = body.export_step_mm(policy).unwrap();
    let original = nurbs_frustum_translated_step_import_demo_json(&step, 0.1).unwrap();
    for chord in [0., -1., f64::NAN, f64::INFINITY, 1e-12] {
        assert!(nurbs_frustum_translated_step_import_demo_json(&step, chord).is_err());
    }
    for text in [
        String::new(),
        "not STEP".into(),
        step[..step.len() / 2].into(),
        format!("{step}garbage"),
        "x".repeat(2 * 1024 * 1024 + 1),
    ] {
        assert!(nurbs_frustum_translated_step_import_demo_json(&text, 0.1).is_err());
    }
    let rotated = NurbsFrustumSolid::new(
        Transform::translation(Vec3::new(12., -5., 8.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.2).unwrap())
            .unwrap(),
        [16., 8.],
        24.,
        policy,
    )
    .unwrap();
    assert!(matches!(
        nurbs_frustum_translated_step_import_demo_json(
            &rotated.export_step_mm(policy).unwrap(),
            0.1
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_bounded_analytic_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        nurbs_frustum_translated_step_import_demo_json(&step, 0.1).unwrap(),
        original
    );
}
