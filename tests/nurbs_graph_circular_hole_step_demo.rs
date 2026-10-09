use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![
        80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 40., 30., 12.,
    ]
}
#[test]
fn exact_circular_step_has_actual_rational_geometry_and_export_only_metadata() {
    for posed in [false, true] {
        let mut x = input();
        if posed {
            x[5] = 0.3;
            x[6] = 10.;
            x[7] = -20.;
            x[8] = 4.;
            x[9] = 0.1;
            x[10] = 0.9;
            x[11] = 0.1;
            x[12] = 0.9;
        }
        let data: Value =
            serde_json::from_str(&nurbs_graph_circular_hole_step_demo_json(&x).unwrap()).unwrap();
        assert_eq!(data["exact"], true);
        assert_eq!(data["units"], "mm");
        assert_eq!(data["application_protocol"], "AP214");
        assert_eq!(data["import_supported"], false);
        assert!(data.get("mesh").is_none());
        assert!(data["volume"].as_f64().unwrap() > 0.);
        assert!(data["removed_volume"].as_f64().unwrap() > 0.);
        let step = data["step"].as_str().unwrap();
        assert!(step.starts_with("ISO-10303-21;"));
        assert!(step.ends_with("END-ISO-10303-21;\n"));
        assert!(step.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
        assert_eq!(step.matches("ADVANCED_FACE(").count(), 10);
        assert_eq!(step.matches("EDGE_CURVE(").count(), 24);
        assert_eq!(step.matches("VERTEX_POINT(").count(), 16);
        assert_eq!(step.matches("FACE_BOUND(").count(), 2);
        assert!(step.contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(step.contains("RATIONAL_B_SPLINE_SURFACE"));
    }
}
#[test]
fn model_only_export_ignores_mesh_budget_and_rejects_bad_models_then_recovers() {
    let good = input();
    let reference: Value =
        serde_json::from_str(&nurbs_graph_circular_hole_step_demo_json(&good).unwrap()).unwrap();
    let mut tiny = good.clone();
    tiny[4] = 1e-12;
    assert!(nurbs_graph_circular_hole_demo_json(&tiny).is_err());
    let exported: Value =
        serde_json::from_str(&nurbs_graph_circular_hole_step_demo_json(&tiny).unwrap()).unwrap();
    assert_eq!(exported["step"], reference["step"]);
    for (i, v) in [
        (15, 0.),
        (15, -1.),
        (15, 30.),
        (15, 1e-14),
        (15, f64::NAN),
        (4, 0.),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(nurbs_graph_circular_hole_step_demo_json(&x).is_err());
    }
    assert!(nurbs_graph_circular_hole_step_demo_json(&good[..15]).is_err());
    assert!(nurbs_graph_circular_hole_step_demo_json(&good).is_ok());
}
