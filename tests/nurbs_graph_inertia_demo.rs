use hagane::*;

#[test]
fn unrepresentable_inertia_does_not_discard_representable_geometry() {
    let normal: serde_json::Value =
        serde_json::from_str(&nurbs_graph_solid_demo_json(80., 60., 20., 0., 0.2).unwrap())
            .unwrap();
    assert!(normal["inertia_properties"]["inertia"].is_array());
    assert!(normal["inertia_error"].is_null());
    let huge: serde_json::Value =
        serde_json::from_str(&nurbs_graph_solid_demo_json(1e62, 1e62, 1e62, 0., 1e61).unwrap())
            .unwrap();
    assert!(huge["inertia_properties"].is_null());
    assert!(!huge["inertia_error"].as_str().unwrap().is_empty());
    let volume = huge["mass_properties"]["volume"].as_f64().unwrap();
    assert!((volume / 1e186 - 1.).abs() < 1e-12);
    for coordinate in huge["mass_properties"]["centroid"].as_array().unwrap() {
        assert!((coordinate.as_f64().unwrap() / 5e61 - 1.).abs() < 1e-12);
    }
    assert!(!huge["mesh"]["triangles"].as_array().unwrap().is_empty());
}
