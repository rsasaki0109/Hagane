use hagane::*;
#[test]
fn demo_retains_open_brep_and_bounded_polygon_mesh_in_all_modes() {
    for mode in 0..3 {
        let text = nurbs_polygon_demo_json(
            35.,
            1.2,
            0.5,
            mode,
            vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]],
        )
        .unwrap();
        let data: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(data["brep"]["closed"], false);
        assert_eq!(data["brep"]["edges"], 3);
        assert_eq!(
            data["brep"]["edge_vertices"],
            serde_json::json!([[0, 1], [1, 2], [2, 0]])
        );
        let triangles = data["mesh"]["triangles"].as_array().unwrap().len();
        assert_eq!(data["positions"].as_array().unwrap().len(), triangles * 9);
        assert_eq!(data["error_bounds"].as_array().unwrap().len(), triangles);
        assert!(data["error_bounds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x.as_f64().unwrap() <= 0.5));
        assert_eq!(
            data["vertex_nodes"].as_array().unwrap().len(),
            data["vertex_uv"].as_array().unwrap().len()
        );
    }
}
#[test]
fn demo_rejects_invalid_parameters_and_boundaries() {
    let corners = vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]];
    for (height, weight, error, mode) in [
        (f64::NAN, 1., 0.5, 0),
        (101., 1., 0.5, 0),
        (35., 0., 0.5, 0),
        (35., 1., 0., 0),
        (35., 1., 0.5, 3),
        (35., 1., 1e-20, 2),
    ] {
        assert!(nurbs_polygon_demo_json(height, weight, error, mode, corners.clone()).is_err());
    }
    assert!(
        nurbs_polygon_demo_json(35., 1., 0.5, 0, vec![[0.1, 0.1], [0.2, 0.2], [0.3, 0.3]]).is_err()
    );
}
