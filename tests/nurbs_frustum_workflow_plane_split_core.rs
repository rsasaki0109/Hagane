use hagane::*;
use std::sync::Arc;
fn document() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"cut","display_chord_tolerance":0.5,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}]})).unwrap()
}
#[test]
fn typed_selection_keeps_actual_oblique_arc_and_refuses_repartition() {
    let mut d = document();
    let p = d.geometry_tol().unwrap();
    let mut session = NurbsFrustumWorkflowSession::new();
    let first = session.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    let lower = first.shape.components()[0].oblique().unwrap().clone();
    assert!(first.shape.components()[0].canonical().is_none());
    assert_eq!(
        first.shape.components()[0]
            .classify_point(Point3::new(0., 0., 6.), p)
            .unwrap(),
        PointLocation::Inside
    );
    let source = session.prefix_snapshot(0).unwrap().components()[0]
        .canonical()
        .unwrap();
    assert!(
        (first
            .shape
            .components()
            .iter()
            .map(|v| v.volume(p).unwrap())
            .sum::<f64>()
            - source.volume(p).unwrap())
        .abs()
            < 1e-8
    );
    d.operations.push(NurbsFrustumWorkflowOperation::Select {
        id: "pick".into(),
        input: "cut".into(),
        component: 0,
    });
    d.output = "pick".into();
    let picked = session.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    assert_eq!(picked.stats.reused_nodes, 2);
    assert!(Arc::ptr_eq(
        &lower,
        picked.shape.components()[0].oblique().unwrap()
    ));
    let accepted = session.accepted_shape().unwrap().clone();
    d.operations.push(NurbsFrustumWorkflowOperation::Partition {
        id: "again".into(),
        input: "pick".into(),
        cuts: vec![1.],
    });
    d.output = "again".into();
    let error = session.rebuild_with(&d, |_, _| Ok(())).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("again"));
    assert_eq!(error.category, "unsupported");
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
}
#[test]
fn later_invalid_plane_is_checked_before_prefix_reuse() {
    let d = document();
    let mut session = NurbsFrustumWorkflowSession::new();
    session.rebuild_with(&d, |_, _| Ok(())).unwrap();
    let accepted = session.accepted_shape().unwrap().clone();
    let mut bad = d.clone();
    bad.operations
        .push(NurbsFrustumWorkflowOperation::PlaneSplit {
            id: "bad".into(),
            input: "stock".into(),
            origin: [0., 0., 12.],
            normal: [0.; 3],
        });
    let error = session.rebuild_with(&bad, |_, _| Ok(())).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("bad"));
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
}
