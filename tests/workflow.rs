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
