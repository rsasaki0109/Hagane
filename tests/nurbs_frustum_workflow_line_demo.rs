use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn call(session: &mut NurbsFrustumWorkflowSession, command: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(session, &command.to_string()).unwrap(),
    )
    .unwrap()
}
fn doc(kind: &str) -> Value {
    let mut d = json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.1,"output":"stock","operations":[{"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}]});
    if kind == "bore" {
        d["operations"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"coaxial_bore","id":"result","input":"stock","radius":4.}));
        d["output"] = json!("result");
    }
    if kind == "split" {
        d["operations"].as_array_mut().unwrap().push(json!({"kind":"plane_split","id":"result","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}));
        d["output"] = json!("result");
    }
    d
}
#[test]
fn actual_intervals_parameters_and_rays_are_readonly_for_all_component_types() {
    for kind in ["canonical", "bore", "split"] {
        let mut session = NurbsFrustumWorkflowSession::new();
        let accepted = call(
            &mut session,
            json!({"command":"rebuild","document":doc(kind)}),
        );
        assert_eq!(accepted["ok"], true);
        let shape = session.accepted_shape().unwrap().clone();
        let history = (session.undo_count(), session.redo_count());
        let report = call(
            &mut session,
            json!({"command":"line","component":0,"origin":[0.,0.,6.],"direction":[2.,0.,0.]}),
        );
        assert_eq!(report["ok"], true, "{report}");
        assert_eq!(report["line"]["kind"], "line");
        assert_eq!(report["line"]["direction"], json!([2., 0., 0.]));
        let expected = if kind == "bore" {
            json!([[-6., -2.], [2., 6.]])
        } else {
            json!([[-6., 6.]])
        };
        for (a, b) in report["line"]["material_intervals"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected.as_array().unwrap())
        {
            for i in 0..2 {
                assert!((a[i].as_f64().unwrap() - b[i].as_f64().unwrap()).abs() < 1e-10);
            }
        }
        assert_eq!(
            report["line"]["material_intervals"]
                .as_array()
                .unwrap()
                .len(),
            expected.as_array().unwrap().len()
        );
        for hit in report["line"]["hits"].as_array().unwrap() {
            assert!(
                (hit["point"][0].as_f64().unwrap() - 2. * hit["parameter"].as_f64().unwrap()).abs()
                    < 1e-10
            );
            assert!(!hit["faces"].as_array().unwrap().is_empty());
        }
        let ray = call(
            &mut session,
            json!({"command":"ray","component":0,"origin":[0.,0.,6.],"direction":[2.,0.,0.]}),
        );
        assert_eq!(ray["ok"], true, "{ray}");
        let first = if kind == "bore" { 2. } else { 0. };
        assert!((ray["line"]["material_intervals"][0][0].as_f64().unwrap() - first).abs() < 1e-10);
        let away = call(
            &mut session,
            json!({"command":"ray","component":0,"origin":[20.,0.,6.],"direction":[2.,0.,0.]}),
        );
        assert_eq!(away["line"]["material_intervals"], json!([]));
        for key in [
            "components",
            "component_steps",
            "mesh",
            "cache",
            "history",
            "document",
        ] {
            assert!(report.get(key).is_none());
        }
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
        let replay = call(
            &mut session,
            json!({"command":"rebuild","document":doc(kind)}),
        );
        assert_eq!(replay["component_steps"], accepted["component_steps"]);
        assert_eq!(replay["cache"]["evaluated_nodes"], 0);
    }
}
#[test]
fn strict_and_geometry_errors_preserve_accepted_state_and_queries_recover() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let valid = json!({"command":"line","component":0,"origin":[0.,0.,6.],"direction":[2.,0.,0.]});
    assert_eq!(
        call(&mut session, valid.clone())["diagnostic"]["code"],
        "no_accepted_document"
    );
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc("bore")}),
    );
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for bad in [
        json!({"command":"line","component":0,"origin":[0.,0.,6.],"direction":[0.,0.,0.]}),
        json!({"command":"ray","component":0,"origin":[4.,0.,6.],"direction":[2.,0.,0.]}),
        json!({"command":"line","component":0,"origin":[1e14,0.,6.],"direction":[2.,0.,0.]}),
        json!({"command":"line","component":9,"origin":[0.,0.,6.],"direction":[2.,0.,0.]}),
        json!({"command":"line","component":0,"origin":[0.,0.,6.],"direction":[2.,0.,0.],"extra":true}),
    ] {
        assert_eq!(call(&mut session, bad)["ok"], false);
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    let bad = nurbs_frustum_workflow_session_command_json(
        &mut session,
        "{\"command\":\"line\",\"component\":0,\"origin\":[1e999,0,6],\"direction\":[2,0,0]}",
    )
    .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&bad).unwrap()["ok"], false);
    assert_eq!(call(&mut session, valid)["ok"], true);
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[6.,0.,6.]})
        )["query"]["location"],
        "inside"
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"segment","component":0,"start":[-20.,0.,6.],"end":[20.,0.,6.]})
        )["ok"],
        true
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"rebuild","document":doc("bore")})
        )["component_steps"],
        accepted["component_steps"]
    );
}
