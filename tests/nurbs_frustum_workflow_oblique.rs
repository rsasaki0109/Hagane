use hagane::*;
use serde_json::json;
use std::sync::Arc;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn document() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"cut","display_chord_tolerance":0.2,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}]})).unwrap()
}
fn signatures(shape: &NurbsFrustumWorkflowShape) -> Vec<String> {
    shape
        .components()
        .iter()
        .map(|body| body.export_step_mm(policy()).unwrap())
        .collect()
}
#[test]
fn exact_oblique_children_and_selected_arc_match_direct_retained_split() {
    let doc = document();
    let mut session = NurbsFrustumWorkflowSession::new();
    let first = session.rebuild(&doc).unwrap();
    assert_eq!(first.shape.components().len(), 2);
    assert_eq!(first.stats.evaluated_nodes, 2);
    let stock = session.prefix_snapshot(0).unwrap().clone();
    let parent = stock.components()[0].canonical().unwrap();
    assert!(first
        .shape
        .components()
        .iter()
        .all(|body| body.canonical().is_none()));
    let lower = first.shape.components()[0].oblique().unwrap();
    let upper = first.shape.components()[1].oblique().unwrap();
    let direct = parent
        .split_by_plane(lower.section().plane(), policy())
        .unwrap();
    for (actual, expected) in [(lower, &direct.lower), (upper, &direct.upper)] {
        assert_eq!(
            format!("{:?}", actual.solid()),
            format!("{:?}", expected.solid())
        );
        assert_eq!(
            actual.export_step_mm(policy()).unwrap(),
            expected.export_step_mm(policy()).unwrap()
        );
        assert_eq!(
            actual.volume(policy()).unwrap(),
            expected.volume(policy()).unwrap()
        );
    }
    assert!(lower.is_lower());
    assert!(!upper.is_lower());
    assert_eq!(
        signatures(&first.shape),
        signatures(&doc.rebuild().unwrap())
    );
    let mut selected = doc.clone();
    selected
        .operations
        .push(NurbsFrustumWorkflowOperation::Select {
            id: "chosen".into(),
            input: "cut".into(),
            component: 1,
        });
    selected.output = "chosen".into();
    let chosen = session.rebuild(&selected).unwrap();
    assert_eq!(
        (chosen.stats.reused_nodes, chosen.stats.evaluated_nodes),
        (2, 1)
    );
    assert!(Arc::ptr_eq(&stock, session.prefix_snapshot(0).unwrap()));
    assert!(Arc::ptr_eq(
        chosen.shape.components()[0].oblique().unwrap(),
        upper
    ));
    assert!(matches!(
        import_step_nurbs_frustum_cardinal_mm(&signatures(&chosen.shape)[0], policy()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn plane_and_chord_edits_reuse_real_prefixes_with_atomic_undo_redo() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let original = document();
    let first = session.rebuild(&original).unwrap();
    let stock = session.prefix_snapshot(0).unwrap().clone();
    let original_steps = signatures(&first.shape);
    let mut changed = original.clone();
    if let NurbsFrustumWorkflowOperation::PlaneSplit { origin, .. } = &mut changed.operations[1] {
        origin[2] = 13.;
    }
    let second = session.rebuild(&changed).unwrap();
    assert_eq!(
        (second.stats.reused_nodes, second.stats.evaluated_nodes),
        (1, 1)
    );
    assert!(Arc::ptr_eq(&stock, session.prefix_snapshot(0).unwrap()));
    assert_ne!(signatures(&second.shape), original_steps);
    let second_shape = second.shape.clone();
    let mut chord = changed.clone();
    chord.display_chord_tolerance = 0.5;
    let accepted = session.rebuild(&chord).unwrap();
    assert_eq!(
        (accepted.stats.reused_nodes, accepted.stats.evaluated_nodes),
        (2, 0)
    );
    assert!(Arc::ptr_eq(&accepted.shape, &second_shape));
    session.undo().unwrap();
    assert_eq!(session.accepted_document(), Some(&changed));
    let history = (session.undo_count(), session.redo_count());
    let before = session.accepted_shape().unwrap().clone();
    assert!(session
        .redo_with(|_, _| Err::<(), _>(Error::Unsupported("forced export refusal")))
        .is_err());
    assert_eq!(history, (session.undo_count(), session.redo_count()));
    assert!(Arc::ptr_eq(&before, session.accepted_shape().unwrap()));
    session.redo().unwrap();
    assert_eq!(session.accepted_document(), Some(&chord));
    session.undo().unwrap();
    session.undo().unwrap();
    assert_eq!(session.accepted_document(), Some(&original));
    assert_eq!(
        signatures(session.accepted_shape().unwrap()),
        original_steps
    );
    session.redo().unwrap();
    assert_eq!(session.accepted_document(), Some(&changed));
    assert_eq!(
        signatures(session.accepted_shape().unwrap()),
        signatures(&changed.rebuild().unwrap())
    );
}
#[test]
fn invalid_oblique_followups_ambiguous_inputs_and_unused_bad_nodes_preserve_accepted_state() {
    let original = document();
    let mut selected = original.clone();
    selected
        .operations
        .push(NurbsFrustumWorkflowOperation::Select {
            id: "chosen".into(),
            input: "cut".into(),
            component: 0,
        });
    selected.output = "chosen".into();
    let mut session = NurbsFrustumWorkflowSession::new();
    session.rebuild(&original).unwrap();
    session.rebuild(&selected).unwrap();
    session.undo().unwrap();
    let accepted = session.accepted_shape().unwrap().clone();
    let prefix = session.prefix_snapshot(0).unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    assert_eq!(history.1, 1);
    let recut = NurbsFrustumWorkflowOperation::PlaneSplit {
        id: "recut".into(),
        input: "chosen".into(),
        origin: [0., 0., 8.],
        normal: [0.2, 0., 1.],
    };
    let repartition = NurbsFrustumWorkflowOperation::Partition {
        id: "recut".into(),
        input: "chosen".into(),
        cuts: vec![1.],
    };
    for (operation, expected_code) in [
        (recut, "geometry_rejected"),
        (repartition, "canonical_required"),
    ] {
        let mut bad = selected.clone();
        bad.operations.push(operation);
        bad.output = "recut".into();
        let error = session.rebuild(&bad).unwrap_err();
        assert_eq!(error.operation_id.as_deref(), Some("recut"));
        assert_eq!(error.code, expected_code);
        assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
        assert!(Arc::ptr_eq(&prefix, session.prefix_snapshot(0).unwrap()));
        assert_eq!(history, (session.undo_count(), session.redo_count()));
    }
    let mut ambiguous = original.clone();
    ambiguous
        .operations
        .push(NurbsFrustumWorkflowOperation::PlaneSplit {
            id: "ambiguous".into(),
            input: "cut".into(),
            origin: [0., 0., 8.],
            normal: [0., 0., 1.],
        });
    ambiguous.output = "ambiguous".into();
    assert_eq!(
        session.rebuild(&ambiguous).unwrap_err().code,
        "ambiguous_components"
    );
    let mut unused = original.clone();
    unused
        .operations
        .push(NurbsFrustumWorkflowOperation::PlaneSplit {
            id: "unused-bad".into(),
            input: "stock".into(),
            origin: [0., 0., 0.],
            normal: [0., 0., 0.],
        });
    unused.output = "stock".into();
    assert_eq!(
        session
            .rebuild(&unused)
            .unwrap_err()
            .operation_id
            .as_deref(),
        Some("unused-bad")
    );
    assert_eq!(session.accepted_document(), Some(&original));
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    assert_eq!(history, (session.undo_count(), session.redo_count()));
    session.redo().unwrap();
    assert_eq!(session.accepted_document(), Some(&selected));
    assert!(session.accepted_shape().unwrap().components()[0]
        .oblique()
        .is_some());
}
