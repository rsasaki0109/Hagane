use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn request(session: &mut PrismWorkflowSession, value: Value) -> Value {
    serde_json::from_str(&prism_workflow_session_command_json(session, &value.to_string()).unwrap())
        .unwrap()
}
fn rebuild(session: &mut PrismWorkflowSession, document: &PrismWorkflowDocument) -> Value {
    let report = request(session, json!({"command":"rebuild","document":document}));
    assert_eq!(report["ok"], true);
    report
}
fn rejected_without_mutation(session: &mut PrismWorkflowSession, value: Value) {
    let document = session.accepted_document().unwrap().clone();
    let shape = session.accepted_shape().unwrap().clone();
    let prefixes: Vec<_> = (0..document.operations.len())
        .map(|index| session.prefix_snapshot(index).unwrap().clone())
        .collect();
    let history = (session.undo_count(), session.redo_count());
    let before = format!("{:?}", shape.components());
    let report = request(session, value);
    assert_eq!(report["ok"], false);
    assert_eq!(report["diagnostic"]["code"], "invalid_request");
    assert_eq!(
        report["accepted_document"],
        serde_json::to_value(&document).unwrap()
    );
    assert_eq!(session.accepted_document(), Some(&document));
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    for (index, prefix) in prefixes.iter().enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(index).unwrap()));
    }
    assert_eq!(history, (session.undo_count(), session.redo_count()));
    assert_eq!(
        before,
        format!("{:?}", session.accepted_shape().unwrap().components())
    );
}

#[test]
fn unknown_command_fields_preserve_actual_geometry_and_history_then_valid_commands_recover() {
    let mut session = PrismWorkflowSession::new();
    let initial = prism_workflow_example_document().unwrap();
    let first = rebuild(&mut session, &initial);
    let mut edited = initial.clone();
    edited.display_chord_tolerance *= 2.;
    let second = rebuild(&mut session, &edited);
    assert_eq!((session.undo_count(), session.redo_count()), (1, 0));

    rejected_without_mutation(&mut session, json!({"command":"undo","unexpected":true}));
    let undo = request(&mut session, json!({"command":"undo"}));
    assert_eq!(undo["ok"], true);
    assert_eq!(undo["document"], first["document"]);
    assert_eq!(undo["components"], first["components"]);
    assert_eq!(undo["component_steps"], first["component_steps"]);
    assert_eq!((session.undo_count(), session.redo_count()), (0, 1));

    rejected_without_mutation(&mut session, json!({"command":"redo","unexpected":null}));
    let redo = request(&mut session, json!({"command":"redo"}));
    assert_eq!(redo["ok"], true);
    assert_eq!(redo["document"], second["document"]);
    assert_eq!(redo["components"], second["components"]);
    assert_eq!(redo["component_steps"], second["component_steps"]);

    rejected_without_mutation(&mut session, json!({"command":"reset","unexpected":{}}));
    rejected_without_mutation(
        &mut session,
        json!({"command":"rebuild","document":initial,"unexpected":[]}),
    );
    let reset = request(&mut session, json!({"command":"reset"}));
    assert_eq!(reset["ok"], true);
    assert_eq!(reset["reset"], true);
    assert!(session.accepted_document().is_none());
    assert!(session.accepted_shape().is_none());
    assert_eq!((session.undo_count(), session.redo_count()), (0, 0));
    assert_eq!(
        rebuild(&mut session, &edited)["component_steps"],
        second["component_steps"]
    );
}
