use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document(radii: [f64; 2]) -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"cut","operations":[
        {"kind":"frustum","id":"stock","radii":radii,"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},
        {"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]},
        {"kind":"select","id":"selected","input":"cut","component":0}
    ]})
}
fn rebuild(session: &mut NurbsFrustumWorkflowSession, doc: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(
            session,
            &json!({"command":"rebuild","document":doc}).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn workflow_reports_both_retained_oblique_bodies_exactly_and_select_keeps_actual_arc() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document([16., 8.]);
    let report = rebuild(&mut session, doc.clone());
    assert_eq!(report["ok"], true);
    assert_eq!(report["components"].as_array().unwrap().len(), 2);
    let direct: Value = serde_json::from_str(
        &nurbs_frustum_plane_split_demo_json(&[
            16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1, 0., 0., 12., 0.1, 0., 1., 0.,
        ])
        .unwrap(),
    )
    .unwrap();
    let shape = session.accepted_shape().unwrap().clone();
    let cut = shape.components()[0].oblique().unwrap().clone();
    assert!(shape.components().iter().all(|c| c.oblique().is_some()));
    for (i, component) in report["components"].as_array().unwrap().iter().enumerate() {
        assert_eq!(component["component_kind"], "oblique_frustum");
        assert_eq!(component["placement_reference"], "source frame");
        assert!(component.get("radii").is_none());
        assert!(component.get("height").is_none());
        assert!(component.get("centroid").is_none());
        assert!(component.get("inertia").is_none());
        assert_eq!(component["mass_properties"]["ok"], true);
        let mut actual = component.clone();
        for key in ["component_kind", "placement_reference", "placement"] {
            actual.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(actual, direct["parts"][i]);
        assert_eq!(report["component_steps"][i], direct["parts"][i]["step"]);
    }
    let source_volume = direct["source_volume"].as_f64().unwrap();
    assert!(
        (report["output"]["total_volume"].as_f64().unwrap() - source_volume).abs()
            < 1e-10 * source_volume
    );
    doc["output"] = json!("selected");
    let selected = rebuild(&mut session, doc);
    assert_eq!(selected["ok"], true);
    assert_eq!(selected["components"].as_array().unwrap().len(), 1);
    assert_eq!(selected["components"][0], report["components"][0]);
    assert_eq!(
        selected["output"]["total_volume"],
        report["components"][0]["volume"]
    );
    assert_eq!(selected["cache"]["reused_nodes"], 3);
    assert!(Arc::ptr_eq(
        &cut,
        session.accepted_shape().unwrap().components()[0]
            .oblique()
            .unwrap()
    ));
}
#[test]
fn optional_mass_refusal_preserves_oblique_geometry_and_failed_display_remains_atomic() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document([16., 16. + 1e-8]);
    let report = rebuild(&mut session, doc.clone());
    assert_eq!(report["ok"], true);
    for component in report["components"].as_array().unwrap() {
        assert_eq!(component["mass_properties"]["ok"], false);
        assert!(!component["mass_properties"]["error"]
            .as_str()
            .unwrap()
            .is_empty());
        assert!(component["volume"].as_f64().unwrap() > 0.);
        assert!(!component["mesh"]["triangles"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(component["step"]
            .as_str()
            .unwrap()
            .contains("MANIFOLD_SOLID_BREP"));
    }
    let accepted = session.accepted_shape().unwrap().clone();
    let prefixes: Vec<_> = (0..3)
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    let history = (session.undo_count(), session.redo_count());
    let mut bad = doc.clone();
    bad["display_chord_tolerance"] = json!(1e-12);
    assert_eq!(rebuild(&mut session, bad)["ok"], false);
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    for (i, prefix) in prefixes.iter().enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
    }
    assert_eq!((session.undo_count(), session.redo_count()), history);
    assert_eq!(
        rebuild(&mut session, doc)["components"],
        report["components"]
    );
}
#[test]
fn canonical_component_reports_keep_existing_flat_mass_and_geometry_fields() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document([16., 8.]);
    doc["output"] = json!("stock");
    doc["operations"] = json!([doc["operations"][0]]);
    let report = rebuild(&mut session, doc);
    assert_eq!(report["ok"], true);
    let data = &report["components"][0];
    assert!(data.get("component_kind").is_none());
    assert!(data.get("mass_properties").is_none());
    let body = session.accepted_shape().unwrap().components()[0]
        .canonical()
        .unwrap();
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let mass = body.mass_properties(policy).unwrap();
    assert_eq!(data["radii"], json!([16., 8.]));
    assert_eq!(data["height"], 24.);
    assert_eq!(
        data["centroid"],
        json!([mass.centroid.x, mass.centroid.y, mass.centroid.z])
    );
    assert_eq!(
        data["inertia"],
        json!(body.inertia_properties(policy).unwrap().inertia)
    );
    assert_eq!(data["step"], body.export_step_mm(policy).unwrap());
}
