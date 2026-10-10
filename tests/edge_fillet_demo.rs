use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![
        80., 60., 20., 0., 0., 0., 0., 1e-6, 0.1, 4., 8., 3., 9., 3., 10., 3., 11., 3.,
    ]
}
fn data(x: &[f64]) -> Value {
    serde_json::from_str(&edge_fillet_demo_json(x).unwrap()).unwrap()
}
#[test]
fn real_cylindrical_fillets_preserve_tangency_topology_and_analytic_volume() {
    for posed in [false, true] {
        let mut x = input();
        if posed {
            x[3] = 0.3;
            x[4] = 10.;
            x[5] = -20.;
            x[6] = 4.;
        }
        let d = data(&x);
        let expected = (4. - std::f64::consts::PI) * 9. * 20.;
        assert!((d["removed_volume"].as_f64().unwrap() - expected).abs() < 1e-8);
        assert!((d["kept"]["volume"].as_f64().unwrap() - (96000. - expected)).abs() < 1e-6);
        assert_eq!(d["kept"]["brep"]["vertices"].as_array().unwrap().len(), 16);
        assert_eq!(d["kept"]["brep"]["edges"], 24);
        assert_eq!(d["kept"]["brep"]["faces"], 10);
        assert_eq!(d["fillets"]["faces"].as_array().unwrap().len(), 4);
        for witness in d["kept"]["brep"]["tangency_witnesses"].as_array().unwrap() {
            assert!((witness["normal_dot"].as_f64().unwrap() - 1.).abs() < 1e-12);
        }
        for face in d["kept"]["brep"]["wires"].as_array().unwrap() {
            for wire in face.as_array().unwrap() {
                for coedge in wire.as_array().unwrap() {
                    for w in coedge["witnesses"].as_array().unwrap() {
                        for k in 0..3 {
                            assert!(
                                (w["point"][k].as_f64().unwrap()
                                    - w["surface_point"][k].as_f64().unwrap())
                                .abs()
                                    < 1e-6
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(
            d["kept"]["error_bounds"].as_array().unwrap().len(),
            d["kept"]["mesh"]["triangles"].as_array().unwrap().len()
        );
        for bound in d["kept"]["error_bounds"].as_array().unwrap() {
            assert!(bound.as_f64().unwrap() <= x[8]);
        }
        assert!(d.get("removed").is_none());
        assert_eq!(d["step_exact"], true);
        assert!(d["step"].as_str().unwrap().contains("CYLINDRICAL_SURFACE"));
    }
}
#[test]
fn mixed_axes_contact_counts_precision_and_display_errors_reject_then_recover() {
    let good = input();
    for (i, v) in [
        (9, 0.),
        (9, 1.5),
        (9, 5.),
        (9, 1e300),
        (10, 0.),
        (10, 8.5),
        (10, 12.),
        (12, 8.),
        (11, 0.),
        (11, 60.),
        (7, 0.),
        (7, 1e-14),
        (8, 0.),
        (8, 1e-14),
        (4, 1e15),
        (0, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(edge_fillet_demo_json(&x).is_err(), "index{i}value{v}");
    }
    assert!(edge_fillet_demo_json(&good[..17]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(edge_fillet_demo_json(&long).is_err());
    assert_eq!(data(&good)["step_exact"], true);
}
