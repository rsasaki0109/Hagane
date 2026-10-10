use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn call(session: &mut NurbsFrustumWorkflowSession, command: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &command.to_string()).unwrap(),
    )
    .unwrap()
}
fn doc(kind: &str) -> Value {
    let mut d = json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"stock","operations":[{"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}]});
    if kind == "bore" {
        d["operations"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"coaxial_bore","id":"result","input":"stock","radius":4.}));
        d["output"] = json!("result");
    }
    if kind == "split" {
        d["operations"].as_array_mut().unwrap().push(json!({"kind":"plane_split","id":"result","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}));
        d["output"] = json!("result");
    }
    d
}
#[test]
fn actual_canonical_and_bored_segment_reports_are_mesh_free_and_preserve_snapshots() {
    for kind in ["canonical", "bore"] {
        let mut session = NurbsFrustumWorkflowSession::new();
        let document = doc(kind);
        let accepted = call(
            &mut session,
            json!({"command":"rebuild","document":document}),
        );
        assert_eq!(accepted["ok"], true);
        let shape = session.accepted_shape().unwrap().clone();
        let history = (session.undo_count(), session.redo_count());
        let report = call(
            &mut session,
            json!({"command":"segment","component":0,"start":[-20.,0.,12.],"end":[20.,0.,12.]}),
        );
        assert_eq!(report["ok"], true, "{report}");
        let intervals = if kind == "bore" {
            json!([[0.2, 0.4], [0.6, 0.8]])
        } else {
            json!([[0.2, 0.8]])
        };
        let actual = report["segment"]["material_intervals"].as_array().unwrap();
        assert_eq!(actual.len(), intervals.as_array().unwrap().len());
        for (a, b) in actual.iter().zip(intervals.as_array().unwrap()) {
            for i in 0..2 {
                assert!((a[i].as_f64().unwrap() - b[i].as_f64().unwrap()).abs() < 1e-12);
            }
        }
        let expected_hits = if kind == "bore" { 4 } else { 2 };
        assert_eq!(
            report["segment"]["hits"].as_array().unwrap().len(),
            expected_hits
        );
        for hit in report["segment"]["hits"].as_array().unwrap() {
            let parameter = hit["parameter"].as_f64().unwrap();
            assert!((hit["point"][0].as_f64().unwrap() - (-20. + 40. * parameter)).abs() < 1e-10);
            assert!(!hit["faces"].as_array().unwrap().is_empty());
        }
        assert_eq!(report["segment"]["mesh_free"], true);
        for key in [
            "components",
            "component_steps",
            "mesh",
            "cache",
            "history",
            "document",
        ] {
            assert!(report.get(key).is_none());
        }
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
        let repeated = call(
            &mut session,
            json!({"command":"rebuild","document":doc(kind)}),
        );
        assert_eq!(repeated["component_steps"], accepted["component_steps"]);
        assert_eq!(repeated["cache"]["evaluated_nodes"], 0);
    }
}
#[test]
fn segment_errors_and_oblique_scope_preserve_accepted_state_and_recover() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let request =
        json!({"command":"segment","component":0,"start":[-20.,0.,12.],"end":[20.,0.,12.]});
    assert_eq!(
        call(&mut session, request.clone())["diagnostic"]["code"],
        "no_accepted_document"
    );
    for kind in ["canonical", "split"] {
        let accepted = call(
            &mut session,
            json!({"command":"rebuild","document":doc(kind)}),
        );
        let shape = session.accepted_shape().unwrap().clone();
        let history = (session.undo_count(), session.redo_count());
        let mut bad = request.clone();
        bad["component"] = json!(9);
        assert_eq!(call(&mut session, bad)["ok"], false);
        let mut bad = request.clone();
        bad["extra"] = json!(true);
        assert_eq!(call(&mut session, bad)["ok"], false);
        let mut bad = request.clone();
        bad["start"] = json!([1e14, 0., 12.]);
        assert_eq!(call(&mut session, bad)["ok"], false);
        assert_eq!(call(&mut session, request.clone())["ok"], true);
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
        let replay = call(
            &mut session,
            json!({"command":"rebuild","document":doc(kind)}),
        );
        assert_eq!(replay["component_steps"], accepted["component_steps"]);
        assert_eq!(replay["cache"]["evaluated_nodes"], 0);
    }
}
