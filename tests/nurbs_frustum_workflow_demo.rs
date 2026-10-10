use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn command(session: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &value.to_string()).unwrap(),
    )
    .unwrap()
}
fn rebuild(session: &mut NurbsFrustumWorkflowSession, doc: &NurbsFrustumWorkflowDocument) -> Value {
    command(session, json!({"command":"rebuild","document":doc}))
}
fn check_report(data: &Value, body: &NurbsFrustumSolid, chord: f64) {
    let tol = GeometryTolerance::default();
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let frame = body.frame();
    let mass = body.mass_properties(tol).unwrap();
    assert_eq!(
        data["placement"],
        json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)})
    );
    assert_eq!(data["axis"], json!(xyz(frame.axes()[2])));
    assert_eq!(data["radii"], json!(body.radii()));
    assert_eq!(data["height"], body.height());
    assert_eq!(data["volume"], mass.volume);
    assert_eq!(data["centroid"], json!(xyz(mass.centroid)));
    assert_eq!(
        data["inertia"],
        json!(body.inertia_properties(tol).unwrap().inertia)
    );
    let mesh = body.tessellate(chord, tol).unwrap();
    assert_eq!(
        data["mesh"]["positions"],
        json!(mesh.positions.into_iter().map(xyz).collect::<Vec<_>>())
    );
    assert_eq!(
        data["mesh"]["normals"],
        json!(mesh.normals.into_iter().map(xyz).collect::<Vec<_>>())
    );
    assert_eq!(data["mesh"]["triangles"], json!(mesh.triangles));
    assert_eq!(data["step"], body.export_step_mm(tol).unwrap());
}

#[test]
fn selected_and_multi_output_reports_use_actual_parts_with_separate_exports() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = nurbs_frustum_workflow_example_document().unwrap();
    let selected = rebuild(&mut session, &doc);
    assert_eq!(selected["ok"], true);
    assert_eq!(selected["output"]["status"], "single");
    assert_eq!(selected["components"].as_array().unwrap().len(), 1);
    let shape = session.accepted_shape().unwrap();
    check_report(
        &selected["components"][0],
        &shape.components()[0],
        doc.display_chord_tolerance,
    );
    assert_eq!(
        selected["component_steps"][0],
        selected["components"][0]["step"]
    );
    let snapshots: Vec<_> = (0..3)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    doc.output = "parts".into();
    let multi = rebuild(&mut session, &doc);
    assert_eq!(multi["ok"], true);
    assert_eq!(multi["output"]["status"], "multiple");
    assert_eq!(multi["output"]["component_count"], 4);
    assert_eq!(multi["cache"]["evaluated_nodes"], 0);
    assert_eq!(multi["cache"]["reused_nodes"], 3);
    for (i, snapshot) in snapshots.iter().enumerate() {
        assert!(Arc::ptr_eq(snapshot, session.prefix_snapshot(i).unwrap()));
    }
    let shape = session.accepted_shape().unwrap();
    let mut sum = 0.;
    for (i, body) in shape.components().iter().enumerate() {
        check_report(&multi["components"][i], body, doc.display_chord_tolerance);
        assert_eq!(multi["component_steps"][i], multi["components"][i]["step"]);
        let imported = import_step_nurbs_frustum_cardinal_mm(
            multi["component_steps"][i].as_str().unwrap(),
            GeometryTolerance::default(),
        )
        .unwrap();
        assert_eq!(
            format!("{:?}", imported.solid()),
            format!("{:?}", body.solid())
        );
        sum += body.volume(GeometryTolerance::default()).unwrap();
    }
    assert!((multi["output"]["total_volume"].as_f64().unwrap() - sum).abs() <= 1e-11 * sum);
    let expected = std::f64::consts::PI * 24. * (16f64.powi(2) + 16. * 8. + 8f64.powi(2)) / 3.;
    assert!((sum - expected).abs() <= 1e-11 * expected);
}

#[test]
fn failed_requests_and_display_serialize_before_commit_preserving_history_and_cache() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = nurbs_frustum_workflow_example_document().unwrap();
    assert_eq!(rebuild(&mut session, &doc)["ok"], true);
    let accepted = session.accepted_shape().unwrap().clone();
    let snapshots: Vec<_> = (0..3)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    let old_history = (session.undo_count(), session.redo_count());
    let mut bad_display = doc.clone();
    bad_display.display_chord_tolerance = 1e-12;
    let failed = rebuild(&mut session, &bad_display);
    assert_eq!(failed["ok"], false);
    assert_eq!(failed["accepted_document"], json!(doc));
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    for (i, snapshot) in snapshots.iter().enumerate() {
        assert!(Arc::ptr_eq(snapshot, session.prefix_snapshot(i).unwrap()));
    }
    assert_eq!((session.undo_count(), session.redo_count()), old_history);
    let mut bad = json!({"command":"rebuild","document":doc});
    bad["document"]["operations"][1]["cuts"] = json!([0.]);
    assert_eq!(command(&mut session, bad)["ok"], false);
    for text in [
        "not JSON",
        "{\"command\":\"rebuild\"}",
        "{\"command\":\"undo\",\"unexpected\":1}",
    ] {
        let result: Value = serde_json::from_str(
            &nurbs_frustum_workflow_session_command_json(&mut session, text).unwrap(),
        )
        .unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(result["diagnostic"]["code"], "invalid_request");
    }
    assert!(nurbs_frustum_workflow_session_command_json(
        &mut session,
        &"x".repeat(2 * 1024 * 1024 + 1)
    )
    .is_err());
    assert_eq!(session.accepted_document(), Some(&doc));
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    assert_eq!((session.undo_count(), session.redo_count()), old_history);
    let recovered = rebuild(&mut session, &doc);
    assert_eq!(recovered["ok"], true);
    assert_eq!(recovered["cache"]["reused_nodes"], 3);
    assert_eq!(recovered["cache"]["evaluated_nodes"], 0);
}

#[test]
fn undo_redo_reset_and_step_stock_commands_report_actual_accepted_bodies() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = nurbs_frustum_workflow_example_document().unwrap();
    let selected = rebuild(&mut session, &doc);
    doc.output = "parts".into();
    let multi = rebuild(&mut session, &doc);
    let undo = command(&mut session, json!({"command":"undo"}));
    assert_eq!(undo["components"], selected["components"]);
    assert_eq!(undo["history"]["can_redo"], true);
    let redo = command(&mut session, json!({"command":"redo"}));
    assert_eq!(redo["components"], multi["components"]);
    let step = multi["component_steps"][1].as_str().unwrap();
    let mut loaded = serde_json::to_value(&doc).unwrap();
    loaded["output"] = json!("imported");
    loaded["operations"] = json!([{"kind":"step_stock","id":"imported","step":step}]);
    let result = command(&mut session, json!({"command":"rebuild","document":loaded}));
    assert_eq!(result["ok"], true);
    assert_eq!(result["components"][0], multi["components"][1]);
    let reset = command(&mut session, json!({"command":"reset"}));
    assert_eq!(reset["reset"], true);
    assert!(session.accepted_document().is_none());
    assert!(session.accepted_shape().is_none());
    assert_eq!((session.undo_count(), session.redo_count()), (0, 0));
    assert_eq!(reset["components"], json!([]));
    assert_eq!(
        command(&mut session, json!({"command":"undo"}))["ok"],
        false
    );
    assert_eq!(rebuild(&mut session, &doc)["ok"], true);
}

#[test]
fn aggregate_display_budget_refuses_individually_valid_meshes_before_commit() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let original = nurbs_frustum_workflow_example_document().unwrap();
    assert_eq!(rebuild(&mut session, &original)["ok"], true);
    let accepted = session.accepted_shape().unwrap().clone();
    let prefixes: Vec<_> = (0..3)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    let history = (session.undo_count(), session.redo_count());
    let tolerance = GeometryTolerance::default();
    let source = NurbsFrustumSolid::new(
        Frame3::new_with_tolerance(
            Point3::new(12., -5., 8.),
            [
                Vec3::new(0., 1., 0.),
                Vec3::new(0., 0., 1.),
                Vec3::new(1., 0., 0.),
            ],
            tolerance,
        )
        .unwrap(),
        [16., 8.],
        24.,
        tolerance,
    )
    .unwrap();
    let cuts: Vec<_> = (1..=16).map(|i| i as f64).collect();
    let parts = source.split_axial_many(&cuts, tolerance).unwrap();
    let chord = 0.02;
    let triangles: usize = parts
        .parts
        .iter()
        .map(|part| part.tessellate(chord, tolerance).unwrap().triangles.len())
        .sum();
    assert!(
        triangles > 131072,
        "fixture actual triangle count {triangles}"
    );
    let mut value = serde_json::to_value(&original).unwrap();
    value["output"] = json!("parts");
    value["display_chord_tolerance"] = json!(chord);
    value["operations"][1]["cuts"] = json!(cuts);
    let result = command(&mut session, json!({"command":"rebuild","document":value}));
    assert_eq!(result["ok"], false);
    assert!(result["diagnostic"]["message"]
        .as_str()
        .unwrap()
        .contains("aggregate display"));
    assert_eq!(session.accepted_document(), Some(&original));
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    for (i, prefix) in prefixes.iter().enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
    }
    assert_eq!((session.undo_count(), session.redo_count()), history);
    assert_eq!(rebuild(&mut session, &original)["ok"], true);
}
