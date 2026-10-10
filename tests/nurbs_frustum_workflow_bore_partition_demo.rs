use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn call(session: &mut NurbsFrustumWorkflowSession, command: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &command.to_string()).unwrap(),
    )
    .unwrap()
}
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"parts","operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.},{"kind":"partition","id":"parts","input":"bored","cuts":[6.,12.,18.]},{"kind":"select","id":"selected","input":"parts","component":2}]})
}
fn rebuild(s: &mut NurbsFrustumWorkflowSession, d: Value) -> Value {
    call(s, json!({"command":"rebuild","document":d}))
}
#[test]
fn every_partition_report_is_an_actual_annular_child_and_selection_can_partition_again() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    let report = rebuild(&mut session, doc.clone());
    assert_eq!(report["ok"], true, "{report}");
    assert_eq!(report["components"].as_array().unwrap().len(), 4);
    let mut sum = 0.;
    for i in 0..4 {
        let r0 = 16. - 2. * i as f64;
        let r1 = r0 - 2.;
        let z = 6. * i as f64;
        let direct: Value = serde_json::from_str(
            &nurbs_frustum_bore_demo_json(&[r0, r1, 6., 0., 0., 0., z, 1e-6, 0.1, 4.]).unwrap(),
        )
        .unwrap();
        let mut actual = report["components"][i].clone();
        actual.as_object_mut().unwrap().remove("component_kind");
        actual
            .as_object_mut()
            .unwrap()
            .remove("placement_reference");
        assert_eq!(actual, direct);
        assert_eq!(actual["genus"], 1);
        assert_eq!(actual["brep"]["wire_edges"][0].as_array().unwrap().len(), 2);
        assert_eq!(report["component_steps"][i], actual["step"]);
        sum += actual["volume"].as_f64().unwrap();
        let ray = call(
            &mut session,
            json!({"command":"ray","component":i,"origin":[0.,0.,z+3.],"direction":[2.,0.,0.]}),
        );
        assert_eq!(ray["ok"], true, "{ray}");
        assert!((ray["line"]["material_intervals"][0][0].as_f64().unwrap() - 2.).abs() < 1e-10);
    }
    assert!((sum - report["output"]["total_volume"].as_f64().unwrap()).abs() < 1e-10);
    let selected = session.accepted_shape().unwrap().components()[2]
        .bored()
        .unwrap()
        .clone();
    doc["output"] = json!("selected");
    assert_eq!(rebuild(&mut session, doc.clone())["ok"], true);
    assert!(Arc::ptr_eq(
        &selected,
        session.accepted_shape().unwrap().components()[0]
            .bored()
            .unwrap()
    ));
    doc["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"partition","id":"fine","input":"selected","cuts":[2.,4.]}));
    doc["output"] = json!("fine");
    let fine = rebuild(&mut session, doc);
    assert_eq!(fine["ok"], true, "{fine}");
    assert_eq!(fine["components"].as_array().unwrap().len(), 3);
    for body in fine["components"].as_array().unwrap() {
        assert_eq!(body["hole_radius"], 4.);
        assert_eq!(body["genus"], 1);
        assert_eq!(body["height"], 2.);
    }
    assert!(
        (fine["output"]["total_volume"].as_f64().unwrap()
            - selected
                .volume(GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap())
                .unwrap())
        .abs()
            < 1e-10
    );
}
#[test]
fn radius_cut_and_display_edits_have_real_prefix_reuse_and_atomic_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    let initial = rebuild(&mut session, doc.clone());
    assert_eq!(initial["ok"], true);
    let root = session.prefix_snapshot(0).unwrap().clone();
    let bored = session.prefix_snapshot(1).unwrap().clone();
    doc["operations"][2]["cuts"] = json!([5., 12., 18.]);
    let changed = rebuild(&mut session, doc.clone());
    assert_eq!(changed["cache"]["reused_nodes"], 2);
    assert!(Arc::ptr_eq(&bored, session.prefix_snapshot(1).unwrap()));
    doc["operations"][1]["radius"] = json!(3.);
    let radius = rebuild(&mut session, doc.clone());
    assert_eq!(radius["cache"]["reused_nodes"], 1);
    assert!(Arc::ptr_eq(&root, session.prefix_snapshot(0).unwrap()));
    doc["display_chord_tolerance"] = json!(0.2);
    let coarse = rebuild(&mut session, doc.clone());
    assert_eq!(coarse["cache"]["evaluated_nodes"], 0);
    assert_eq!(coarse["component_steps"], radius["component_steps"]);
    assert_eq!(
        call(&mut session, json!({"command":"undo"}))["components"],
        radius["components"]
    );
    assert_eq!(
        call(&mut session, json!({"command":"redo"}))["components"],
        coarse["components"]
    );
    let accepted = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    let mut bad = doc.clone();
    bad["operations"][2]["cuts"] = json!([0.]);
    assert_eq!(rebuild(&mut session, bad)["ok"], false);
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    assert_eq!((session.undo_count(), session.redo_count()), history);
    assert_eq!(
        rebuild(&mut session, doc)["component_steps"],
        coarse["component_steps"]
    );
}
