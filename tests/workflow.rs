use hagane::*;
use serde_json::{json, Value};
fn document() -> Value {
    serde_json::from_str(include_str!("../docs/workflow-example.json")).unwrap()
}
fn run(d: &Value) -> Value {
    serde_json::from_str(&evaluate_workflow_json(&d.to_string()).unwrap()).unwrap()
}
#[test]
fn exact_history_roundtrips_and_rebuilds_analytic_shapes() {
    let mut d = document();
    for mode in ["blind", "through"] {
        d["operations"][1]["mode"] = json!(mode);
        if mode == "through" {
            d["operations"][1].as_object_mut().unwrap().remove("depth");
        } else {
            d["operations"][1]["depth"] = json!(16.);
        }
        let result = run(&d);
        assert_eq!(result["ok"], true);
        assert_eq!(result, run(&result["document"]));
        let typed: WorkflowDocument = serde_json::from_value(d.clone()).unwrap();
        let s = typed.rebuild().unwrap();
        s.validate(Tolerance::default()).unwrap();
        let depth = if mode == "blind" { 16. } else { 24. };
        assert!(
            (s.volume().unwrap() - (115200. - std::f64::consts::PI * 196. * depth)).abs() < 1e-8
        );
        assert_eq!(result["document"]["operations"][1]["input"], "box-1");
    }
    d["operations"].as_array_mut().unwrap().pop();
    assert_eq!(run(&d)["mesh"]["volume"], 115200.);
}
#[test]
fn rejected_edits_have_measured_diagnostics_and_no_replacement_mesh() {
    let mut d = document();
    d["operations"][1]["radius"] = json!(30.);
    let result = run(&d);
    assert_eq!(result["ok"], false);
    assert_eq!(result["diagnostic"]["code"], "side_clearance");
    assert_eq!(result["diagnostic"]["operation_id"], "bore-1");
    assert_eq!(result["diagnostic"]["measured_clearance"], 0.);
    assert!(result["diagnostic"]["required_clearance"].as_f64().unwrap() > 0.);
    assert!(result.get("mesh").is_none());
    assert_eq!(result["candidate_segments"].as_array().unwrap().len(), 100);
    d = document();
    d["operations"][1]["depth"] = json!(24.);
    let result = run(&d);
    assert_eq!(result["diagnostic"]["code"], "floor_thickness");
    assert!(result.get("mesh").is_none());
    d["operations"][1]["depth"] = json!(16.);
    assert_eq!(run(&d)["ok"], true);
    d["operations"][1]["center"] = json!([40., 0.]);
    assert_eq!(run(&d)["diagnostic"]["code"], "side_clearance");
    d = document();
    d["operations"][1]["center"] = json!([f64::MAX, 0.]);
    d["operations"][1]["radius"] = json!(f64::MAX);
    assert_eq!(run(&d)["diagnostic"]["code"], "finite_clearance");
    d = document();
    d["operations"][0]["size"] = json!([80., 60., 0.]);
    assert_eq!(run(&d)["diagnostic"]["operation_id"], "box-1");
}
#[test]
fn invalid_documents_versions_units_references_and_operations_are_rejected() {
    for (key, value, code) in [
        ("schema_version", json!(2), "unsupported_schema"),
        ("units", json!("inch"), "unsupported_units"),
    ] {
        let mut d = document();
        d[key] = value;
        assert_eq!(run(&d)["diagnostic"]["code"], code);
    }
    for (key, value) in [
        ("radius", json!(-1.)),
        ("depth", json!(0.)),
        ("input", json!("missing")),
        ("id", json!("box-1")),
        ("kind", json!("fillet")),
        ("unexpected", json!(true)),
    ] {
        let mut d = document();
        d["operations"][1][key] = value;
        assert_eq!(run(&d)["ok"], false);
    }
    let mut d = document();
    d["operations"][1]["mode"] = json!("through");
    assert_eq!(run(&d)["diagnostic"]["code"], "unexpected_depth");
    let mut d = document();
    d["tolerance"]["linear"] = json!(-1.);
    assert_eq!(run(&d)["diagnostic"]["code"], "invalid_tolerance");
    for text in [
        "{}",
        "{",
        "[]",
        "{\"schema_version\":1,\"schema_version\":2}",
        "1e999",
    ] {
        let result: Value = serde_json::from_str(&evaluate_workflow_json(text).unwrap()).unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(result["diagnostic"]["code"], "invalid_document");
    }
    let result: Value =
        serde_json::from_str(&evaluate_workflow_json(&" ".repeat(65537)).unwrap()).unwrap();
    assert_eq!(result["ok"], false);
}

#[test]
fn mixed_history_preserves_all_cuts_and_identifies_failed_operation() {
    let mut d = document();
    d["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":"bore-2","input":"bore-1","mode":"through","center":[24.,0.],"radius":4.}));
    let expected = 115200. - std::f64::consts::PI * (196. * 16. + 16. * 24.);
    let report = run(&d);
    assert_eq!(report["ok"], true, "{report}");
    assert!((report["mesh"]["volume"].as_f64().unwrap() - expected).abs() < 1e-8);
    assert_eq!(report["mesh"]["faces"], 9);
    assert_eq!(report, run(&report["document"]));
    let typed: WorkflowDocument = serde_json::from_value(d.clone()).unwrap();
    typed
        .rebuild()
        .unwrap()
        .validate(Tolerance::default())
        .unwrap();
    for x in [18., 18. + 5e-8, 0.] {
        d["operations"][2]["center"] = json!([x, 0.]);
        let failed = run(&d);
        assert_eq!(failed["diagnostic"]["code"], "bore_clearance");
        assert_eq!(failed["diagnostic"]["operation_id"], "bore-2");
        assert!(failed["diagnostic"]["message"]
            .as_str()
            .unwrap()
            .contains("bore-1"));
        assert!(failed.get("mesh").is_none());
        assert_eq!(failed["candidate_segments"].as_array().unwrap().len(), 100);
        assert!((failed["candidate_segments"][0][0][0].as_f64().unwrap() - (x + 4.)).abs() < 1e-8);
    }
    d["operations"][2]["center"] = json!([24., 0.]);
    d["operations"][2]["input"] = json!("box-1");
    assert_eq!(run(&d)["diagnostic"]["code"], "invalid_reference");
    d["operations"][2]["input"] = json!("bore-1");
    d["operations"][2]["id"] = json!("bore-1");
    assert_eq!(run(&d)["diagnostic"]["code"], "invalid_reference");
}

#[test]
fn mixed_box_bore_kernel_checks_domains_and_analytic_metrics() {
    let t = Tolerance::default();
    for scale in [0.001, 1., 1000.] {
        let b = BoxSpec {
            min: Point3::new(-40. * scale, -30. * scale, -12. * scale),
            size: Vec3::new(80. * scale, 60. * scale, 24. * scale),
        };
        let bores = [
            BoxBore {
                center: [-15. * scale, 0.],
                radius: 8. * scale,
                depth: Some(16. * scale),
            },
            BoxBore {
                center: [15. * scale, 0.],
                radius: 6. * scale,
                depth: None,
            },
        ];
        let s = subtract_box_bores(b, &bores, t).unwrap();
        s.validate(t).unwrap();
        let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
        for (point, expected) in [
            (Point3::new(-15., 0., 0.), PointLocation::Outside),
            (Point3::new(-15., 0., -8.), PointLocation::Inside),
            (Point3::new(-15., 0., -4.), PointLocation::Boundary),
            (Point3::new(15., 0., -8.), PointLocation::Outside),
            (Point3::new(0., 0., 0.), PointLocation::Inside),
        ] {
            assert_eq!(
                classify_point_in_solid(&s, point * scale, policy).unwrap(),
                expected
            );
        }
        assert_eq!(s.bounds(), make_box(b, t).unwrap().bounds());

        let expected = (115200. - std::f64::consts::PI * (64. * 16. + 36. * 24.)) * scale.powi(3);
        assert!((s.volume().unwrap() - expected).abs() < expected * 1e-12);
        let reverse = subtract_box_bores(b, &[bores[1], bores[0]], t).unwrap();
        assert!((reverse.volume().unwrap() - expected).abs() < expected * 1e-12);
        assert!(subtract_box_bores(b, &[bores[0], bores[0]], t).is_err());
        let mut bad = bores;
        bad[0].depth = Some(24. * scale);
        assert!(subtract_box_bores(b, &bad, t).is_err());
        bad = bores;
        bad[1].radius = f64::NAN;
        assert!(subtract_box_bores(b, &bad, t).is_err());
        assert!(subtract_box_bores(b, &vec![bores[0]; 257], t).is_err());
    }
}

fn mixed_document() -> WorkflowDocument {
    serde_json::from_str(include_str!("../docs/workflow-multiple-example.json")).unwrap()
}
fn assert_fresh(result: &WorkflowRebuild, document: &WorkflowDocument) {
    let fresh = document.rebuild().unwrap();
    result
        .solid
        .validate(Tolerance::new(document.tolerance.linear).unwrap())
        .unwrap();
    assert_eq!(
        result.solid.mesh_json(0.05, Tolerance::default()).unwrap(),
        fresh.mesh_json(0.05, Tolerance::default()).unwrap()
    );
}
#[test]
fn incremental_prefix_reuse_edit_append_truncate_and_policy_invalidation() {
    use std::sync::Arc;
    let mut session = WorkflowSession::new();
    let mut doc = mixed_document();
    doc.operations.pop();
    let first = session.rebuild(&doc).unwrap();
    assert_eq!(first.stats.reused_operations, 0);
    assert_eq!(first.stats.rebuilt_operation_ids, vec!["box-1", "bore-1"]);
    let same = session.rebuild(&doc).unwrap();
    assert_eq!(same.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&first.solid, &same.solid));
    doc = mixed_document();
    let appended = session.rebuild(&doc).unwrap();
    assert_eq!(appended.stats.reused_operations, 2);
    assert_eq!(appended.stats.rebuilt_operation_ids, vec!["bore-2"]);
    assert_fresh(&appended, &doc);
    doc.operations.pop();
    let truncated = session.rebuild(&doc).unwrap();
    assert_eq!(truncated.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&first.solid, &truncated.solid));
    doc = mixed_document();
    session.rebuild(&doc).unwrap();
    if let WorkflowOperation::Bore { radius, .. } = &mut doc.operations[2] {
        *radius = 5.;
    }
    let late = session.rebuild(&doc).unwrap();
    assert_eq!(late.stats.reused_operations, 2);
    assert_eq!(late.stats.rebuilt_operation_ids, vec!["bore-2"]);
    assert_fresh(&late, &doc);
    if let WorkflowOperation::Bore { depth, .. } = &mut doc.operations[1] {
        *depth = Some(12.);
    }
    let early = session.rebuild(&doc).unwrap();
    assert_eq!(early.stats.reused_operations, 1);
    assert_eq!(early.stats.rebuilt_operation_ids, vec!["bore-1", "bore-2"]);
    assert_fresh(&early, &doc);
    if let WorkflowOperation::Box { size, .. } = &mut doc.operations[0] {
        size[0] = 100.;
    }
    let stock = session.rebuild(&doc).unwrap();
    assert_eq!(stock.stats.reused_operations, 0);
    assert_eq!(stock.stats.rebuilt_operations, 3);
    assert_fresh(&stock, &doc);
    for component in ["angular", "relative", "linear"] {
        match component {
            "angular" => doc.tolerance.angular *= 2.,
            "relative" => doc.tolerance.relative *= 2.,
            _ => doc.tolerance.linear *= 2.,
        };
        let policy = session.rebuild(&doc).unwrap();
        assert_eq!(policy.stats.reused_operations, 0);
        assert_fresh(&policy, &doc);
    }
    session.reset();
    assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 3);
}

#[test]
fn incremental_failures_preserve_accepted_geometry_and_cache_ownership() {
    use std::sync::Arc;
    let mut session = WorkflowSession::new();
    let doc = mixed_document();
    let initial = session.rebuild(&doc).unwrap();
    let mut bad = doc.clone();
    if let WorkflowOperation::Bore { center, .. } = &mut bad.operations[2] {
        center[0] = 18.;
    }
    assert_eq!(session.rebuild(&bad).unwrap_err().code, "bore_clearance");
    let unchanged = session.rebuild(&doc).unwrap();
    assert_eq!(unchanged.stats.rebuilt_operations, 0);
    assert!(Arc::ptr_eq(&initial.solid, &unchanged.solid));
    bad = doc.clone();
    bad.schema_version = 99;
    assert!(session.rebuild(&bad).is_err());
    bad = doc.clone();
    bad.units = "m".into();
    assert!(session.rebuild(&bad).is_err());
    bad = doc.clone();
    if let WorkflowOperation::Bore { input, .. } = &mut bad.operations[2] {
        *input = "box-1".into();
    }
    assert_eq!(session.rebuild(&bad).unwrap_err().code, "invalid_reference");
    let mut caller = session.rebuild(&doc).unwrap();
    Arc::make_mut(&mut caller.solid).shell.faces.clear();
    assert!(caller.solid.validate(Tolerance::default()).is_err());
    let intact = session.rebuild(&doc).unwrap();
    assert!(Arc::ptr_eq(&initial.solid, &intact.solid));
    assert_fresh(&intact, &doc);
}

#[test]
fn incremental_json_is_transactional_and_stateless_reports_remain_compatible() {
    let mut session = WorkflowSession::new();
    let doc = mixed_document();
    let input = serde_json::to_string(&doc).unwrap();
    let first: Value = serde_json::from_str(&session.evaluate_json(&input).unwrap()).unwrap();
    assert_eq!(first["rebuild"]["rebuilt_operations"], 3);
    for invalid in ["{", "{}", &" ".repeat(65537)] {
        let report: Value = serde_json::from_str(&session.evaluate_json(invalid).unwrap()).unwrap();
        assert_eq!(report["ok"], false);
        assert!(report.get("rebuild").is_none());
    }
    // A valid exact solid can exceed the display sampling budget. Do not
    // commit this edit to the session merely because geometry construction passed.
    let mut oversized = doc.clone();
    oversized.operations.pop();
    oversized.tolerance.linear = 1e-3;
    if let WorkflowOperation::Box { size, .. } = &mut oversized.operations[0] {
        *size = [1e9, 1e9, 24.];
    }
    if let WorkflowOperation::Bore { radius, .. } = &mut oversized.operations[1] {
        *radius = 1e8;
    }
    oversized.rebuild().unwrap();
    let rejected: Value = serde_json::from_str(
        &session
            .evaluate_json(&serde_json::to_string(&oversized).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(rejected["diagnostic"]["code"], "display_rejected");
    assert!(rejected.get("mesh").is_none());
    assert!(rejected.get("rebuild").is_none());
    let same: Value = serde_json::from_str(&session.evaluate_json(&input).unwrap()).unwrap();
    assert_eq!(same["rebuild"]["reused_operations"], 3);
    assert_eq!(same["rebuild"]["rebuilt_operations"], 0);
    let stateless: Value = serde_json::from_str(&evaluate_workflow_json(&input).unwrap()).unwrap();
    assert_eq!(same["mesh"], stateless["mesh"]);
    assert!(stateless.get("rebuild").is_none());
}
