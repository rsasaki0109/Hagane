use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"bored","operations":[
        {"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},
        {"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.},
        {"kind":"select","id":"selected","input":"bored","component":0}]})
}
fn command(session: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &value.to_string()).unwrap(),
    )
    .unwrap()
}
fn rebuild(session: &mut NurbsFrustumWorkflowSession, doc: Value) -> Value {
    command(session, json!({"command":"rebuild","document":doc}))
}
#[test]
fn actual_cached_bore_report_matches_direct_body_and_selection_reuses_geometry() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    let report = rebuild(&mut session, doc.clone());
    assert_eq!(report["ok"], true);
    let direct: Value = serde_json::from_str(
        &nurbs_frustum_bore_demo_json(&[16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1, 4.]).unwrap(),
    )
    .unwrap();
    let mut actual = report["components"][0].clone();
    actual.as_object_mut().unwrap().remove("component_kind");
    actual
        .as_object_mut()
        .unwrap()
        .remove("placement_reference");
    assert_eq!(actual, direct);
    assert_eq!(report["component_steps"][0], direct["step"]);
    assert_eq!(report["output"]["total_volume"], direct["volume"]);
    let retained = session.accepted_shape().unwrap().components()[0]
        .bored()
        .unwrap()
        .clone();
    doc["output"] = json!("selected");
    let selected = rebuild(&mut session, doc.clone());
    assert_eq!(selected["components"], report["components"]);
    assert_eq!(selected["cache"]["evaluated_nodes"], 0);
    assert!(Arc::ptr_eq(
        &retained,
        session.accepted_shape().unwrap().components()[0]
            .bored()
            .unwrap()
    ));
    doc["display_chord_tolerance"] = json!(0.2);
    let coarse = rebuild(&mut session, doc);
    assert_eq!(coarse["component_steps"], selected["component_steps"]);
    assert_eq!(coarse["cache"]["evaluated_nodes"], 0);
    assert!(Arc::ptr_eq(
        &retained,
        session.accepted_shape().unwrap().components()[0]
            .bored()
            .unwrap()
    ));
    assert_eq!(
        command(&mut session, json!({"command":"undo"}))["components"],
        selected["components"]
    );
    assert_eq!(
        command(&mut session, json!({"command":"redo"}))["components"],
        coarse["components"]
    );
}
#[test]
fn invalid_radius_or_display_rolls_back_actual_acceptance_and_recovers() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let accepted = rebuild(&mut session, doc.clone());
    let retained = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for (key, value) in [("radius", 8.), ("radius", -1.), ("display", 1e-12)] {
        let mut bad = doc.clone();
        if key == "radius" {
            bad["operations"][1]["radius"] = json!(value);
        } else {
            bad["display_chord_tolerance"] = json!(value);
        }
        assert_eq!(rebuild(&mut session, bad)["ok"], false);
        assert!(Arc::ptr_eq(&retained, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    let recovered = rebuild(&mut session, doc);
    assert_eq!(recovered["components"], accepted["components"]);
    assert_eq!(recovered["cache"]["evaluated_nodes"], 0);
}
