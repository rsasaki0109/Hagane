use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.2,"output":"cut","operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.},{"kind":"plane_split","id":"cut","input":"bored","origin":[0.,0.,12.],"normal":[0.2,0.,1.]},{"kind":"select","id":"selected","input":"cut","component":1}]})
}
fn call(s: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(s, &value.to_string()).unwrap(),
    )
    .unwrap()
}
#[test]
fn actual_annular_child_queries_are_small_and_preserve_geometry_metrics_and_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    assert_eq!(accepted["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let before = format!("{:?}", shape);
    let history = (session.undo_count(), session.redo_count());
    let prefixes: Vec<_> = (0..4)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    for (point, classes) in [
        ([6., 0., 6.], ["inside", "outside"]),
        ([6., 0., 18.], ["outside", "inside"]),
        ([0., 0., 6.], ["outside", "outside"]),
        ([6., 0., 10.8], ["boundary", "boundary"]),
        ([0., 0., 12.], ["outside", "outside"]),
        ([4., 0., 6.], ["boundary", "outside"]),
    ] {
        for (component, location) in classes.iter().enumerate() {
            let query = call(
                &mut session,
                json!({"command":"query","component":component,"point":point}),
            );
            assert_eq!(query["ok"], true, "{query}");
            assert_eq!(query["query"]["location"], *location);
            assert_eq!(query["query"]["mesh_free"], true);
            for key in ["components", "component_steps", "mesh", "history", "cache"] {
                assert!(query.get(key).is_none());
            }
        }
    }
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    assert_eq!(before, format!("{:?}", session.accepted_shape().unwrap()));
    assert_eq!((session.undo_count(), session.redo_count()), history);
    for (i, prefix) in prefixes.iter().enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
    }
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
#[test]
fn selected_child_query_and_refusal_leave_accepted_state_and_exports_unchanged() {
    let mut rim_session = NurbsFrustumWorkflowSession::new();
    let mut rim_doc = document();
    rim_doc["operations"][0]["radii"] = json!([12., 12.]);
    rim_doc["operations"][2]["normal"] = json!([0., 0., 1.]);
    assert_eq!(
        call(
            &mut rim_session,
            json!({"command":"rebuild","document":rim_doc})
        )["ok"],
        true
    );
    let rim_shape = rim_session.accepted_shape().unwrap().clone();
    let ambiguous = call(
        &mut rim_session,
        json!({"command":"query","component":0,"point":[4.-0.8e-6,0.,12.+0.8e-6]}),
    );
    assert_eq!(ambiguous["ok"], false);
    assert_eq!(ambiguous["diagnostic"]["category"], "unsupported");
    assert!(Arc::ptr_eq(
        &rim_shape,
        rim_session.accepted_shape().unwrap()
    ));

    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    doc["output"] = json!("selected");
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    assert_eq!(accepted["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[6.,0.,18.]})
        )["query"]["location"],
        "inside"
    );
    for bad in [
        json!({"command":"query","component":1,"point":[6.,0.,18.]}),
        json!({"command":"query","component":0,"point":[1e14,0.,18.]}),
        json!({"command":"query","component":0,"point":[6.,0.,18.],"extra":true}),
    ] {
        assert_eq!(call(&mut session, bad)["ok"], false);
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    let malformed = nurbs_frustum_workflow_session_command_json(
        &mut session,
        "{\"command\":\"query\",\"component\":0,\"point\":[1e999,0,18]}",
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&malformed).unwrap()["ok"],
        false
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[0.,0.,18.]})
        )["query"]["location"],
        "outside"
    );
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
}
