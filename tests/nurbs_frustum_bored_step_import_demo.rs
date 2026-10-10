use hagane::*;
use serde_json::Value;
fn sample() -> NurbsBoredFrustumSolid {
    let policy = GeometryTolerance::default();
    NurbsFrustumSolid::new(
        Frame3::new_with_tolerance(
            Point3::new(12., -5., 8.),
            [
                Vec3::new(0., 1., 0.),
                Vec3::new(0., 0., 1.),
                Vec3::new(1., 0., 0.),
            ],
            policy,
        )
        .unwrap(),
        [16., 8.],
        24.,
        policy,
    )
    .unwrap()
    .split_axial(10., policy)
    .unwrap()
    .upper
    .bore_coaxial(4., policy)
    .unwrap()
}
#[test]
fn actual_imported_upper_child_report_retains_geometry_mesh_mass_and_opening() {
    let policy = GeometryTolerance::default();
    let source = sample();
    let text = source.export_step_mm(policy).unwrap();
    let imported = import_step_nurbs_bored_frustum_cardinal_mm(&text, policy).unwrap();
    let report: Value =
        serde_json::from_str(&nurbs_frustum_bored_step_import_demo_json(&text, 0.2).unwrap())
            .unwrap();
    assert_eq!(report["import"]["kind"], "nurbs_bored_frustum_cardinal");
    assert_eq!(report["import"]["source_geometry_preserved"], true);
    assert_eq!(report["genus"], 1);
    assert_eq!(report["brep"]["faces"], 10);
    assert_eq!(report["brep"]["edges"], 24);
    assert_eq!(report["brep"]["vertices"].as_array().unwrap().len(), 16);
    assert_eq!(report["brep"]["wire_edges"][0].as_array().unwrap().len(), 2);
    assert_eq!(report["step"], imported.export_step_mm(policy).unwrap());
    let mesh = imported.tessellate(0.2, policy).unwrap();
    assert_eq!(
        report["mesh"]["positions"],
        serde_json::json!(mesh
            .positions
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<_>>())
    );
    assert_eq!(
        report["mesh"]["triangles"],
        serde_json::json!(mesh.triangles)
    );
    let mass = imported.inertia_properties(policy).unwrap();
    assert_eq!(
        report["mass_properties"]["centroid"],
        serde_json::json!([mass.centroid.x, mass.centroid.y, mass.centroid.z])
    );
    assert_eq!(
        report["mass_properties"]["inertia"],
        serde_json::json!(mass.inertia)
    );
    let frame = imported.source().frame();
    assert_eq!(
        report["placement"]["translation"],
        serde_json::json!([frame.origin().x, frame.origin().y, frame.origin().z])
    );
    assert_eq!(
        imported
            .classify_point(frame.origin() + frame.axes()[2] * 7., policy)
            .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        imported
            .classify_point(
                frame.origin() + frame.axes()[0] * 6. + frame.axes()[2] * 7.,
                policy
            )
            .unwrap(),
        PointLocation::Inside
    );
    assert!(import_step_nurbs_frustum_cardinal_mm(&text, policy).is_err());
    assert!((report["volume"].as_f64().unwrap() - source.volume(policy).unwrap()).abs() < 1e-8);
}
#[test]
fn invalid_scope_text_or_chord_produces_no_phantom_import_report_and_recovers() {
    let policy = GeometryTolerance::default();
    let text = sample().export_step_mm(policy).unwrap();
    let accepted = nurbs_frustum_bored_step_import_demo_json(&text, 0.2).unwrap();
    for error in [0., -1., f64::NAN, 1e-12] {
        assert!(nurbs_frustum_bored_step_import_demo_json(&text, error).is_err());
    }
    for bad in ["", "ISO-10303-21;END-ISO-10303-21;"] {
        assert!(nurbs_frustum_bored_step_import_demo_json(bad, 0.2).is_err());
    }
    let plain = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy)
        .unwrap()
        .export_step_mm(policy)
        .unwrap();
    assert!(nurbs_frustum_bored_step_import_demo_json(&plain, 0.2).is_err());
    let (sin, cos) = 0.37_f64.sin_cos();
    let posed = NurbsFrustumSolid::new(
        Frame3::new_with_tolerance(
            Point3::new(0., 0., 0.),
            [
                Vec3::new(cos, 0., -sin),
                Vec3::new(0., 1., 0.),
                Vec3::new(sin, 0., cos),
            ],
            policy,
        )
        .unwrap(),
        [16., 8.],
        24.,
        policy,
    )
    .unwrap()
    .bore_coaxial(4., policy)
    .unwrap()
    .export_step_mm(policy)
    .unwrap();
    assert!(nurbs_frustum_bored_step_import_demo_json(&posed, 0.2).is_err());
    assert_eq!(
        nurbs_frustum_bored_step_import_demo_json(&text, 0.2).unwrap(),
        accepted
    );
}
