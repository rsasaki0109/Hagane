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
fn queries_use_actual_canonical_bored_and_oblique_components_without_reports_or_mutation() {
    for (kind, index, point, location) in [
        ("canonical", 0, [0., 0., 6.], "inside"),
        ("bore", 0, [0., 0., 6.], "outside"),
        ("bore", 0, [6., 0., 6.], "inside"),
        ("split", 0, [0., 0., 6.], "inside"),
        ("split", 1, [0., 0., 6.], "outside"),
        ("split", 1, [0., 0., 12.], "boundary"),
    ] {
        let mut session = NurbsFrustumWorkflowSession::new();
        let document = doc(kind);
        assert_eq!(
            call(
                &mut session,
                json!({"command":"rebuild","document":document})
            )["ok"],
            true
        );
        let shape = session.accepted_shape().unwrap().clone();
        let prefixes: Vec<_> = (0..session.accepted_document().unwrap().operations.len())
            .map(|i| session.prefix_snapshot(i).unwrap().clone())
            .collect();
        let before = format!("{:?}", shape);
        let history = (session.undo_count(), session.redo_count());
        let report = call(
            &mut session,
            json!({"command":"query","component":index,"point":point}),
        );
        assert_eq!(report["ok"], true, "{report}");
        assert_eq!(report["query"]["location"], location);
        assert_eq!(report["query"]["point"], json!(point));
        assert_eq!(report["query"]["mesh_free"], true);
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
        assert_eq!(before, format!("{:?}", session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
        for (i, prefix) in prefixes.iter().enumerate() {
            assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
        }
    }
}
#[test]
fn strict_query_failures_preserve_document_cache_export_and_allow_recovery() {
    let mut session = NurbsFrustumWorkflowSession::new();
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[0.,0.,6.]})
        )["diagnostic"]["code"],
        "no_accepted_document"
    );
    let document = doc("canonical");
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":document.clone()}),
    );
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for bad in [
        json!({"command":"query","component":1,"point":[0.,0.,6.]}),
        json!({"command":"query","component":0,"point":[1e14,0.,6.]}),
        json!({"command":"query","component":0,"point":[0.,0.,6.],"extra":true}),
        json!({"command":"query","point":[0.,0.,6.]}),
        json!({"command":"query","component":-1,"point":[0.,0.,6.]}),
    ] {
        assert_eq!(call(&mut session, bad)["ok"], false);
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!(
            serde_json::to_value(session.accepted_document().unwrap()).unwrap(),
            document
        );
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    let malformed = nurbs_frustum_workflow_session_command_json(
        &mut session,
        "{\"command\":\"query\",\"component\":0,\"point\":[1e999,0,6]}",
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&malformed).unwrap()["ok"],
        false
    );
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[0.,0.,6.]})
        )["query"]["location"],
        "inside"
    );
    let replay = call(
        &mut session,
        json!({"command":"rebuild","document":document}),
    );
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
