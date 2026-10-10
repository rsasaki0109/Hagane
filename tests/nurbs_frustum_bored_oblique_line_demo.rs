use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> Value {
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.2,"output":"cut","operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.},{"kind":"plane_split","id":"cut","input":"bored","origin":[0.,0.,12.],"normal":[0.2,0.,1.]},{"kind":"select","id":"selected","input":"cut","component":1}]})
}
fn call(s: &mut NurbsFrustumWorkflowSession, value: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(s, &value.to_string()).unwrap(),
    )
    .unwrap()
}
fn xyz(value: &Value) -> Vec3 {
    Vec3::new(
        value[0].as_f64().unwrap(),
        value[1].as_f64().unwrap(),
        value[2].as_f64().unwrap(),
    )
}
#[test]
fn actual_annular_line_and_ray_parameters_intervals_and_uv_witnesses_are_readonly() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document();
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    assert_eq!(accepted["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for (component, z) in [(0, 6.), (1, 18.)] {
        let radius = 16. - z / 3.;
        let line = call(
            &mut session,
            json!({"command":"line","component":component,"origin":[0.,0.,z],"direction":[2.,0.,0.]}),
        );
        assert_eq!(line["ok"], true, "{line}");
        assert_eq!(line["line"]["direction"], json!([2., 0., 0.]));
        assert_eq!(line["line"]["kind"], "line");
        let expected = [[-radius / 2., -2.], [2., radius / 2.]];
        let intervals = line["line"]["material_intervals"].as_array().unwrap();
        assert_eq!(intervals.len(), 2);
        for (actual, expected) in intervals.iter().zip(expected) {
            for i in 0..2 {
                assert!((actual[i].as_f64().unwrap() - expected[i]).abs() < 1e-10);
            }
        }
        for hit in line["line"]["hits"].as_array().unwrap() {
            let point = xyz(&hit["point"]);
            assert!(
                (point - Vec3::new(2. * hit["parameter"].as_f64().unwrap(), 0., z)).norm() < 1e-9
            );
            for witness in hit["faces"].as_array().unwrap() {
                let face = witness["face_id"].as_u64().unwrap() as usize;
                let uv = &witness["uv"];
                let exact = shape.components()[component].solid().shell.faces[face]
                    .surface
                    .try_evaluate(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap())
                    .unwrap();
                assert!((point - exact).norm() < 1e-8);
            }
        }
        let ray = call(
            &mut session,
            json!({"command":"ray","component":component,"origin":[0.,0.,z],"direction":[2.,0.,0.]}),
        );
        assert_eq!(ray["ok"], true);
        for (i, expected) in [2., radius / 2.].iter().enumerate() {
            assert!(
                (ray["line"]["material_intervals"][0][i].as_f64().unwrap() - expected).abs()
                    < 1e-10
            );
        }
        let material = call(
            &mut session,
            json!({"command":"ray","component":component,"origin":[6.,0.,z],"direction":[2.,0.,0.]}),
        );
        assert_eq!(material["ok"], true);
        assert_eq!(material["line"]["material_intervals"][0][0], 0.);
        let axial = call(
            &mut session,
            json!({"command":"line","component":component,"origin":[6.,1.,0.],"direction":[0.,0.,2.]}),
        );
        assert_eq!(axial["ok"], true);
        let expected = if component == 0 {
            [0., 5.4]
        } else {
            [5.4, 12.]
        };
        for i in 0..2 {
            assert!(
                (axial["line"]["material_intervals"][0][i].as_f64().unwrap() - expected[i]).abs()
                    < 1e-10
            );
        }
        assert!(line.get("components").is_none());
        assert_eq!(line["line"]["mesh_free"], true);
    }
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    assert_eq!((session.undo_count(), session.redo_count()), history);
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
#[test]
fn line_and_ray_refusals_preserve_selected_child_and_recovery_reports() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    doc["output"] = json!("selected");
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for (origin, direction) in [
        ([0., 4., 18.], [2., 0., 0.]),
        ([0., 0., 18.], [0., 0., 0.]),
        ([1e14, 0., 18.], [2., 0., 0.]),
    ] {
        for kind in ["line", "ray"] {
            let result = call(
                &mut session,
                json!({"command":kind,"component":0,"origin":origin,"direction":direction}),
            );
            assert_eq!(result["ok"], false, "{result}");
            assert_eq!(result["diagnostic"]["operation_id"], "selected");
            assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
            assert_eq!((session.undo_count(), session.redo_count()), history);
        }
    }
    assert_eq!(
        call(
            &mut session,
            json!({"command":"ray","component":0,"origin":[6.,0.,10.8],"direction":[0.,0.,2.]})
        )["ok"],
        false
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"ray","component":0,"origin":[24.,0.,18.],"direction":[2.,0.,0.]})
        )["line"]["material_intervals"],
        json!([])
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[6.,0.,18.]})
        )["query"]["location"],
        "inside"
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"segment","component":0,"start":[-24.,0.,18.],"end":[24.,0.,18.]})
        )["ok"],
        true
    );
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
}
