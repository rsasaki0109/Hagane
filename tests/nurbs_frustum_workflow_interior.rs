use hagane::*;
use serde_json::{json, Value};
fn document(bored: bool) -> Value {
    let mut ops = vec![
        json!({"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}),
    ];
    let source = if bored {
        ops.push(json!({"kind":"coaxial_bore","id":"bore","input":"stock","radius":4.}));
        "bore"
    } else {
        "stock"
    };
    ops.extend([json!({"kind":"plane_split","id":"first","input":source,"origin":[0.,0.,12.],"normal":[0.2,0.,1.]}),json!({"kind":"select","id":"lower","input":"first","component":0}),json!({"kind":"interior_plane_split","id":"second","input":"lower","origin":[0.,0.,6.],"normal":[-0.1,0.1,1.]}),json!({"kind":"select","id":"middle","input":"second","component":1})]);
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"operations":ops,"output":"middle"})
}
fn command(s: &mut NurbsFrustumWorkflowSession, c: Value) -> Value {
    serde_json::from_str(&nurbs_frustum_workflow_session_command_json(s, &c.to_string()).unwrap())
        .unwrap()
}
#[test]
fn explicit_skew_interior_node_retains_actual_body_and_query_state() {
    for bored in [false, true] {
        let doc = document(bored);
        let mut session = NurbsFrustumWorkflowSession::new();
        let report = command(&mut session, json!({"command":"rebuild","document":doc}));
        assert_eq!(report["ok"], true, "{report}");
        let body = &report["components"][0];
        let area = std::f64::consts::PI * (144. - if bored { 16. } else { 0. });
        assert!((body["volume"].as_f64().unwrap() - 6. * area).abs() < 1e-8);
        assert_eq!(
            body["component_kind"],
            if bored {
                "bored_plane_bounded_frustum"
            } else {
                "plane_bounded_frustum"
            }
        );
        assert_eq!(body["brep"]["closed"], true);
        assert_eq!(body["mass_properties"]["ok"], true);
        let ratio = (144. + if bored { 16. } else { 0. }) / 4.;
        let expected = [-ratio / 20., ratio / 60., 9. + ratio / 600.];
        for (a, x) in expected.iter().enumerate() {
            assert!((body["mass_properties"]["centroid"][a].as_f64().unwrap() - x).abs() < 1e-10);
        }
        for query in [
            json!({"command":"query","component":0,"point":[6.,0.,8.]}),
            json!({"command":"segment","component":0,"start":[6.,0.,-2.],"end":[6.,0.,26.]}),
            json!({"command":"line","component":0,"origin":[6.,0.,-2.],"direction":[0.,0.,2.]}),
            json!({"command":"ray","component":0,"origin":[6.,0.,8.],"direction":[0.,0.,2.]}),
        ] {
            let result = command(&mut session, query);
            assert_eq!(result["ok"], true, "{result}");
        }
        let mut old = doc.clone();
        let count = old["operations"].as_array().unwrap().len();
        old["operations"][count - 2]["kind"] = json!("plane_split");
        let rejected = command(&mut session, json!({"command":"rebuild","document":old}));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["accepted_document"], doc);
        let same = command(&mut session, json!({"command":"rebuild","document":doc}));
        assert_eq!(same["cache"]["evaluated_nodes"], 0);
        assert_eq!(same["history"], report["history"]);
        assert_eq!(same["component_steps"], report["component_steps"]);
    }
}
