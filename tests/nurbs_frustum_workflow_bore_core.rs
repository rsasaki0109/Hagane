use hagane::*;
use std::sync::Arc;
fn doc() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"hole","display_chord_tolerance":0.5,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"hole","input":"stock","radius":4.}]})).unwrap()
}
#[test]
fn radius_edit_reuses_stock_and_select_keeps_actual_bored_arc() {
    let mut d = doc();
    let p = d.geometry_tol().unwrap();
    let mut s = NurbsFrustumWorkflowSession::new();
    let first = s.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    let stock = s.prefix_snapshot(0).unwrap().clone();
    let source = stock.components()[0].canonical().unwrap();
    let direct = source.bore_coaxial(4., p).unwrap();
    assert_eq!(
        first.shape.components()[0].export_step_mm(p).unwrap(),
        direct.export_step_mm(p).unwrap()
    );
    let actual = first.shape.components()[0].bored().unwrap().clone();
    d.operations.push(NurbsFrustumWorkflowOperation::Select {
        id: "pick".into(),
        input: "hole".into(),
        component: 0,
    });
    d.output = "pick".into();
    let picked = s.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    assert!(Arc::ptr_eq(
        &actual,
        picked.shape.components()[0].bored().unwrap()
    ));
    if let NurbsFrustumWorkflowOperation::CoaxialBore { radius, .. } = &mut d.operations[1] {
        *radius = 3.;
    }
    let edited = s.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    assert_eq!(
        (edited.stats.reused_nodes, edited.stats.evaluated_nodes),
        (1, 2)
    );
    assert!(Arc::ptr_eq(&stock, s.prefix_snapshot(0).unwrap()));
    assert_eq!(edited.shape.components()[0].bored().unwrap().radius(), 3.);
}
#[test]
fn unsupported_suffix_and_invalid_unused_radius_leave_history_unchanged() {
    let d = doc();
    let mut s = NurbsFrustumWorkflowSession::new();
    s.rebuild_with(&d, |_, _| Ok(())).unwrap();
    let accepted = s.accepted_shape().unwrap().clone();
    let history = s.undo_count();
    for operation in [
        NurbsFrustumWorkflowOperation::CoaxialBore {
            id: "bad".into(),
            input: "hole".into(),
            radius: 2.,
        },
        NurbsFrustumWorkflowOperation::CoaxialBore {
            id: "bad".into(),
            input: "hole".into(),
            radius: 3.,
        },
    ] {
        let mut bad = d.clone();
        bad.operations.push(operation);
        bad.output = "bad".into();
        let e = s.rebuild_with(&bad, |_, _| Ok(())).unwrap_err();
        assert_eq!(e.category, "unsupported");
        assert_eq!(e.operation_id.as_deref(), Some("bad"));
        assert!(Arc::ptr_eq(&accepted, s.accepted_shape().unwrap()));
        assert_eq!(s.undo_count(), history);
    }
    let mut bad = d.clone();
    bad.operations
        .push(NurbsFrustumWorkflowOperation::CoaxialBore {
            id: "unused".into(),
            input: "stock".into(),
            radius: 0.,
        });
    let e = s.rebuild_with(&bad, |_, _| Ok(())).unwrap_err();
    assert_eq!(e.category, "invalid_input");
    assert_eq!(e.operation_id.as_deref(), Some("unused"));
    assert!(Arc::ptr_eq(&accepted, s.accepted_shape().unwrap()));
}
