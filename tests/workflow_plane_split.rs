use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document(side: &str) -> WorkflowDocument {
    let center = if side == "negative" { -12. } else { 37. };
    let radius = if side == "negative" { 4. } else { 1. };
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":1e-10},"operations":[{"kind":"rounded_box","id":"stock","size":[80.,60.,20.],"corner_radius":8.},{"kind":"plane_split","id":"split-1","input":"stock","offset":34.,"normal_angle":0.,"side":side},{"kind":"bore","id":"bore-1","input":"split-1","mode":"through","center":[center,0.],"radius":radius}]})).unwrap()
}
fn direct(doc: &WorkflowDocument) -> Solid {
    let t = GeometryTolerance::new(1e-6, 1e-10, 1e-10).unwrap();
    let stock = make_box(
        BoxSpec {
            min: Point3::new(-40., -30., -10.),
            size: Vec3::new(80., 60., 20.),
        },
        t.absolute(),
    )
    .unwrap();
    let stock = fillet_parallel_box_edges(&stock, &[(8, 8.), (9, 8.), (10, 8.), (11, 8.)], t)
        .unwrap()
        .into_solid();
    let WorkflowOperation::PlaneSplit {
        offset,
        normal_angle,
        side,
        ..
    } = doc.operations[1]
    else {
        panic!()
    };
    let (s, c) = normal_angle.sin_cos();
    let plane = Surface::Plane {
        origin: Point3::new(c * offset, s * offset, 0.),
        u: Vec3::new(0., 0., 1.),
        v: Vec3::new(s, -c, 0.),
    };
    let (negative, positive) = split_normal_arc_line_prism_by_plane(&stock, &plane, t)
        .unwrap()
        .into_solids();
    let kept = if side == WorkflowSplitSide::Negative {
        negative
    } else {
        positive
    };
    let WorkflowOperation::Bore { center, radius, .. } = doc.operations[2] else {
        panic!()
    };
    bore_normal_arc_line_prism(&kept, Point3::new(center[0], center[1], 0.), radius, t)
        .unwrap()
        .into_solids()
        .0
}
#[test]
fn both_actual_children_followed_by_bore_display_and_step_match_direct_operations() {
    for side in ["negative", "positive"] {
        let d = document(side);
        let body = d.rebuild().unwrap();
        let expected = direct(&d);
        assert_eq!(format!("{body:?}"), format!("{expected:?}"));
        let text = serde_json::to_string(&d).unwrap();
        let report: Value = serde_json::from_str(&evaluate_workflow_json(&text).unwrap()).unwrap();
        assert_eq!(report["ok"], true);
        assert_eq!(report["document"]["operations"][1]["side"], side);
        let exported: Value =
            serde_json::from_str(&export_workflow_step_mm_json(&text).unwrap()).unwrap();
        let imported = import_step_bounded_analytic_mm(
            exported["step"].as_str().unwrap(),
            Tolerance::new(1e-6).unwrap(),
        )
        .unwrap();
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-8);
    }
}
#[test]
fn ordered_split_and_bore_cache_rebuilds_suffix_and_rejects_atomically() {
    let mut session = WorkflowSession::new();
    let mut d = document("negative");
    let initial = session.rebuild(&d).unwrap();
    let same = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&initial.solid, &same.solid));
    assert_eq!(same.stats.rebuilt_operations, 0);
    if let WorkflowOperation::Bore { radius, .. } = &mut d.operations[2] {
        *radius = 3.5;
    }
    assert_eq!(session.rebuild(&d).unwrap().stats.reused_operations, 2);
    if let WorkflowOperation::PlaneSplit { offset, .. } = &mut d.operations[1] {
        *offset = 33.;
    }
    let accepted = session.rebuild(&d).unwrap();
    assert_eq!(accepted.stats.reused_operations, 1);
    assert_eq!(
        accepted.stats.rebuilt_operation_ids,
        vec!["split-1", "bore-1"]
    );
    let mut bad = d.clone();
    if let WorkflowOperation::PlaneSplit { side, .. } = &mut bad.operations[1] {
        *side = WorkflowSplitSide::Positive;
    }
    let failure: Value = serde_json::from_str(
        &session
            .evaluate_json(&serde_json::to_string(&bad).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(failure["ok"], false);
    assert_eq!(failure["diagnostic"]["operation_id"], "bore-1");
    assert!(failure.get("mesh").is_none());
    let stable = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&stable.solid, &accepted.solid));
    let mut terminal = d.clone();
    terminal.operations.truncate(2);
    let prefix = session.rebuild(&terminal).unwrap();
    assert_eq!(prefix.stats.rebuilt_operations, 0);
    assert_eq!(session.rebuild(&d).unwrap().stats.reused_operations, 2);
    let mut no_split = d.clone();
    no_split.operations.remove(1);
    if let WorkflowOperation::Bore { input, .. } = &mut no_split.operations[1] {
        *input = "stock".into();
    }
    let rebuilt = session.rebuild(&no_split).unwrap();
    assert_eq!(rebuilt.stats.reused_operations, 1);
    assert_eq!(
        format!("{:?}", rebuilt.solid),
        format!("{:?}", no_split.rebuild().unwrap())
    );
}
#[test]
fn holes_contacts_bad_roots_and_nonfinite_nodes_reject_with_operation_identity() {
    let mut d = document("negative");
    d.operations.insert(
        1,
        WorkflowOperation::Bore {
            id: "prior-hole".into(),
            input: "stock".into(),
            mode: WorkflowBoreMode::Through,
            entry: WorkflowBoreEntry::Top,
            center: [34., 0.],
            radius: 1.,
            depth: None,
        },
    );
    if let WorkflowOperation::PlaneSplit { input, .. } = &mut d.operations[2] {
        *input = "prior-hole".into();
    }
    assert_eq!(
        d.rebuild().unwrap_err().operation_id.as_deref(),
        Some("split-1")
    );
    for offset in [40., f64::INFINITY] {
        let mut d = document("negative");
        if let WorkflowOperation::PlaneSplit { offset: o, .. } = &mut d.operations[1] {
            *o = offset;
        }
        let error = d.rebuild().unwrap_err();
        assert_eq!(error.operation_id.as_deref(), Some("split-1"));
    }
    let mut d = document("negative");
    d.operations[0] = WorkflowOperation::Box {
        id: "stock".into(),
        size: [80., 60., 20.],
    };
    let solid = d.rebuild().unwrap();
    solid.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((solid.volume().unwrap() - (88800. - 320. * std::f64::consts::PI)).abs() < 1e-8);
    let mut d = document("negative");
    if let WorkflowOperation::Bore { mode, depth, .. } = &mut d.operations[2] {
        *mode = WorkflowBoreMode::Blind;
        *depth = Some(4.);
    }
    assert_eq!(
        d.rebuild().unwrap_err().code,
        "rounded_blind_bore_unsupported"
    );
}

#[test]
fn terminal_line_child_and_appended_second_split_use_the_actual_selected_body() {
    let q = std::f64::consts::FRAC_PI_2;
    let mut d:WorkflowDocument=serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":1e-10},"operations":[{"kind":"arc_line_extrusion","id":"stock","height":5.,"outer":[{"kind":"line","start":[0.,-4.],"end":[20.,-4.]},{"kind":"line","start":[20.,-4.],"end":[20.,4.]},{"kind":"line","start":[20.,4.],"end":[0.,4.]},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":q,"sweep":q},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":2.*q,"sweep":q}]},{"kind":"plane_split","id":"split-1","input":"stock","offset":10.,"normal_angle":0.,"side":"positive"}]})).unwrap();
    let mut session = WorkflowSession::new();
    let terminal = session.rebuild(&d).unwrap();
    assert!(terminal
        .solid
        .edges
        .iter()
        .all(|e| matches!(e.curve, Curve::Line { .. })));
    assert!((terminal.solid.volume().unwrap() - 400.).abs() < 1e-10);
    let report: Value = serde_json::from_str(
        &session
            .evaluate_json(&serde_json::to_string(&d).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["ok"], true);
    let step: Value = serde_json::from_str(
        &export_workflow_step_mm_json(&serde_json::to_string(&d).unwrap()).unwrap(),
    )
    .unwrap();
    assert!(import_step_bounded_analytic_mm(
        step["step"].as_str().unwrap(),
        Tolerance::new(1e-6).unwrap()
    )
    .is_ok());
    d.operations.push(WorkflowOperation::Bore {
        id: "next".into(),
        input: "split-1".into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center: [15., 0.],
        radius: 0.5,
        depth: None,
    });
    let continued = session.rebuild(&d).unwrap();
    assert_eq!(continued.stats.reused_operations, 2);
    assert!(
        (continued.solid.volume().unwrap() - (400. - 5. * std::f64::consts::PI * 0.25)).abs()
            < 1e-8
    );
    assert_eq!(
        format!("{:?}", continued.solid),
        format!("{:?}", d.rebuild().unwrap())
    );
    d.operations.pop();
    assert!(Arc::ptr_eq(
        &session.rebuild(&d).unwrap().solid,
        &terminal.solid
    ));
    let mut d = document("negative");
    d.operations.truncate(2);
    let mut session = WorkflowSession::new();
    let first = session.rebuild(&d).unwrap();
    let initial_volume = first.solid.volume().unwrap();
    d.operations.push(WorkflowOperation::PlaneSplit {
        id: "split-2".into(),
        input: "split-1".into(),
        offset: 0.,
        normal_angle: q,
        side: WorkflowSplitSide::Negative,
    });
    let second = session.rebuild(&d).unwrap();
    assert_eq!(second.stats.reused_operations, 2);
    assert_eq!(second.stats.rebuilt_operation_ids, vec!["split-2"]);
    assert!((second.solid.volume().unwrap() - initial_volume / 2.).abs() < 1e-8);
    d.operations.push(WorkflowOperation::Bore {
        id: "last".into(),
        input: "split-2".into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center: [-12., -10.],
        radius: 1.,
        depth: None,
    });
    let bored = session.rebuild(&d).unwrap();
    assert_eq!(bored.stats.reused_operations, 3);
    assert_eq!(
        format!("{:?}", bored.solid),
        format!("{:?}", d.rebuild().unwrap())
    );
}

#[test]
fn split_nodes_do_not_consume_the_sixteen_bore_budget() {
    let mut d = document("negative");
    d.operations.truncate(1);
    for i in 0..17 {
        let input = if i == 0 {
            "stock".into()
        } else {
            format!("split-{i}")
        };
        d.operations.push(WorkflowOperation::PlaneSplit {
            id: format!("split-{}", i + 1),
            input,
            offset: 34. - i as f64 * 0.08,
            normal_angle: 0.,
            side: WorkflowSplitSide::Negative,
        });
    }
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&d).unwrap();
    assert_eq!(accepted.stats.rebuilt_operations, 18);
    let unchanged = session.rebuild(&d).unwrap();
    assert_eq!(unchanged.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&accepted.solid, &unchanged.solid));
    let mut single = d.clone();
    single.operations.truncate(1);
    single.operations.push(WorkflowOperation::PlaneSplit {
        id: "last".into(),
        input: "stock".into(),
        offset: 34. - 16. * 0.08,
        normal_angle: 0.,
        side: WorkflowSplitSide::Negative,
    });
    assert!(
        (single.rebuild().unwrap().volume().unwrap() - accepted.solid.volume().unwrap()).abs()
            < 1e-8
    );
}
