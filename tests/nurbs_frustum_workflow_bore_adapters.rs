use hagane::*;
use serde_json::{json, Value};
fn command(s: &mut NurbsFrustumWorkflowSession, c: Value) -> Value {
    serde_json::from_str(&nurbs_frustum_workflow_session_command_json(s, &c.to_string()).unwrap())
        .unwrap()
}
fn document(level: usize, initially_bored: bool) -> Value {
    let mut ops = vec![
        json!({"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}),
    ];
    let mut input = "stock";
    if initially_bored {
        ops.push(json!({"kind":"coaxial_bore","id":"initial","input":input,"radius":2.}));
        input = "initial";
    }
    if level > 0 {
        ops.push(json!({"kind":"plane_split","id":"first","input":input,"origin":[0.,0.,12.],"normal":[0.2,0.,1.]}));
        ops.push(json!({"kind":"select","id":"lower","input":"first","component":0}));
        input = "lower";
    }
    if level > 1 {
        ops.push(json!({"kind":"plane_split","id":"second","input":input,"origin":[0.,0.,6.],"normal":[0.2,0.,1.]}));
        ops.push(json!({"kind":"select","id":"middle","input":"second","component":1}));
        input = "middle";
    }
    if !initially_bored {
        ops.push(json!({"kind":"coaxial_bore","id":"initial","input":input,"radius":2.}));
        input = "initial";
    }
    ops.push(json!({"kind":"enlarge_coaxial_bore","id":"wide","input":input,"radius":4.}));
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"wide","operations":ops})
}
#[test]
fn certified_old_retained_bore_adapters_preserve_actual_metrics_cache_and_recovery() {
    for level in 0..3 {
        for initially_bored in [false, true] {
            let doc = document(level, initially_bored);
            let mut s = NurbsFrustumWorkflowSession::new();
            let mut prior = doc.clone();
            let input = prior["operations"].as_array_mut().unwrap().pop().unwrap()["input"].clone();
            prior["output"] = input;
            let initial = command(&mut s, json!({"command":"rebuild","document":prior}));
            assert_eq!(initial["ok"], true);
            let result = command(&mut s, json!({"command":"rebuild","document":doc}));
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(result["cache"]["evaluated_nodes"], 1);
            let body = &result["components"][0];
            let h = [24., 12., 6.][level];
            let expected = 128. * std::f64::consts::PI * h;
            assert!((body["volume"].as_f64().unwrap() - expected).abs() < 1e-8);
            assert_eq!(body["mass_properties"]["ok"], true);
            assert_eq!(body["hole_radius"], 4.);
            assert_eq!(
                body["component_kind"],
                if level == 0 {
                    "bored_frustum"
                } else {
                    "bored_plane_bounded_frustum"
                }
            );
            let mut fresh = NurbsFrustumWorkflowSession::new();
            let replay = command(&mut fresh, json!({"command":"rebuild","document":doc}));
            assert_eq!(result["components"], replay["components"]);
            assert_eq!(result["component_steps"], replay["component_steps"]);
            for radius in [0., 1., 2., 2. + 0.5e-6, 12.] {
                let mut bad = doc.clone();
                bad["operations"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["radius"] = json!(radius);
                let rejected = command(&mut s, json!({"command":"rebuild","document":bad}));
                assert_eq!(rejected["ok"], false);
                assert_eq!(rejected["accepted_document"], doc);
            }
            let same = command(&mut s, json!({"command":"rebuild","document":doc}));
            assert_eq!(same["cache"]["evaluated_nodes"], 0);
            assert_eq!(same["history"], result["history"]);
            assert_eq!(same["component_steps"], result["component_steps"]);
            assert_eq!(
                command(&mut s, json!({"command":"undo"}))["components"],
                initial["components"]
            );
            assert_eq!(
                command(&mut s, json!({"command":"redo"}))["components"],
                result["components"]
            );
            let z = [12., 6., 9.][level];
            assert_eq!(
                command(
                    &mut s,
                    json!({"command":"query","component":0,"point":[0.,0.,z]})
                )["query"]["location"],
                "outside"
            );
        }
    }
}
