use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn doc(radii: [f64; 2], posed: bool) -> Value {
    let (sin, cos) = if posed { 0.37_f64.sin_cos() } else { (0., 1.) };
    let axes = [[cos, 0., -sin], [0., 1., 0.], [sin, 0., cos]];
    let origin = if posed { [12., -5., 8.] } else { [0., 0., 0.] };
    let cut = [origin[0] + 12. * sin, origin[1], origin[2] + 12. * cos];
    let normal = [sin + 0.2 * cos, 0.1, cos - 0.2 * sin];
    let mut doc = json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.2,"output":"cut","operations":[{"kind":"frustum","id":"stock","radii":radii,"height":24.,"origin":origin,"axes":axes},{"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.},{"kind":"plane_split","id":"cut","input":"bored","origin":cut,"normal":normal}]});
    if posed {
        doc["operations"][0] = json!({"kind":"posed_frustum","id":"stock","radii":radii,"height":24.,"origin":origin,"rotation_axis":[0.,1.,0.],"angle":0.37});
    }
    doc
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
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 1e-8 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
#[test]
fn actual_reported_mass_and_world_tensor_conserve_parent_without_changing_geometry() {
    for radii in [[16., 8.], [12., 12.]] {
        for posed in [false, true] {
            let mut session = NurbsFrustumWorkflowSession::new();
            let document = doc(radii, posed);
            let report = rebuild(&mut session, document.clone());
            assert_eq!(report["ok"], true, "{report}");
            let shape = session.accepted_shape().unwrap().clone();
            let parent = session.prefix_snapshot(1).unwrap().components()[0]
                .bored()
                .unwrap()
                .inertia_properties(policy())
                .unwrap();
            let mut volume = 0.;
            let mut first = Vec3::new(0., 0., 0.);
            let mut tensor = [[0.; 3]; 3];
            for (i, component) in shape.components().iter().enumerate() {
                let NurbsFrustumWorkflowComponent::BoredOblique(child) = component else {
                    panic!("actual bored oblique component required")
                };
                let metrics = child.inertia_properties(policy()).unwrap();
                let data = &report["components"][i];
                assert_eq!(data["mass_properties"]["ok"], true);
                assert_eq!(data["mass_properties"]["inertia"], json!(metrics.inertia));
                assert_eq!(
                    data["mass_properties"]["centroid"],
                    json!([metrics.centroid.x, metrics.centroid.y, metrics.centroid.z])
                );
                assert_eq!(data["step"], child.export_step_mm(policy()).unwrap());
                let mesh = child.tessellate(0.2, policy()).unwrap();
                assert_eq!(data["mesh"]["triangles"], json!(mesh.triangles));
                assert_eq!(
                    data["mesh"]["positions"],
                    json!(mesh
                        .positions
                        .iter()
                        .map(|p| [p.x, p.y, p.z])
                        .collect::<Vec<_>>())
                );
                let before = format!("{:?}", child.solid());
                child.mass_properties(policy()).unwrap();
                child.inertia_properties(policy()).unwrap();
                assert_eq!(before, format!("{:?}", child.solid()));
                volume += metrics.volume;
                first = first + metrics.centroid * metrics.volume;
                let delta = metrics.centroid - parent.centroid;
                let d = [delta.x, delta.y, delta.z];
                for a in 0..3 {
                    for b in 0..3 {
                        tensor[a][b] += metrics.inertia[a][b]
                            + metrics.volume
                                * (if a == b { delta.dot(delta) } else { 0. } - d[a] * d[b]);
                    }
                }
            }
            close(
                volume,
                std::f64::consts::PI
                    * 24.
                    * ((radii[0] * radii[0] + radii[0] * radii[1] + radii[1] * radii[1]) / 3.
                        - 16.),
            );
            close(volume, parent.volume);
            let centroid = first * (1. / volume);
            for (a, b) in [centroid.x, centroid.y, centroid.z].into_iter().zip([
                parent.centroid.x,
                parent.centroid.y,
                parent.centroid.z,
            ]) {
                close(a, b);
            }
            for (a, row) in tensor.iter().enumerate() {
                for (b, value) in row.iter().enumerate() {
                    close(*value, parent.inertia[a][b]);
                }
            }
            let replay = rebuild(&mut session, document);
            assert_eq!(replay["components"], report["components"]);
            assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
            assert_eq!(replay["cache"]["evaluated_nodes"], 0);
        }
    }
}
#[test]
fn unresolved_optional_metrics_retain_actual_geometry_exports_and_cached_acceptance() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let document = doc([16., 16. + 1e-8], false);
    let report = rebuild(&mut session, document.clone());
    assert_eq!(report["ok"], true, "{report}");
    let shape = session.accepted_shape().unwrap().clone();
    let history = (session.undo_count(), session.redo_count());
    for (i, component) in shape.components().iter().enumerate() {
        let NurbsFrustumWorkflowComponent::BoredOblique(child) = component else {
            panic!("actual bored oblique component required")
        };
        assert!(matches!(
            child.inertia_properties(policy()),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(report["components"][i]["mass_properties"]["ok"], false);
        assert!(report["components"][i]["mass_properties"]
            .get("centroid")
            .is_none());
        assert!(child.volume(policy()).unwrap() > 0.);
        assert_eq!(
            report["component_steps"][i],
            child.export_step_mm(policy()).unwrap()
        );
        assert!(!child
            .tessellate(0.2, policy())
            .unwrap()
            .triangles
            .is_empty());
    }
    assert_eq!((session.undo_count(), session.redo_count()), history);
    let replay = rebuild(&mut session, document);
    assert_eq!(replay["components"], report["components"]);
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
}
