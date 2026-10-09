use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn value() -> Value {
    serde_json::from_str(include_str!("../docs/graph-workflow.json")).unwrap()
}
fn typed(v: &Value) -> GraphWorkflowDocument {
    serde_json::from_value(v.clone()).unwrap()
}
fn same_fresh(v: &Value, result: &GraphWorkflowRebuild) {
    let d = typed(v);
    let fresh = d.rebuild().unwrap();
    let t = Tolerance::new(d.tolerance.linear).unwrap();
    assert_eq!(
        result.shape.export_step_mm(t).unwrap(),
        fresh.export_step_mm(t).unwrap()
    );
    assert_eq!(
        result.shape.volume().unwrap().to_bits(),
        fresh.volume().unwrap().to_bits()
    );
}
#[test]
fn actual_prefix_arcs_survive_bore_and_pose_edits_and_match_fresh_replay() {
    let mut session = GraphWorkflowSession::new();
    let mut v = value();
    let first = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(first.stats.rebuilt_operations, 4);
    same_fresh(&v, &first);
    let stock = Arc::clone(session.prefix_snapshot(0).unwrap());
    let trim = Arc::clone(session.prefix_snapshot(1).unwrap());
    let pose = Arc::clone(session.prefix_snapshot(2).unwrap());
    v["operations"][3]["radius"] = json!(8.);
    let bore = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(bore.stats.reused_operations, 3);
    assert_eq!(bore.stats.rebuilt_operation_ids, vec!["bore"]);
    assert!(Arc::ptr_eq(&pose, session.prefix_snapshot(2).unwrap()));
    same_fresh(&v, &bore);
    v["operations"][2]["angle"] = json!(0.2);
    let moved = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(moved.stats.reused_operations, 2);
    assert_eq!(moved.stats.rebuilt_operations, 2);
    assert!(Arc::ptr_eq(&stock, session.prefix_snapshot(0).unwrap()));
    assert!(Arc::ptr_eq(&trim, session.prefix_snapshot(1).unwrap()));
    assert!(!Arc::ptr_eq(&pose, session.prefix_snapshot(2).unwrap()));
    same_fresh(&v, &moved);
    v["operations"][0]["bulge"] = json!(20.);
    let changed = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(changed.stats.reused_operations, 0);
    assert_eq!(changed.stats.rebuilt_operations, 4);
    assert!(!Arc::ptr_eq(&stock, session.prefix_snapshot(0).unwrap()));
    same_fresh(&v, &changed);
}
#[test]
fn display_only_edits_reuse_geometry_but_tolerance_and_reset_invalidate() {
    let mut session = GraphWorkflowSession::new();
    let mut v = value();
    let first = session.rebuild(&typed(&v)).unwrap();
    v["display"]["max_error"] = json!(0.25);
    let display = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(display.stats.reused_operations, 4);
    assert_eq!(display.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&first.shape, &display.shape));
    assert_eq!(session.accepted_document(), Some(&typed(&v)));
    v["tolerance"]["linear"] = json!(2e-8);
    let tolerance = session.rebuild(&typed(&v)).unwrap();
    assert_eq!(tolerance.stats.reused_operations, 0);
    assert_eq!(tolerance.stats.rebuilt_operations, 4);
    assert!(!Arc::ptr_eq(&first.shape, &tolerance.shape));
    session.reset();
    assert!(session.accepted_document().is_none());
    assert!(session.accepted_shape().is_none());
    assert!(session.prefix_snapshot(0).is_none());
    assert_eq!(
        session
            .rebuild(&typed(&v))
            .unwrap()
            .stats
            .rebuilt_operations,
        4
    );
}
#[test]
fn candidate_geometry_display_and_parse_failures_preserve_all_accepted_state() {
    let mut session = GraphWorkflowSession::new();
    let baseline = value();
    let first = session.rebuild(&typed(&baseline)).unwrap();
    let accepted = typed(&baseline);
    let stock = Arc::clone(session.prefix_snapshot(0).unwrap());
    let mut invalid = baseline.clone();
    invalid["operations"][3]["radius"] = json!(100.);
    assert!(session.rebuild(&typed(&invalid)).is_err());
    for input in [invalid.to_string(), "{}".into(), " ".repeat(65537)] {
        let report: Value = serde_json::from_str(&session.evaluate_json(&input).unwrap()).unwrap();
        assert_eq!(report["ok"], false);
        assert!(Arc::ptr_eq(&first.shape, session.accepted_shape().unwrap()));
        assert_eq!(session.accepted_document(), Some(&accepted));
        assert!(Arc::ptr_eq(&stock, session.prefix_snapshot(0).unwrap()));
    }
    let mut edited = baseline.clone();
    edited["operations"][3]["radius"] = json!(8.);
    let report: Value = serde_json::from_str(
        &session
            .evaluate_json_with(&edited.to_string(), |_, _| {
                Err(Error::Tessellation("independent display rejection"))
            })
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["diagnostic"]["code"], "display_rejected");
    assert_eq!(report["diagnostic"]["category"], "validation_failed");
    assert!(Arc::ptr_eq(&first.shape, session.accepted_shape().unwrap()));
    assert_eq!(session.accepted_document(), Some(&accepted));
    let precision: Value = serde_json::from_str(
        &session
            .evaluate_json_with(&edited.to_string(), |_, _| {
                Err(Error::Unsupported(
                    "independent numerical precision rejection",
                ))
            })
            .unwrap(),
    )
    .unwrap();
    assert_eq!(precision["diagnostic"]["category"], "unsupported");
    assert!(Arc::ptr_eq(&first.shape, session.accepted_shape().unwrap()));
    edited["display"]["max_triangles"] = json!(32);
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&edited.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], false);
    assert!(Arc::ptr_eq(&first.shape, session.accepted_shape().unwrap()));
    edited["display"]["max_triangles"] = json!(65536);
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&edited.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["rebuild"]["reused_operations"], 3);
    assert_eq!(report["rebuild"]["rebuilt_operations"], 1);
    assert_eq!(session.accepted_document(), Some(&typed(&edited)));
}
#[test]
fn shortening_history_retains_last_plain_snapshot_and_appending_rebuilds_one() {
    let mut session = GraphWorkflowSession::new();
    let full = value();
    session.rebuild(&typed(&full)).unwrap();
    let pose = Arc::clone(session.prefix_snapshot(2).unwrap());
    let mut short = full.clone();
    short["operations"].as_array_mut().unwrap().pop();
    let result = session.rebuild(&typed(&short)).unwrap();
    assert_eq!(result.stats.reused_operations, 3);
    assert_eq!(result.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&pose, &result.shape));
    assert!(session.prefix_snapshot(3).is_none());
    same_fresh(&short, &result);
    let result = session.rebuild(&typed(&full)).unwrap();
    assert_eq!(result.stats.reused_operations, 3);
    assert_eq!(result.stats.rebuilt_operations, 1);
    same_fresh(&full, &result);
}
