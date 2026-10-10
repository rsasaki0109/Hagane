use hagane::*;
use std::sync::Arc;
fn doc() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"pick","display_chord_tolerance":0.5,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-5.,8.],"axes":[[0.,1.,0.],[0.,0.,1.],[1.,0.,0.]]},{"kind":"partition","id":"parts","input":"stock","cuts":[6.,12.]},{"kind":"select","id":"pick","input":"parts","component":1}]})).unwrap()
}
#[test]
fn real_prefix_sharing_and_atomic_callback_history() {
    let d = doc();
    let mut s = NurbsFrustumWorkflowSession::new();
    let first = s
        .rebuild_with(&d, |shape, doc| {
            for b in shape.components() {
                b.export_step_mm(doc.geometry_tol().unwrap())?;
            }
            Ok(())
        })
        .unwrap()
        .0;
    assert_eq!(first.stats.evaluated_nodes, 3);
    let stock = s.prefix_snapshot(0).unwrap().clone();
    let selected = first.shape.components()[0].clone();
    let fresh = d.rebuild().unwrap();
    assert_eq!(
        selected.export_step_mm(d.geometry_tol().unwrap()).unwrap(),
        fresh.components()[0]
            .export_step_mm(d.geometry_tol().unwrap())
            .unwrap()
    );
    let mut changed = d.clone();
    changed.output = "parts".into();
    let r = s.rebuild_with(&changed, |_, _| Ok(())).unwrap().0;
    assert_eq!(r.stats.reused_nodes, 3);
    assert_eq!(r.shape.components().len(), 3);
    assert!(Arc::ptr_eq(&stock, s.prefix_snapshot(0).unwrap()));
    let history = s.undo_count();
    let accepted = s.accepted_shape().unwrap().clone();
    let mut bad = changed.clone();
    bad.display_chord_tolerance = 0.25;
    assert!(s
        .rebuild_with(&bad, |_, _| Err::<(), _>(Error::Unsupported(
            "display refused"
        )))
        .is_err());
    assert_eq!(s.accepted_document(), Some(&changed));
    assert_eq!(s.undo_count(), history);
    assert!(Arc::ptr_eq(&accepted, s.accepted_shape().unwrap()));
    s.undo_with(|_, _| Ok(())).unwrap();
    assert_eq!(s.accepted_document(), Some(&d));
    s.redo_with(|_, _| Ok(())).unwrap();
    assert_eq!(s.accepted_document(), Some(&changed));
}
#[test]
fn actual_step_stock_and_unambiguous_branch_policy() {
    let d = doc();
    let p = d.geometry_tol().unwrap();
    let shape = d.rebuild().unwrap();
    let b = &shape.components()[0];
    let step = b.export_step_mm(p).unwrap();
    let mut input = d.clone();
    input.operations = vec![NurbsFrustumWorkflowOperation::StepStock {
        id: "actual".into(),
        step: step.clone(),
    }];
    input.output = "actual".into();
    let got = input.rebuild().unwrap();
    assert_eq!(got.components()[0].export_step_mm(p).unwrap(), step);
    let mut ambiguous = d.clone();
    ambiguous
        .operations
        .push(NurbsFrustumWorkflowOperation::Partition {
            id: "bad".into(),
            input: "parts".into(),
            cuts: vec![1.],
        });
    ambiguous.output = "bad".into();
    assert_eq!(
        ambiguous.rebuild().unwrap_err().code,
        "ambiguous_components"
    );
    let mut invalid = d.clone();
    if let NurbsFrustumWorkflowOperation::Frustum { axes, .. } = &mut invalid.operations[0] {
        axes[0][0] = 0.1;
    }
    assert!(invalid.rebuild().is_err());
    let mut json = serde_json::to_value(&d).unwrap();
    json["operations"][0]["fake"] = serde_json::json!(true);
    assert!(serde_json::from_value::<NurbsFrustumWorkflowDocument>(json).is_err());
}

#[test]
fn default_acceptance_rejects_aggregate_display_without_committing() {
    let mut d = doc();
    d.operations.truncate(2);
    if let NurbsFrustumWorkflowOperation::Partition { cuts, .. } = &mut d.operations[1] {
        *cuts = (1..17).map(|i| 24. * i as f64 / 17.).collect();
    }
    d.output = "parts".into();
    d.display_chord_tolerance = 0.5;
    let mut session = NurbsFrustumWorkflowSession::new();
    session.rebuild(&d).unwrap();
    let accepted = session.accepted_shape().unwrap().clone();
    let history = session.undo_count();
    let mut tight = d.clone();
    tight.display_chord_tolerance = 0.01;
    let fresh = tight.rebuild().unwrap();
    let policy = tight.geometry_tol().unwrap();
    let mut triangles = 0;
    for body in fresh.components() {
        triangles += body
            .tessellate(tight.display_chord_tolerance, policy)
            .unwrap()
            .triangles
            .len();
    }
    assert!(triangles > 131072);
    let error = session.rebuild(&tight).unwrap_err();
    assert!(error.message.contains("aggregate display triangle budget"));
    assert_eq!(session.accepted_document(), Some(&d));
    assert_eq!(session.undo_count(), history);
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
}
