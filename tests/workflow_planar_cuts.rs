use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn doc(root: Value) -> WorkflowDocument {
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[root]})).unwrap()
}
fn bore(id: &str, input: &str, center: [f64; 2], radius: f64) -> WorkflowOperation {
    WorkflowOperation::Bore {
        id: id.into(),
        input: input.into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center,
        radius,
        depth: None,
    }
}
fn cut(input: &str, offset: f64) -> WorkflowOperation {
    WorkflowOperation::PlaneSplit {
        id: "cut".into(),
        input: input.into(),
        offset,
        normal_angle: 0.,
        side: WorkflowSplitSide::Negative,
    }
}
fn check(doc: &WorkflowDocument, solid: &Solid) {
    solid.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    assert_eq!(
        format!("{solid:?}"),
        format!("{:?}", doc.rebuild().unwrap())
    );
    for edge in 0..solid.edges.len() {
        let mut uses = Vec::new();
        for face in &solid.shell.faces {
            for wire in &face.wires {
                for c in &wire.coedges {
                    if c.edge == edge {
                        uses.push(if face.orientation > 0 {
                            c.forward
                        } else {
                            !c.forward
                        });
                    }
                    for j in 0..=4 {
                        let range = solid.edges[c.edge].curve.range();
                        let t = range[0] + (range[1] - range[0]) * j as f64 / 4.;
                        let uv = c.pcurve.try_evaluate(t).unwrap();
                        assert!(
                            (solid.edges[c.edge].curve.try_evaluate(t).unwrap()
                                - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                            .norm()
                                < 1e-6
                        );
                    }
                }
            }
        }
        assert_eq!(uses.len(), 2);
        assert_ne!(uses[0], uses[1]);
    }
    let exported: Value = serde_json::from_str(
        &export_workflow_step_mm_json(&serde_json::to_string(doc).unwrap()).unwrap(),
    )
    .unwrap();
    let imported = if doc
        .operations
        .iter()
        .any(|op| matches!(op, WorkflowOperation::PlaneSplit { .. }))
    {
        import_step_bounded_analytic_mm(
            exported["step"].as_str().unwrap(),
            Tolerance::new(1e-6).unwrap(),
        )
    } else {
        import_step_mm(
            exported["step"].as_str().unwrap(),
            Tolerance::new(1e-6).unwrap(),
        )
    }
    .unwrap();
    assert!((imported.volume().unwrap() - solid.volume().unwrap()).abs() < 1e-6);
}
#[test]
fn box_bores_before_after_cut_and_mode_transitions_rebuild_actual_prefixes() {
    let mut d = doc(json!({"kind":"box","id":"stock","size":[40.,30.,10.]}));
    d.operations.push(bore("first", "stock", [-10., 0.], 2.));
    let mut session = WorkflowSession::new();
    let legacy = session.rebuild(&d).unwrap();
    assert!((legacy.solid.volume().unwrap() - (12000. - 40. * std::f64::consts::PI)).abs() < 1e-6);
    d.operations.push(cut("first", 0.));
    let split = session.rebuild(&d).unwrap();
    assert_eq!(split.stats.reused_operations, 1);
    assert_eq!(split.stats.rebuilt_operation_ids, vec!["first", "cut"]);
    assert!((split.solid.volume().unwrap() - (6000. - 40. * std::f64::consts::PI)).abs() < 1e-6);
    check(&d, &split.solid);
    d.operations.push(bore("second", "cut", [-5., 7.], 1.));
    let bored = session.rebuild(&d).unwrap();
    assert_eq!(bored.stats.reused_operations, 3);
    assert!((bored.solid.volume().unwrap() - (6000. - 50. * std::f64::consts::PI)).abs() < 1e-6);
    check(&d, &bored.solid);
    d.operations.pop();
    assert!(Arc::ptr_eq(
        &session.rebuild(&d).unwrap().solid,
        &split.solid
    ));
    d.operations.pop();
    let restored = session.rebuild(&d).unwrap();
    assert_eq!(restored.stats.reused_operations, 1);
    assert_eq!(restored.stats.rebuilt_operation_ids, vec!["first"]);
    check(&d, &restored.solid);
}
#[test]
fn concave_polygon_initial_hole_and_cut_crossing_hole_use_actual_material() {
    let root = json!({"kind":"extrusion","id":"stock","height":10.,"outer":[[-20.,-15.],[20.,-15.],[20.,-5.],[0.,-5.],[0.,15.],[-20.,15.]],"holes":[[[-12.,-3.],[-12.,3.],[-8.,3.],[-8.,-3.]]]});
    for (offset, expected) in [(-5., 4260.), (-10.5, 2760.)] {
        let mut d = doc(root.clone());
        d.operations.push(cut("stock", offset));
        let actual = d.rebuild().unwrap();
        assert!((actual.volume().unwrap() - expected).abs() < 1e-6);
        check(&d, &actual);
    }
}
#[test]
fn rejected_wrong_side_blind_or_skew_edits_preserve_actual_accepted_cache() {
    let mut d = doc(json!({"kind":"box","id":"stock","size":[40.,30.,10.]}));
    d.operations.push(cut("stock", 0.));
    d.operations.push(bore("bore", "cut", [-10., 0.], 1.));
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&d).unwrap();
    let mut wrong = d.clone();
    let WorkflowOperation::PlaneSplit { side, .. } = &mut wrong.operations[1] else {
        panic!()
    };
    *side = WorkflowSplitSide::Positive;
    let error = session.rebuild(&wrong).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("bore"));
    let mut blind = d.clone();
    let WorkflowOperation::Bore { mode, depth, .. } = &mut blind.operations[2] else {
        panic!()
    };
    *mode = WorkflowBoreMode::Blind;
    *depth = Some(3.);
    assert!(session.rebuild(&blind).is_err());
    assert!(Arc::ptr_eq(
        &session.rebuild(&d).unwrap().solid,
        &accepted.solid
    ));
    let mut skew = doc(
        json!({"kind":"extrusion","id":"stock","height":10.,"offset":[1.,0.],"outer":[[-20.,-15.],[20.,-15.],[20.,15.],[-20.,15.]]}),
    );
    skew.operations.push(cut("stock", 0.));
    assert!(session.rebuild(&skew).is_err());
    assert!(Arc::ptr_eq(
        &session.rebuild(&d).unwrap().solid,
        &accepted.solid
    ));
}
