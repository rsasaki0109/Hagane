use hagane::*;
use serde_json::{json, Value};
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"middle","operations":[{"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"plane_split","id":"first","input":"stock","origin":[0.,0.,12.],"normal":[0.2,0.,1.]},{"kind":"select","id":"lower","input":"first","component":0},{"kind":"interior_plane_split","id":"second","input":"lower","origin":[0.,0.,6.],"normal":[-0.1,0.1,1.]},{"kind":"select","id":"middle","input":"second","component":1}]})
}
fn command(s: &mut NurbsFrustumWorkflowSession, c: Value) -> Value {
    serde_json::from_str(&nurbs_frustum_workflow_session_command_json(s, &c.to_string()).unwrap())
        .unwrap()
}
#[test]
fn retained_bore_and_explicit_enlargement_match_fresh_replay_and_preserve_failures() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    let stock = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(stock["ok"], true);
    doc["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"coaxial_bore","id":"bore","input":"middle","radius":2.}));
    doc["output"] = json!("bore");
    let bore = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(bore["ok"], true, "{bore}");
    assert_eq!(bore["cache"]["evaluated_nodes"], 1);
    assert_eq!(bore["cache"]["reused_nodes"], 5);
    doc["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"enlarge_coaxial_bore","id":"wide","input":"bore","radius":4.}));
    doc["output"] = json!("wide");
    let wide = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(wide["ok"], true, "{wide}");
    let body = &wide["components"][0];
    assert!((body["volume"].as_f64().unwrap() - 768. * std::f64::consts::PI).abs() < 1e-8);
    assert_eq!(body["mass_properties"]["ok"], true);
    assert_eq!(body["hole_radius"], 4.);
    assert_eq!(body["component_kind"], "bored_plane_bounded_frustum");
    let mut fresh = NurbsFrustumWorkflowSession::new();
    let replay = command(&mut fresh, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], wide["components"]);
    assert_eq!(replay["component_steps"], wide["component_steps"]);
    for radius in [0., 1., 2., 2. + 0.5e-6, 12.] {
        let mut bad = doc.clone();
        bad["operations"][6]["radius"] = json!(radius);
        let rejected = command(&mut session, json!({"command":"rebuild","document":bad}));
        assert_eq!(rejected["ok"], false, "{rejected}");
        assert_eq!(rejected["accepted_document"], doc);
    }
    for operation in [
        json!({"kind":"enlarge_coaxial_bore","id":"bad","input":"stock","radius":5.}),
        json!({"kind":"enlarge_coaxial_bore","id":"bad","input":"middle","radius":5.}),
        json!({"kind":"enlarge_coaxial_bore","id":"bad","input":"second","radius":5.}),
        json!({"kind":"coaxial_bore","id":"bad","input":"wide","radius":5.}),
    ] {
        let mut bad = doc.clone();
        bad["operations"].as_array_mut().unwrap().push(operation);
        bad["output"] = json!("bad");
        let rejected = command(&mut session, json!({"command":"rebuild","document":bad}));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["accepted_document"], doc);
    }
    for (point, expected) in [
        ([0., 0., 8.], "outside"),
        ([3., 0., 8.], "outside"),
        ([6., 0., 8.], "inside"),
    ] {
        let result = command(
            &mut session,
            json!({"command":"query","component":0,"point":point}),
        );
        assert_eq!(result["query"]["location"], expected);
    }
    let unchanged = command(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(unchanged["cache"]["evaluated_nodes"], 0);
    assert_eq!(unchanged["history"], wide["history"]);
    assert_eq!(unchanged["component_steps"], wide["component_steps"]);
    assert_eq!(
        command(&mut session, json!({"command":"undo"}))["components"],
        bore["components"]
    );
    assert_eq!(
        command(&mut session, json!({"command":"redo"}))["components"],
        wide["components"]
    );
}
