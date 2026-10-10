use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![80., 60., 20., 0., 0., 0., 0., 1e-8, 2., 0., 3., 8., 5.]
}
fn report(x: &[f64]) -> Value {
    serde_json::from_str(&edge_chamfer_multi_demo_json(x).unwrap()).unwrap()
}
#[test]
fn crossing_chamfers_remove_overlap_once_and_report_final_bevels() {
    let x = input();
    let d = report(&x);
    assert!((d["removed_volume"].as_f64().unwrap() - 592.).abs() < 1e-7);
    assert!((d["kept"]["volume"].as_f64().unwrap() - 95408.).abs() < 1e-7);
    assert_eq!(d["removed"].as_array().unwrap().len(), 2);
    assert_eq!(
        d["chamfers"]["selections"],
        serde_json::json!([[0, 3.], [8, 5.]])
    );
    for patch in d["chamfers"]["bevel_patches"].as_array().unwrap() {
        for p in patch["rings"][0].as_array().unwrap() {
            let point = Point3::new(
                p[0].as_f64().unwrap(),
                p[1].as_f64().unwrap(),
                p[2].as_f64().unwrap(),
            );
            for plane in d["chamfers"]["bevel_planes"].as_array().unwrap() {
                let xyz = |key: &str| {
                    Vec3::new(
                        plane[key][0].as_f64().unwrap(),
                        plane[key][1].as_f64().unwrap(),
                        plane[key][2].as_f64().unwrap(),
                    )
                };
                let o = xyz("origin");
                let n = xyz("u").cross(xyz("v")).normalized().unwrap();
                assert!((point - Point3::new(o.x, o.y, o.z)).dot(n) < 1e-8);
            }
        }
    }
    assert_eq!(d["kept"]["brep"]["closed"], true);
    for removed in d["removed"].as_array().unwrap() {
        assert_eq!(removed["brep"]["closed"], true);
    }
    let mut reversed = x.clone();
    reversed[9..].copy_from_slice(&[8., 5., 0., 3.]);
    assert!((report(&reversed)["kept"]["volume"].as_f64().unwrap() - 95408.).abs() < 1e-7);
    assert!(d["step"].as_str().unwrap().contains("MANIFOLD_SOLID_BREP"));
}
#[test]
fn malformed_counts_duplicate_edges_contact_and_late_failures_reject_then_recover() {
    let good = input();
    for (i, v) in [
        (8, 0.),
        (8, 1.5),
        (8, 13.),
        (8, 1e300),
        (9, 0.5),
        (9, 12.),
        (9, 1e300),
        (11, 0.),
        (12, 3.),
        (12, 60.),
        (12, 0.),
        (7, 0.),
        (0, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(
            edge_chamfer_multi_demo_json(&x).is_err(),
            "index {i} value {v}"
        );
    }
    assert!(edge_chamfer_multi_demo_json(&good[..12]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(edge_chamfer_multi_demo_json(&long).is_err());
    assert_eq!(report(&good)["step_exact"], true);
}
