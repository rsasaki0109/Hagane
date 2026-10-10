use hagane::*;
use serde_json::Value;
fn all() -> Vec<f64> {
    let mut x = vec![80., 60., 20., 0., 0., 0., 0., 1e-6, 12.];
    for edge in 0..12 {
        x.extend([edge as f64, 3.]);
    }
    x
}
fn data(x: &[f64]) -> Value {
    serde_json::from_str(&edge_chamfer_contact_demo_json(x).unwrap()).unwrap()
}
#[test]
fn all_edges_and_equal_adjacent_contacts_report_actual_closed_geometry() {
    let x = all();
    let d = data(&x);
    assert_eq!(d["mode"], "vertex_contacts");
    assert!((d["kept"]["volume"].as_f64().unwrap() - 93282.).abs() < 1e-6);
    assert!((d["removed_volume"].as_f64().unwrap() - 2718.).abs() < 1e-6);
    assert_eq!(d["kept"]["brep"]["vertices"].as_array().unwrap().len(), 32);
    assert_eq!(d["kept"]["brep"]["edges"], 48);
    assert_eq!(d["kept"]["brep"]["faces"], 18);
    assert_eq!(d["chamfers"]["bevel_patches"].as_array().unwrap().len(), 12);
    let pair = vec![80., 60., 20., 0., 0., 0., 0., 1e-6, 2., 0., 3., 8., 3.];
    assert!(edge_chamfer_multi_demo_json(&pair).is_err());
    assert_eq!(data(&pair)["kept"]["brep"]["closed"], true);
    let mut posed = x.clone();
    posed[3] = 0.3;
    posed[4] = 10.;
    posed[5] = -20.;
    posed[6] = 4.;
    assert!((data(&posed)["kept"]["volume"].as_f64().unwrap() - 93282.).abs() < 1e-6);
    assert!(d["step"].as_str().unwrap().contains("MANIFOLD_SOLID_BREP"));
}
#[test]
fn invalid_count_contact_conditioning_and_model_reject_then_recover() {
    let good = all();
    for (i, v) in [
        (8, 0.),
        (8, 1.5),
        (8, 13.),
        (8, 1e300),
        (9, 0.5),
        (9, 12.),
        (11, 0.),
        (10, 0.),
        (10, 60.),
        (7, 0.),
        (7, 1e-14),
        (4, 1e15),
        (0, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(
            edge_chamfer_contact_demo_json(&x).is_err(),
            "index{i}value{v}"
        );
    }
    assert!(edge_chamfer_contact_demo_json(&good[..32]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(edge_chamfer_contact_demo_json(&long).is_err());
    assert_eq!(data(&good)["step_exact"], true);
}
