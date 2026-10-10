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
fn actual_bored_child_segments_have_analytic_intervals_and_retained_face_witnesses() {
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
        let report = call(
            &mut session,
            json!({"command":"segment","component":component,"start":[-24.,0.,z],"end":[24.,0.,z]}),
        );
        assert_eq!(report["ok"], true, "{report}");
        let radius = 16. - z / 3.;
        let expected = [
            [(24. - radius) / 48., 20. / 48.],
            [28. / 48., (24. + radius) / 48.],
        ];
        let intervals = report["segment"]["material_intervals"].as_array().unwrap();
        assert_eq!(intervals.len(), 2);
        for (a, b) in intervals.iter().zip(expected) {
            for i in 0..2 {
                assert!((a[i].as_f64().unwrap() - b[i]).abs() < 1e-10);
            }
        }
        assert_eq!(report["segment"]["hits"].as_array().unwrap().len(), 4);
        for hit in report["segment"]["hits"].as_array().unwrap() {
            let point = xyz(&hit["point"]);
            assert!(
                (point - Vec3::new(-24. + 48. * hit["parameter"].as_f64().unwrap(), 0., z)).norm()
                    < 1e-10
            );
            for witness in hit["faces"].as_array().unwrap() {
                let face = witness["face_id"].as_u64().unwrap() as usize;
                let uv = &witness["uv"];
                let actual = shape.components()[component].solid().shell.faces[face]
                    .surface
                    .try_evaluate(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap())
                    .unwrap();
                assert!((actual - point).norm() < 1e-8);
            }
        }
        let caps = call(
            &mut session,
            json!({"command":"segment","component":component,"start":[6.,1.,-4.],"end":[6.,1.,28.]}),
        );
        assert_eq!(caps["ok"], true, "{caps}");
        assert_eq!(
            caps["segment"]["material_intervals"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let expected = if component == 0 {
            [4. / 32., 14.8 / 32.]
        } else {
            [14.8 / 32., 28. / 32.]
        };
        for i in 0..2 {
            assert!(
                (caps["segment"]["material_intervals"][0][i]
                    .as_f64()
                    .unwrap()
                    - expected[i])
                    .abs()
                    < 1e-10
            );
        }
    }
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    assert_eq!((session.undo_count(), session.redo_count()), history);
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
#[test]
fn selected_child_refusals_leave_accepted_geometry_metrics_and_history_intact() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    doc["output"] = json!("selected");
    let accepted = call(
        &mut session,
        json!({"command":"rebuild","document":doc.clone()}),
    );
    assert_eq!(accepted["ok"], true);
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for (start, end) in [
        ([-24., 4., 18.], [24., 4., 18.]),
        ([6., 0., 10.8], [6., 0., 28.]),
        ([1e14, 0., 18.], [24., 0., 18.]),
    ] {
        let report = call(
            &mut session,
            json!({"command":"segment","component":0,"start":start,"end":end}),
        );
        assert_eq!(report["ok"], false, "{report}");
        assert_eq!(report["diagnostic"]["operation_id"], "selected");
        assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
        assert_eq!((session.undo_count(), session.redo_count()), history);
    }
    for kind in ["line", "ray"] {
        assert_eq!(
            call(
                &mut session,
                json!({"command":kind,"component":0,"origin":[0.,0.,18.],"direction":[2.,0.,0.]})
            )["diagnostic"]["category"],
            "unsupported"
        );
    }
    assert_eq!(
        call(
            &mut session,
            json!({"command":"segment","component":0,"start":[-24.,0.,18.],"end":[24.,0.,18.]})
        )["ok"],
        true
    );
    assert_eq!(
        call(
            &mut session,
            json!({"command":"query","component":0,"point":[6.,0.,18.]})
        )["query"]["location"],
        "inside"
    );
    let replay = call(&mut session, json!({"command":"rebuild","document":doc}));
    assert_eq!(replay["components"], accepted["components"]);
    assert_eq!(replay["component_steps"], accepted["component_steps"]);
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
}
