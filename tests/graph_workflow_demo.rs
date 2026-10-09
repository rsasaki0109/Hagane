use hagane::*;
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_str(include_str!("../docs/graph-workflow.json")).unwrap()
}
fn apply(session: &mut GraphWorkflowSession, doc: &Value) -> Value {
    serde_json::from_str(&session.evaluate_json(&doc.to_string()).unwrap()).unwrap()
}
#[test]
fn actual_retained_geometry_drives_reports_export_and_queries() {
    let mut session = GraphWorkflowSession::new();
    let doc = fixture();
    let report = apply(&mut session, &doc);
    assert_eq!(report["ok"], true);
    assert_eq!(report["document"], doc);
    assert_eq!(report["shape"]["kind"], "circular");
    assert_eq!(report["shape"]["brep"]["faces"], 10);
    assert_eq!(report["shape"]["brep"]["closed"], true);
    assert!(!report["shape"]["mesh"]["triangles"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(report["shape"]["inertia_error"].is_null());
    let step: Value =
        serde_json::from_str(&graph_workflow_step_export_json(&session).unwrap()).unwrap();
    assert_eq!(
        step["step"].as_str().unwrap(),
        session
            .accepted_shape()
            .unwrap()
            .export_step_mm(Tolerance::default())
            .unwrap()
    );
    let world = session
        .accepted_shape()
        .unwrap()
        .source()
        .placement()
        .point(Point3::new(40., 30., 10.));
    let query: Value =
        serde_json::from_str(&graph_workflow_point_query_json(&session, world, 1e-5).unwrap())
            .unwrap();
    assert_eq!(query["location"], "Outside");
}
#[test]
fn rejected_document_display_query_and_transport_preserve_accepted_export() {
    let mut session = GraphWorkflowSession::new();
    let doc = fixture();
    assert_eq!(apply(&mut session, &doc)["ok"], true);
    let previous = graph_workflow_step_export_json(&session).unwrap();
    let mut bad = doc.clone();
    bad["operations"][3]["radius"] = serde_json::json!(30.);
    assert_eq!(apply(&mut session, &bad)["ok"], false);
    let mut display = doc.clone();
    display["display"]["max_error"] = serde_json::json!(1e-12);
    assert_eq!(apply(&mut session, &display)["ok"], false);
    let (status, report) = graph_workflow_output(session.evaluate_json("not JSON"));
    assert_eq!(status, 1);
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["ok"], false);
    assert!(
        graph_workflow_point_query_json(&session, Point3::new(f64::NAN, 0., 0.), 1e-5).is_err()
    );
    assert!(graph_workflow_point_query_json(&session, Point3::new(0., 0., 0.), 0.).is_err());
    assert_eq!(graph_workflow_step_export_json(&session).unwrap(), previous);
    assert_eq!(
        session.accepted_document().unwrap(),
        &serde_json::from_value::<GraphWorkflowDocument>(doc.clone()).unwrap()
    );
    assert_eq!(apply(&mut session, &doc)["ok"], true);
}
#[test]
fn numeric_output_protocol_and_empty_session_are_explicit() {
    let empty = GraphWorkflowSession::new();
    assert!(graph_workflow_step_export_json(&empty).is_err());
    assert!(graph_workflow_point_query_json(&empty, Point3::new(0., 0., 0.), 1e-5).is_err());
    assert_eq!(graph_workflow_output(Ok("{\"ok\":true}".into())).0, 0);
    assert_eq!(graph_workflow_output(Ok("{\"ok\":false}".into())).0, 1);
    assert_eq!(
        graph_workflow_output(Err(Error::InvalidInput("bad transport"))).0,
        1
    );
}
