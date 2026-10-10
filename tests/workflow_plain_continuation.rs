use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> WorkflowDocument {
    let q = std::f64::consts::FRAC_PI_2;
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[{"kind":"arc_line_extrusion","id":"stock","height":5.,"outer":[{"kind":"line","start":[0.,-4.],"end":[20.,-4.]},{"kind":"line","start":[20.,-4.],"end":[20.,4.]},{"kind":"line","start":[20.,4.],"end":[0.,4.]},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":q,"sweep":q},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":2.*q,"sweep":q}]},{"kind":"plane_split","id":"plain","input":"stock","offset":10.,"normal_angle":0.,"side":"positive"}]})).unwrap()
}
fn bore(input: &str) -> WorkflowOperation {
    WorkflowOperation::Bore {
        id: "bore".into(),
        input: input.into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center: [15., 0.],
        radius: 1.,
        depth: None,
    }
}
fn cut(input: &str, offset: f64) -> WorkflowOperation {
    WorkflowOperation::PlaneSplit {
        id: "next-cut".into(),
        input: input.into(),
        offset,
        normal_angle: 0.,
        side: WorkflowSplitSide::Negative,
    }
}
fn fresh_equal(doc: &WorkflowDocument, solid: &Solid) {
    assert_eq!(
        format!("{solid:?}"),
        format!("{:?}", doc.rebuild().unwrap())
    );
    solid.validate(Tolerance::new(1e-6).unwrap()).unwrap();
}
#[test]
fn plain_child_bore_and_further_cut_use_actual_retained_geometry() {
    let mut d = document();
    let mut session = WorkflowSession::new();
    let plain = session.rebuild(&d).unwrap();
    assert!((plain.solid.volume().unwrap() - 400.).abs() < 1e-9);
    assert!(plain
        .solid
        .edges
        .iter()
        .all(|e| matches!(e.curve, Curve::Line { .. })));
    d.operations.push(bore("plain"));
    let bored = session.rebuild(&d).unwrap();
    assert_eq!(bored.stats.reused_operations, 2);
    assert!((bored.solid.volume().unwrap() - (400. - 5. * std::f64::consts::PI)).abs() < 1e-8);
    let holes = bored
        .solid
        .shell
        .faces
        .iter()
        .map(|f| f.wires.len() - 1)
        .sum::<usize>();
    assert_eq!(
        bored.solid.vertices.len() as i64 - bored.solid.edges.len() as i64
            + bored.solid.shell.faces.len() as i64
            - holes as i64,
        0
    );
    fresh_equal(&d, &bored.solid);
    d.operations.push(cut("bore", 18.));
    let cut = session.rebuild(&d).unwrap();
    assert_eq!(cut.stats.reused_operations, 3);
    assert!((cut.solid.volume().unwrap() - (320. - 5. * std::f64::consts::PI)).abs() < 1e-8);
    fresh_equal(&d, &cut.solid);
    let step: Value = serde_json::from_str(
        &export_workflow_step_mm_json(&serde_json::to_string(&d).unwrap()).unwrap(),
    )
    .unwrap();
    let imported = import_step_bounded_analytic_mm(
        step["step"].as_str().unwrap(),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    assert!((imported.volume().unwrap() - cut.solid.volume().unwrap()).abs() < 1e-8);
    d.operations.pop();
    let restored = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&restored.solid, &bored.solid));
    fresh_equal(&d, &restored.solid);
}
#[test]
fn plain_child_further_partition_and_wrong_side_edit_preserve_cache_atomically() {
    let mut d = document();
    d.operations.push(cut("plain", 15.));
    let mut session = WorkflowSession::new();
    let divided = session.rebuild(&d).unwrap();
    assert!((divided.solid.volume().unwrap() - 200.).abs() < 1e-8);
    fresh_equal(&d, &divided.solid);
    d.operations.pop();
    d.operations.push(bore("plain"));
    let accepted = session.rebuild(&d).unwrap();
    let mut rejected = d.clone();
    let WorkflowOperation::PlaneSplit { side, .. } = &mut rejected.operations[1] else {
        panic!()
    };
    *side = WorkflowSplitSide::Negative;
    let error = session.rebuild(&rejected).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("bore"));
    let unchanged = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &unchanged.solid));
    assert_eq!(unchanged.stats.rebuilt_operations, 0);
    let mut crossing = d.clone();
    crossing.operations.push(WorkflowOperation::PlaneSplit {
        id: "notch".into(),
        input: "bore".into(),
        offset: 15. * 0.37_f64.cos() + 0.3,
        normal_angle: 0.37,
        side: WorkflowSplitSide::Negative,
    });
    let actual = session.rebuild(&crossing).unwrap();
    fresh_equal(&crossing, &actual.solid);
    assert!(actual.solid.volume().unwrap() > 0.);
    for face in &actual.solid.shell.faces {
        if let Surface::Plane { u, v, .. } = face.surface {
            if u.cross(v).normalized().unwrap().z.abs() > 0.99 {
                assert_eq!(face.wires.len(), 1);
            }
        }
    }
}
