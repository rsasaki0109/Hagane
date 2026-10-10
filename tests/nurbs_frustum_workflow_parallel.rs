use hagane::*;
use serde_json::{json, Value};
fn document(bored: bool) -> Value {
    let mut operations = vec![
        json!({"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}),
    ];
    let source = if bored {
        operations.push(json!({"kind":"coaxial_bore","id":"bore","input":"stock","radius":4.}));
        "bore"
    } else {
        "stock"
    };
    operations.extend([json!({"kind":"plane_split","id":"first","input":source,"origin":[0.,0.,16.],"normal":[-0.2,-0.1,1.]}),json!({"kind":"select","id":"lower","input":"first","component":0}),json!({"kind":"plane_split","id":"second","input":"lower","origin":[0.,0.,8.],"normal":[-0.2,-0.1,1.]})]);
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"second","operations":operations})
}
fn command(session: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &value.to_string()).unwrap(),
    )
    .unwrap()
}
#[test]
fn retained_parallel_children_have_analytic_volumes_and_atomic_guards() {
    for bored in [false, true] {
        let mut session = NurbsFrustumWorkflowSession::new();
        let mut doc = document(bored);
        let first = command(&mut session, json!({"command":"rebuild","document":doc}));
        assert_eq!(first["ok"], true, "{first}");
        let area = std::f64::consts::PI * (144. - if bored { 16. } else { 0. });
        for body in first["components"].as_array().unwrap() {
            assert!((body["volume"].as_f64().unwrap() - 8. * area).abs() < 1e-8);
            assert_eq!(body["mass_properties"]["ok"], true);
            assert!((body["mass_properties"]["volume"].as_f64().unwrap() - 8. * area).abs() < 1e-8);
            assert_eq!(body["mass_properties"]["inertia_units"], "mm^5");
            assert_eq!(body["mass_properties"]["tensor_axes"], "world");
            assert_eq!(body["brep"]["closed"], true);
            assert_eq!(body["genus"], if bored { 1 } else { 0 });
            assert!(body["step"].as_str().unwrap().contains("CLOSED_SHELL"));
        }
        let repeat = command(&mut session, json!({"command":"rebuild","document":doc}));
        assert_eq!(repeat["cache"]["evaluated_nodes"], 0);
        assert_eq!(repeat["component_steps"], first["component_steps"]);
        let queried = command(
            &mut session,
            json!({"command":"query","component":1,"point":[6.,0.,10.]}),
        );
        assert_eq!(queried["ok"], true, "{queried}");
        assert_eq!(queried["query"]["location"], "inside");
        let segment = command(
            &mut session,
            json!({"command":"segment","component":1,"start":[5.,0.,-5.],"end":[7.,0.,30.]}),
        );
        assert_eq!(segment["ok"], true, "{segment}");
        assert_eq!(segment["segment"]["hits"].as_array().unwrap().len(), 2);
        assert_eq!(
            segment["segment"]["material_intervals"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        for (kind, key) in [("line", "line"), ("ray", "line")] {
            let queried = command(
                &mut session,
                json!({"command":kind,"component":1,"origin":[5.,0.,-5.],"direction":[2.,0.,35.]}),
            );
            assert_eq!(queried["ok"], true, "{queried}");
            assert_eq!(queried[key]["hits"].as_array().unwrap().len(), 2);
            assert_eq!(
                queried[key]["material_intervals"].as_array().unwrap().len(),
                1
            );
        }
        let rejected_query = command(
            &mut session,
            json!({"command":"ray","component":1,"origin":[6.,0.,10.],"direction":[0.,0.,0.]}),
        );
        assert_eq!(rejected_query["ok"], false);
        assert_eq!(rejected_query["accepted_document"], doc);
        let mut skew = doc.clone();
        let n = skew["operations"].as_array().unwrap().len();
        skew["operations"][n - 1]["normal"] = json!([0., 0., 1.]);
        let rejected = command(&mut session, json!({"command":"rebuild","document":skew}));
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["accepted_document"], doc);
        doc["operations"].as_array_mut().unwrap().extend([json!({"kind":"select","id":"middle","input":"second","component":1}),json!({"kind":"plane_split","id":"third","input":"middle","origin":[0.,0.,12.],"normal":[-0.2,-0.1,1.]})]);
        doc["output"] = json!("third");
        let third = command(&mut session, json!({"command":"rebuild","document":doc}));
        assert_eq!(third["ok"], true, "{third}");
        assert_eq!(third["cache"]["evaluated_nodes"], 2);
        for body in third["components"].as_array().unwrap() {
            assert!((body["volume"].as_f64().unwrap() - 4. * area).abs() < 1e-8);
        }
    }
}
