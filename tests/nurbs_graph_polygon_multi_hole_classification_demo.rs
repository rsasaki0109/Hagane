use hagane::*;
use serde_json::Value;
fn model() -> Vec<f64> {
    vec![
        80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 0., 2., 4., 0.2, 0.5, 0.3, 0.4,
        0.4, 0.5, 0.3, 0.6, 4., 0.6, 0.5, 0.7, 0.4, 0.8, 0.5, 0.7, 0.6,
    ]
}
fn query(mut model: Vec<f64>, point: [f64; 3], tolerance: f64) -> Result<String> {
    model.extend(point);
    model.push(tolerance);
    nurbs_graph_polygon_multi_hole_point_demo_json(&model)
}
fn parsed(model: Vec<f64>, point: [f64; 3]) -> Value {
    serde_json::from_str(&query(model, point, 1e-8).unwrap()).unwrap()
}
#[test]
fn material_each_opening_and_actual_boundaries_report_checked_locations() {
    for (point, location) in [
        ([40., 30., 10.], "Inside"),
        ([24., 30., 10.], "Outside"),
        ([56., 30., 10.], "Outside"),
        ([16., 30., 10.], "Boundary"),
        ([0., 0., 10.], "Boundary"),
        ([0., 30., 10.], "Boundary"),
        ([40., 30., -1.], "Outside"),
        ([40., 30., 40.], "Outside"),
    ] {
        let data = parsed(model(), point);
        assert_eq!(data["location"], location, "{point:?}");
        assert_eq!(data["point"], serde_json::json!(point));
        assert_eq!(data["source_point"], serde_json::json!(point));
        assert_eq!(data["linear_tolerance"], 1e-8);
        assert_eq!(data["relative_tolerance"], 0.);
        assert_eq!(data["genus"], 2);
        assert!(!data["reason"].as_str().unwrap().is_empty());
        assert!(data.get("mesh").is_none());
    }
}
#[test]
fn exact_model_query_succeeds_without_display_resources() {
    let mut values = model();
    values[4] = 1e-12;
    assert!(nurbs_graph_polygon_multi_hole_demo_json(&values).is_err());
    let data = parsed(values, [40., 30., 10.]);
    assert_eq!(data["location"], "Inside");
}
#[test]
fn placed_query_preserves_world_point_and_recovers_source_coordinates() {
    let mut values = model();
    values[5] = 0.4;
    values[6] = 100.;
    values[7] = -30.;
    values[8] = 12.;
    let transform = Transform::translation(Point3::new(100., -30., 12.))
        .unwrap()
        .compose(Transform::rotation(Point3::new(0., 1., 0.), 0.4).unwrap())
        .unwrap();
    let source = Point3::new(40., 30., 10.);
    let world = transform.point(source);
    let point = [world.x, world.y, world.z];
    let data = parsed(values, point);
    assert_eq!(data["location"], "Inside");
    assert_eq!(data["point"], serde_json::json!(point));
    for (i, x) in [source.x, source.y, source.z].into_iter().enumerate() {
        assert!((data["source_point"][i].as_f64().unwrap() - x).abs() < 1e-12);
    }
}
#[test]
fn invalid_tolerance_counts_contact_and_transport_reject_then_recover() {
    for tolerance in [0., -1., f64::NAN, f64::INFINITY, 1e-14] {
        assert!(query(model(), [40., 30., 10.], tolerance).is_err());
    }
    for (index, value) in [
        (13, 2.),
        (13, 3.5),
        (14, 0.),
        (14, 5.),
        (15, 4.5),
        (0, f64::NAN),
    ] {
        let mut values = model();
        values[index] = value;
        assert!(query(values, [40., 30., 10.], 1e-8).is_err());
    }
    let mut trailing = model();
    trailing.push(0.);
    assert!(query(trailing, [40., 30., 10.], 1e-8).is_err());
    let mut missing = model();
    missing.pop();
    assert!(query(missing, [40., 30., 10.], 1e-8).is_err());
    let mut touching = model();
    for i in (25..33).step_by(2) {
        touching[i] -= 0.2;
    }
    assert!(query(touching, [40., 30., 10.], 1e-8).is_err());
    assert!(query(model(), [f64::NAN, 30., 10.], 1e-8).is_err());
    assert!(nurbs_graph_polygon_multi_hole_point_demo_json(&vec![0.; 152]).is_err());
    assert!(nurbs_graph_polygon_multi_hole_point_demo_json(&[0.; 25]).is_err());
    assert_eq!(parsed(model(), [40., 30., 10.])["location"], "Inside");
}
