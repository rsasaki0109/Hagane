use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn value() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"hole","display_chord_tolerance":0.2,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-3.,5.],"axes":[[0.,1.,0.],[0.,0.,1.],[1.,0.,0.]]},{"kind":"coaxial_bore","id":"hole","input":"stock","radius":4.}]})
}
fn doc(v: Value) -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(v).unwrap()
}
#[test]
fn actual_direct_body_prefix_selection_and_radius_cache_reuse() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut document = doc(value());
    let accepted = session.rebuild(&document).unwrap();
    let stock = session.prefix_snapshot(0).unwrap().clone();
    let source = stock.components()[0].canonical().unwrap();
    let expected = source.bore_coaxial(4., policy()).unwrap();
    let hole = accepted.shape.components()[0].bored().unwrap();
    assert_eq!(
        format!("{:?}", hole.solid()),
        format!("{:?}", expected.solid())
    );
    assert_eq!(
        format!("{:?}", &hole.solid().edges[..12]),
        format!("{:?}", source.solid().edges)
    );
    assert_eq!(
        hole.mass_properties(policy()).unwrap().centroid,
        expected.mass_properties(policy()).unwrap().centroid
    );
    document
        .operations
        .push(NurbsFrustumWorkflowOperation::Select {
            id: "selected".into(),
            input: "hole".into(),
            component: 0,
        });
    document.output = "selected".into();
    let selected = session.rebuild(&document).unwrap();
    assert!(Arc::ptr_eq(
        selected.shape.components()[0].bored().unwrap(),
        hole
    ));
    assert_eq!(selected.stats.reused_nodes, 2);
    let mut edited = value();
    edited["operations"][1]["radius"] = json!(5.);
    let changed = session.rebuild(&doc(edited)).unwrap();
    assert_eq!(changed.stats.reused_nodes, 1);
    assert_eq!(changed.stats.evaluated_nodes, 1);
    assert!(Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &stock));
    let fresh = doc({
        let mut v = value();
        v["operations"][1]["radius"] = json!(5.);
        v
    })
    .rebuild()
    .unwrap();
    assert_eq!(
        format!("{:?}", changed.shape.components()[0].solid()),
        format!("{:?}", fresh.components()[0].solid())
    );
}
#[test]
fn terminal_body_static_unused_errors_and_display_failure_are_atomic() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let good = doc(value());
    let accepted = session.rebuild(&good).unwrap();
    let before = session.accepted_document().unwrap().clone();
    let step = accepted.shape.components()[0]
        .export_step_mm(policy())
        .unwrap();
    let mut candidates = Vec::new();
    for op in [
        json!({"kind":"coaxial_bore","id":"later","input":"hole","radius":2.}),
        json!({"kind":"coaxial_bore","id":"later","input":"hole","radius":3.}),
    ] {
        let mut v = value();
        v["operations"].as_array_mut().unwrap().push(op);
        v["output"] = json!("later");
        candidates.push(v);
    }
    let mut ambiguous = value();
    ambiguous["operations"].as_array_mut().unwrap().extend([
        json!({"kind":"partition","id":"parts","input":"hole","cuts":[12.]}),
        json!({"kind":"partition","id":"later","input":"parts","cuts":[6.]}),
    ]);
    ambiguous["output"] = json!("later");
    candidates.push(ambiguous);
    let mut unused = value();
    unused["output"] = json!("stock");
    unused["operations"][1]["radius"] = json!(-1.);
    candidates.push(unused);
    let mut tiny = value();
    tiny["display_chord_tolerance"] = json!(1e-14);
    candidates.push(tiny);
    for v in candidates {
        assert!(session.rebuild(&doc(v)).is_err());
        assert!(Arc::ptr_eq(
            session.accepted_shape().unwrap(),
            &accepted.shape
        ));
        assert_eq!(session.accepted_document(), Some(&before));
        assert_eq!(
            session.accepted_shape().unwrap().components()[0]
                .export_step_mm(policy())
                .unwrap(),
            step
        );
    }
    assert!(session.undo().is_err());
    let recovered = session.rebuild(&good).unwrap();
    assert_eq!(recovered.stats.reused_nodes, 2);
    assert_eq!(recovered.stats.evaluated_nodes, 0);
}
