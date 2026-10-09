use hagane::*;
use serde_json::Value;
fn payload() -> Vec<f64> {
    vec![
        80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 0., 2., 4., 0.2, 0.5, 0.3, 0.4,
        0.4, 0.5, 0.3, 0.6, 4., 0.6, 0.5, 0.7, 0.4, 0.8, 0.5, 0.7, 0.6,
    ]
}
#[test]
fn exact_multi_opening_step_payload_is_export_only() {
    let data: Value =
        serde_json::from_str(&nurbs_graph_polygon_multi_hole_step_demo_json(&payload()).unwrap())
            .unwrap();
    assert_eq!(data["units"], "mm");
    assert_eq!(data["schema"], "AUTOMOTIVE_DESIGN");
    assert_eq!(data["exact"], true);
    assert_eq!(data["import_supported"], false);
    assert_eq!(data["genus"], 2);
    assert_eq!(data["openings"].as_array().unwrap().len(), 2);
    let step = data["step"].as_str().unwrap();
    assert!(step.starts_with("ISO-10303-21;"));
    assert!(step.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
    assert!(step.contains("MANIFOLD_SOLID_BREP("));
    assert!(step.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
    assert!(step.contains("PCURVE("));
    assert_eq!(step.matches("FACE_BOUND(").count(), 4);
    assert!(step.ends_with("END-ISO-10303-21;\n"));
    assert!(data["scope"].as_str().unwrap().contains("export only"));
    assert!(data["scope"].as_str().unwrap().contains("import"));
    assert!(data.get("mesh").is_none());
}
#[test]
fn exact_export_does_not_request_display_accuracy_or_mesh_budget() {
    let mut values = payload();
    values[4] = 1e-12;
    assert!(nurbs_graph_polygon_multi_hole_demo_json(&values).is_err());
    let step: Value =
        serde_json::from_str(&nurbs_graph_polygon_multi_hole_step_demo_json(&values).unwrap())
            .unwrap();
    assert_eq!(step["exact"], true);
    let reference: Value =
        serde_json::from_str(&nurbs_graph_polygon_multi_hole_step_demo_json(&payload()).unwrap())
            .unwrap();
    assert_eq!(step["step"], reference["step"]);
}
#[test]
fn malformed_counts_touching_holes_and_nonfinite_input_reject_then_recover() {
    let values = payload();
    for (index, value) in [
        (13, 1.5),
        (14, 0.),
        (14, 5.),
        (15, 3.5),
        (0, f64::NAN),
        (4, 0.),
        (4, f64::INFINITY),
    ] {
        let mut invalid = values.clone();
        invalid[index] = value;
        assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&invalid).is_err());
    }
    for end in [0, 14, 15, 20, values.len() - 1] {
        assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&values[..end]).is_err());
    }
    let mut trailing = values.clone();
    trailing.push(0.);
    assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&trailing).is_err());
    let mut coincident = values.clone();
    coincident[25..33].copy_from_slice(&values[16..24]);
    assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&coincident).is_err());
    let mut touching = values.clone();
    for index in (25..33).step_by(2) {
        touching[index] -= 0.2;
    }
    assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&touching).is_err());
    assert!(nurbs_graph_polygon_multi_hole_step_demo_json(&values).is_ok());
}
