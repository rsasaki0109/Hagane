use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document(angle: f64) -> Value {
    let mut doc = serde_json::to_value(nurbs_frustum_workflow_example_document().unwrap()).unwrap();
    doc["operations"][0] = json!({"kind":"posed_frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-5.,8.],"rotation_axis":[1.,2.,3.],"angle":angle});
    doc
}
fn rebuild(session: &mut NurbsFrustumWorkflowSession, doc: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(
            session,
            &json!({"command":"rebuild","document":doc}).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn posed_pipeline_reports_actual_world_geometry_and_rebuilds_pose_dependencies() {
    let mut session = NurbsFrustumWorkflowSession::new();
    for angle in [0.37, -0.63] {
        let report = rebuild(&mut session, document(angle));
        assert_eq!(report["ok"], true);
        assert_eq!(report["cache"]["evaluated_nodes"], 3);
        assert_eq!(report["cache"]["reused_nodes"], 0);
        let tol = GeometryTolerance::default();
        let frame = Transform::translation(Vec3::new(12., -5., 8.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap())
            .unwrap();
        let original = NurbsFrustumSolid::new(frame, [16., 8.], 24., tol).unwrap();
        let expected = original.split_axial_many(&[6., 12., 18.], tol).unwrap();
        let body = &expected.parts[1];
        let actual = &report["components"][0];
        let xyz = |v: Vec3| [v.x, v.y, v.z];
        assert_eq!(
            actual["placement"],
            json!({"translation":xyz(body.frame().origin()),"axes":body.frame().axes().map(xyz)})
        );
        assert_eq!(
            actual["inertia"],
            json!(body.inertia_properties(tol).unwrap().inertia)
        );
        let bounds = body.bounds(tol).unwrap();
        assert_eq!(
            actual["bounds"],
            json!({"min":xyz(bounds.min),"max":xyz(bounds.max)})
        );
        let mesh = body.tessellate(0.1, tol).unwrap();
        assert_eq!(
            actual["mesh"]["positions"],
            json!(mesh.positions.into_iter().map(xyz).collect::<Vec<_>>())
        );
        assert_eq!(actual["step"], body.export_step_mm(tol).unwrap());
        assert!(matches!(
            import_step_nurbs_frustum_cardinal_mm(actual["step"].as_str().unwrap(), tol),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(
            format!(
                "{:?}",
                session.accepted_shape().unwrap().components()[0].solid()
            ),
            format!("{:?}", body.solid())
        );
    }
}
#[test]
fn invalid_pose_and_display_preserve_accepted_shape_cache_and_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let good = document(0.37);
    assert_eq!(rebuild(&mut session, good.clone())["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let snapshots: Vec<_> = (0..3)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    let history = (session.undo_count(), session.redo_count());
    for (field, value) in [("angle", json!(4.)), ("rotation_axis", json!([0., 0., 0.]))] {
        let mut bad = good.clone();
        bad["operations"][0][field] = value;
        assert_eq!(rebuild(&mut session, bad)["ok"], false);
    }
    let mut bad = good.clone();
    bad["display_chord_tolerance"] = json!(1e-12);
    assert_eq!(rebuild(&mut session, bad)["ok"], false);
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    for (i, snapshot) in snapshots.iter().enumerate() {
        assert!(Arc::ptr_eq(snapshot, session.prefix_snapshot(i).unwrap()));
    }
    assert_eq!((session.undo_count(), session.redo_count()), history);
    let recovered = rebuild(&mut session, good);
    assert_eq!(recovered["cache"]["reused_nodes"], 3);
}
