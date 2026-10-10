use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn call(session: &mut NurbsFrustumWorkflowSession, command: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &command.to_string()).unwrap(),
    )
    .unwrap()
}
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"cut","operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]},{"kind":"select","id":"selected","input":"cut","component":1}]})
}
#[test]
fn reports_actual_child_hits_and_uv_without_rebuilding_and_select_preserves_child_identity() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    assert_eq!(accepted["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for component in 0..2 {
        let child = shape.components()[component].oblique().unwrap();
        let start = Vec3::new(1., 2., -4.);
        let end = Vec3::new(1., 2., 28.);
        let direct = child.intersect_segment(start, end, policy).unwrap();
        let report = call(
            &mut session,
            json!({"command":"segment","component":component,"start":[1.,2.,-4.],"end":[1.,2.,28.]}),
        );
        assert_eq!(report["ok"], true, "{report}");
        let hits:Vec<_>=direct.hits.iter().map(|h|json!({"parameter":h.parameter,"point":[h.point.x,h.point.y,h.point.z],"faces":h.faces.iter().map(|f|json!({"face_id":f.face_id,"uv":f.uv})).collect::<Vec<_>>()})).collect();
        assert_eq!(report["segment"]["hits"], json!(hits));
        assert_eq!(
            report["segment"]["material_intervals"],
            json!(direct.material_interval.into_iter().collect::<Vec<_>>())
        );
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert!(report.get("components").is_none());
        let z = if component == 0 { 6. } else { 18. };
        let side = call(
            &mut session,
            json!({"command":"segment","component":component,"start":[-24.,3.,z],"end":[24.,3.,z]}),
        );
        assert_eq!(side["ok"], true, "{side}");
        assert!(side["segment"]["hits"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|h| h["faces"].as_array().unwrap())
            .any(|f| f["face_id"].as_u64().unwrap() >= 2
                && f["uv"][1].as_f64().unwrap() > 0.
                && f["uv"][1].as_f64().unwrap() < 1.));
    }
    let upper = shape.components()[1].oblique().unwrap().clone();
    doc["output"] = json!("selected");
    assert_eq!(
        call(&mut session, json!({"command":"rebuild","document":doc}))["ok"],
        true
    );
    assert!(Arc::ptr_eq(
        &upper,
        session.accepted_shape().unwrap().components()[0]
            .oblique()
            .unwrap()
    ));
    assert_eq!(
        call(
            &mut session,
            json!({"command":"segment","component":0,"start":[1.,2.,-4.],"end":[1.,2.,28.]})
        )["ok"],
        true
    );
}
#[test]
fn ambiguous_cut_endpoint_rejects_without_mutating_history_geometry_or_exports() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for component in 0..2 {
        let bad = call(
            &mut session,
            json!({"command":"segment","component":component,"start":[0.,0.,12.],"end":[1.,2.,28.]}),
        );
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["diagnostic"]["category"], "unsupported");
        assert_eq!(bad["diagnostic"]["operation_id"], "cut");
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[0.,0.,6.]})
        )["query"]["location"],
        "inside"
    );
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
