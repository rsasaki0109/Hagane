use hagane::*;
#[test]
fn unselected_display_budget_does_not_block_selected_part() {
    let mut x = vec![
        80., 60., 20., 30., 0.2, 0., 0., 0., 0., 0., 1., 0., 1., 0.855, 0., 0.855, 1., 0.,
    ];
    for i in 0..16 {
        let angle = std::f64::consts::TAU * i as f64 / 16.;
        x.extend([0.5 + 0.4 * angle.cos(), 0.5 + 0.4 * angle.sin()]);
    }
    let data: serde_json::Value =
        serde_json::from_str(&nurbs_graph_polygon_split_demo_json(&x).unwrap()).unwrap();
    assert_eq!(data["split"]["negative"]["brep"]["closed"], true);
    assert!(data["mesh"]["triangles"].as_array().unwrap().len() <= 65536);
    x[17] = 1.;
    assert!(matches!(
        nurbs_graph_polygon_split_demo_json(&x),
        Err(Error::Tessellation(_))
    ));
}
