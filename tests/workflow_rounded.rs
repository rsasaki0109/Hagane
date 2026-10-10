use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> WorkflowDocument {
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":1e-10},"operations":[{"kind":"rounded_box","id":"rounded-1","size":[80.,60.,20.],"corner_radius":8.},{"kind":"bore","id":"bore-1","input":"rounded-1","mode":"through","center":[-15.,0.],"radius":3.},{"kind":"bore","id":"bore-2","input":"bore-1","mode":"through","center":[15.,0.],"radius":4.}]})).unwrap()
}
#[test]
fn rounded_intent_rebuilds_actual_arc_stock_holes_display_and_step() {
    let d = document();
    let body = d.rebuild().unwrap();
    body.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    let expected =
        (80. * 60. - (4. - std::f64::consts::PI) * 64. - std::f64::consts::PI * 25.) * 20.;
    assert!((body.volume().unwrap() - expected).abs() < 1e-8);
    assert_eq!(
        body.shell
            .faces
            .iter()
            .filter(|f| f.wires.len() == 3)
            .count(),
        2
    );
    assert!(body
        .edges
        .iter()
        .any(|e| matches!(e.curve, Curve::Arc { .. })));
    let text = serde_json::to_string(&d).unwrap();
    let report: Value = serde_json::from_str(&evaluate_workflow_json(&text).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["document"]["operations"][0]["corner_radius"], 8.);
    assert!((report["mesh"]["volume"].as_f64().unwrap() - expected).abs() < 1e-8);
    let exported: Value =
        serde_json::from_str(&export_workflow_step_mm_json(&text).unwrap()).unwrap();
    let step = exported["step"].as_str().unwrap();
    let imported = import_step_bounded_analytic_mm(step, Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((imported.volume().unwrap() - expected).abs() < 1e-8);
    assert!(import_step_mm(step, Tolerance::new(1e-6).unwrap()).is_err());
}
#[test]
fn prefix_cache_rebuilds_real_suffix_and_failed_edits_are_atomic() {
    let mut session = WorkflowSession::new();
    let d = document();
    let accepted = session.rebuild(&d).unwrap();
    assert_eq!(accepted.stats.rebuilt_operations, 3);
    let unchanged = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &unchanged.solid));
    assert_eq!(unchanged.stats.rebuilt_operations, 0);
    let mut edited = d.clone();
    if let WorkflowOperation::Bore { radius, .. } = &mut edited.operations[2] {
        *radius = 4.5;
    }
    let rebuilt = session.rebuild(&edited).unwrap();
    assert_eq!(rebuilt.stats.reused_operations, 2);
    assert_eq!(rebuilt.stats.rebuilt_operation_ids, vec!["bore-2"]);
    assert!(
        (rebuilt.solid.volume().unwrap() - edited.rebuild().unwrap().volume().unwrap()).abs()
            < 1e-9
    );
    let mut bad = edited.clone();
    if let WorkflowOperation::Bore { center, .. } = &mut bad.operations[2] {
        *center = [38., 28.];
    }
    assert!(session.rebuild(&bad).is_err());
    let failed: Value = serde_json::from_str(
        &session
            .evaluate_json(&serde_json::to_string(&bad).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(failed["ok"], false);
    assert!(failed.get("mesh").is_none());
    let stable = session.rebuild(&edited).unwrap();
    assert!(Arc::ptr_eq(&stable.solid, &rebuilt.solid));
    assert_eq!(stable.stats.rebuilt_operations, 0);
    if let WorkflowOperation::RoundedBox { corner_radius, .. } = &mut edited.operations[0] {
        *corner_radius = 9.;
    }
    assert_eq!(session.rebuild(&edited).unwrap().stats.reused_operations, 0);
}
#[test]
fn rounded_corner_scope_and_resource_failures_are_explicit() {
    let mut d = document();
    if let WorkflowOperation::Bore { mode, depth, .. } = &mut d.operations[1] {
        *mode = WorkflowBoreMode::Blind;
        *depth = Some(5.);
    }
    let error = d.rebuild().unwrap_err();
    assert_eq!(error.category, "unsupported");
    assert_eq!(error.code, "rounded_blind_bore_unsupported");
    assert_eq!(error.operation_id.as_deref(), Some("bore-1"));
    for radius in [0., -1., 40., f64::NAN] {
        let mut d = document();
        if let WorkflowOperation::RoundedBox { corner_radius, .. } = &mut d.operations[0] {
            *corner_radius = radius;
        }
        assert_eq!(d.rebuild().unwrap_err().code, "invalid_corner_radius");
    }
    let mut d = document();
    d.operations.truncate(1);
    d.operations.push(WorkflowOperation::Bore {
        id: "corner".into(),
        input: "rounded-1".into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center: [38., 28.],
        radius: 0.5,
        depth: None,
    });
    // Bounding-box clearance is positive, but the actual rounded corner is void.
    assert!(d.rebuild().is_err());
    let mut d = document();
    while d.operations.len() < 18 {
        let last = if let WorkflowOperation::Bore { id, .. } = d.operations.last().unwrap() {
            id.clone()
        } else {
            unreachable!()
        };
        let id = format!("bore-{}", d.operations.len());
        d.operations.push(WorkflowOperation::Bore {
            id,
            input: last,
            mode: WorkflowBoreMode::Through,
            entry: WorkflowBoreEntry::Top,
            center: [0., 0.],
            radius: 0.5,
            depth: None,
        });
    }
    assert_eq!(d.rebuild().unwrap_err().code, "unsupported_history");
}
