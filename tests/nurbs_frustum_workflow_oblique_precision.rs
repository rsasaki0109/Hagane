use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"split","display_chord_tolerance":0.2,"operations":[{"kind":"frustum","id":"stock","radii":[12.,12.0000000001],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"plane_split","id":"split","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}]})
}
fn command(session: &mut NurbsFrustumWorkflowSession, document: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(
            session,
            &json!({"command":"rebuild","document":document}).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn unresolved_optional_moments_do_not_discard_actual_shape() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let report = command(&mut session, document());
    assert_eq!(report["ok"], true, "{report}");
    let accepted = session.accepted_shape().unwrap();
    assert_eq!(accepted.components().len(), 2);
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for component in accepted.components() {
        assert!(component.volume(policy).unwrap() > 0.);
        assert!(matches!(
            component.mass_properties(policy),
            Err(Error::Unsupported(_))
        ));
        assert!(!component.export_step_mm(policy).unwrap().is_empty());
    }
    assert_eq!(report["component_steps"].as_array().unwrap().len(), 2);
}
#[test]
fn display_and_unused_geometry_rejection_preserve_actual_cache_and_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let good = document();
    assert_eq!(command(&mut session, good.clone())["ok"], true);
    let accepted = session.accepted_shape().unwrap().clone();
    let accepted_document = session.accepted_document().unwrap().clone();
    let mut failures = Vec::new();
    let mut display = good.clone();
    display["display_chord_tolerance"] = json!(1e-14);
    failures.push(display);
    let mut distant = good.clone();
    distant["operations"][1]["origin"] = json!([1e16, 0., 12.]);
    failures.push(distant);
    let mut unused = good.clone();
    unused["output"] = json!("stock");
    unused["operations"][1]["normal"] = json!([0., 0., 0.]);
    failures.push(unused);
    for candidate in failures {
        let report = command(&mut session, candidate);
        assert_eq!(report["ok"], false, "{report}");
        assert!(Arc::ptr_eq(session.accepted_shape().unwrap(), &accepted));
        assert_eq!(session.accepted_document(), Some(&accepted_document));
    }
    let recovery = command(&mut session, good);
    assert_eq!(recovery["ok"], true);
    assert_eq!(recovery["cache"]["evaluated_nodes"], 0);
    assert_eq!(recovery["history"]["can_undo"], false);
}
