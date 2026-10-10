use hagane::*;
use serde_json::{json, Value};
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"cut","operations":[{"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"bore","input":"stock","radius":4.},{"kind":"plane_split","id":"cut","input":"bore","origin":[0.,0.,12.],"normal":[-0.2,-0.1,1.]}]})
}
fn command(session: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &value.to_string()).unwrap(),
    )
    .unwrap()
}
#[test]
fn actual_annular_children_and_atomic_unsupported_queries() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let report = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(report["ok"], true);
    assert_eq!(report["components"].as_array().unwrap().len(), 2);
    for body in report["components"].as_array().unwrap() {
        assert_eq!(body["component_kind"], "bored_oblique_frustum");
        assert_eq!(body["mass_properties"]["ok"], true);
        assert!((body["volume"].as_f64().unwrap() - 1536. * std::f64::consts::PI).abs() < 1e-8);
        assert_eq!(body["brep"]["vertices"].as_array().unwrap().len(), 16);
        assert_eq!(body["brep"]["edges"], 24);
        assert_eq!(body["brep"]["faces"], 10);
        let loops = body["brep"]["wire_edges"].as_array().unwrap();
        assert_eq!(loops[0].as_array().unwrap().len(), 2);
        assert_eq!(loops[1].as_array().unwrap().len(), 2);
        assert!(body["step"].as_str().unwrap().contains("CLOSED_SHELL"));
    }
    let children = report["components"].as_array().unwrap();
    for (i, body) in children.iter().enumerate() {
        let expected = if i == 0 {
            [2. / 3., 1. / 3., 73. / 12.]
        } else {
            [-2. / 3., -1. / 3., 215. / 12.]
        };
        for (axis, value) in expected.iter().enumerate() {
            assert!(
                (body["mass_properties"]["centroid"][axis].as_f64().unwrap() - value).abs() < 1e-10
            );
        }
    }
    let classified = command(
        &mut session,
        json!({"command":"query","component":0,"point":[6.,0.,4.]}),
    );
    assert_eq!(classified["query"]["location"], "inside");
    for query in [
        json!({"command":"segment","component":0,"start":[-20.,0.,6.],"end":[20.,0.,6.]}),
        json!({"command":"line","component":0,"origin":[0.,0.,6.],"direction":[1.,0.,0.]}),
        json!({"command":"ray","component":0,"origin":[0.,0.,6.],"direction":[1.,0.,0.]}),
    ] {
        let rejected = command(&mut session, query);
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["diagnostic"]["category"], "unsupported");
        assert_eq!(rejected["accepted_document"], doc);
    }
    let again = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(
        again["cache"],
        json!({"evaluated_nodes":0,"reused_nodes":3})
    );
    assert_eq!(again["component_steps"], report["component_steps"]);
}
#[test]
fn selected_child_cannot_be_cut_again_and_preserves_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    doc["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"select","id":"one","input":"cut","component":0}));
    doc["output"] = json!("one");
    let accepted = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(accepted["ok"], true);
    let mut invalid = doc.clone();
    invalid["operations"].as_array_mut().unwrap().push(json!({"kind":"plane_split","id":"again","input":"one","origin":[0.,0.,6.],"normal":[0.,0.,1.]}));
    invalid["output"] = json!("again");
    let rejected = command(
        &mut session,
        json!({"command":"rebuild","document":invalid}),
    );
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["accepted_document"], doc);
}
#[test]
fn finite_retained_cap_queries_preserve_cache_and_exports() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let accepted = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(accepted["ok"], true);
    for (point, lower, upper) in [
        ([6., 0., 4.], "inside", "outside"),
        ([6., 0., 20.], "outside", "inside"),
        ([0., 0., 6.], "outside", "outside"),
        ([6., 0., 13.2], "boundary", "boundary"),
        ([6., 0., 0.], "boundary", "outside"),
        ([6., 0., 24.], "outside", "boundary"),
    ] {
        for (index, expected) in [(0, lower), (1, upper)] {
            let result = command(
                &mut session,
                json!({"command":"query","component":index,"point":point}),
            );
            assert_eq!(result["ok"], true);
            assert_eq!(result["query"]["location"], expected);
            assert_eq!(result["query"]["mesh_free"], true);
        }
    }
    let rejected = command(
        &mut session,
        json!({"command":"query","component":0,"point":[1e100,0.,0.]}),
    );
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["accepted_document"], doc);
    let repeat = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(
        repeat["cache"],
        json!({"evaluated_nodes":0,"reused_nodes":3})
    );
    assert_eq!(repeat["components"], accepted["components"]);
    assert_eq!(repeat["component_steps"], accepted["component_steps"]);
    assert_eq!(repeat["history"], accepted["history"]);
}
