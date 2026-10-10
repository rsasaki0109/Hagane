use hagane::*;
use serde_json::Value;
fn input(edge: usize) -> Vec<f64> {
    vec![80., 60., 20., edge as f64, 3., 0., 0., 0., 0., 1e-8]
}
#[test]
fn actual_kept_removed_geometry_has_equal_setback_and_wedge_volume() {
    for edge in [0, 4, 8] {
        let x = input(edge);
        let data: Value = serde_json::from_str(&edge_chamfer_demo_json(&x).unwrap()).unwrap();
        let pair = data["source"]["brep"]["edge_vertices"][edge]
            .as_array()
            .unwrap();
        let points = &data["source"]["brep"]["vertices"];
        let point = |id: usize| {
            Point3::new(
                points[id][0].as_f64().unwrap(),
                points[id][1].as_f64().unwrap(),
                points[id][2].as_f64().unwrap(),
            )
        };
        let a = point(pair[0].as_u64().unwrap() as usize);
        let b = point(pair[1].as_u64().unwrap() as usize);
        let axis = (b - a).normalized().unwrap();
        let length = (b - a).norm();
        assert!((data["removed"]["volume"].as_f64().unwrap() - 0.5 * 9. * length).abs() < 1e-7);
        assert!(
            (data["kept"]["volume"].as_f64().unwrap()
                + data["removed"]["volume"].as_f64().unwrap()
                - 96000.)
                .abs()
                < 1e-7
        );
        for p in data["chamfer"]["bevel_patch"]["rings"][0]
            .as_array()
            .unwrap()
        {
            let delta = Point3::new(
                p[0].as_f64().unwrap(),
                p[1].as_f64().unwrap(),
                p[2].as_f64().unwrap(),
            ) - a;
            assert!(((delta - axis * delta.dot(axis)).norm() - 3.).abs() < 1e-8);
        }
        assert_eq!(data["kept"]["brep"]["closed"], true);
        assert_eq!(data["removed"]["brep"]["closed"], true);
        assert!(!data["kept"]["mesh"]["triangles"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(data["step"]
            .as_str()
            .unwrap()
            .contains("MANIFOLD_SOLID_BREP"));
        assert_eq!(data["step_exact"], true);
    }
}
#[test]
fn rigid_pose_preserves_volume_and_invalid_inputs_recover() {
    let good = input(0);
    let mut posed = good.clone();
    posed[5] = 0.3;
    posed[6] = 10.;
    posed[7] = -20.;
    posed[8] = 4.;
    let data: Value = serde_json::from_str(&edge_chamfer_demo_json(&posed).unwrap()).unwrap();
    let reference: Value = serde_json::from_str(&edge_chamfer_demo_json(&good).unwrap()).unwrap();
    assert!(
        (data["kept"]["volume"].as_f64().unwrap() - reference["kept"]["volume"].as_f64().unwrap())
            .abs()
            < 1e-7
    );
    for (i, v) in [
        (0, 0.),
        (3, -1.),
        (3, 0.5),
        (3, 12.),
        (3, 1e300),
        (4, 0.),
        (4, 20.),
        (4, 1e-14),
        (9, 0.),
        (9, 1e-14),
        (6, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(edge_chamfer_demo_json(&x).is_err());
    }
    assert!(edge_chamfer_demo_json(&good[..9]).is_err());
    assert!(edge_chamfer_demo_json(&good).is_ok());
}
