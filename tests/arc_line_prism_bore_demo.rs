use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![80., 60., 20., 8., 0., 0., 8., 0., 0., 0., 0., 1e-6, 0.1]
}
fn data(x: &[f64]) -> Value {
    serde_json::from_str(&arc_line_prism_bore_demo_json(x).unwrap()).unwrap()
}
#[test]
fn actual_bore_solids_have_closed_shared_topology_analytic_volume_and_step() {
    for posed in [false, true] {
        let mut x = input();
        if posed {
            x[7] = 0.3;
            x[8] = 10.;
            x[9] = -20.;
            x[10] = 4.;
        }
        let d = data(&x);
        let removed = std::f64::consts::PI * 64. * 20.;
        let source = (4800. - (4. - std::f64::consts::PI) * 64.) * 20.;
        assert!((d["removed_volume"].as_f64().unwrap() - removed).abs() < 1e-7);
        assert!((d["kept"]["volume"].as_f64().unwrap() - (source - removed)).abs() < 1e-6);
        assert!((d["removed"]["volume"].as_f64().unwrap() - removed).abs() < 1e-7);
        for name in ["kept", "removed"] {
            let b = &d[name]["brep"];
            assert_eq!(b["closed"], true);
            let mut edges = vec![Vec::new(); b["edges"].as_u64().unwrap() as usize];
            for face in b["wires"].as_array().unwrap() {
                for wire in face.as_array().unwrap() {
                    for c in wire.as_array().unwrap() {
                        edges[c["edge"].as_u64().unwrap() as usize]
                            .push(c["forward"].as_bool().unwrap());
                        for w in c["witnesses"].as_array().unwrap() {
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
            for uses in edges {
                assert_eq!(uses.len(), 2);
                assert_ne!(uses[0], uses[1]);
            }
            let reimport = import_step_bounded_analytic_mm(
                d[format!("{name}_step")].as_str().unwrap(),
                Tolerance::new(1e-6).unwrap(),
            )
            .unwrap();
            assert!(
                (reimport.volume().unwrap() - d[name]["volume"].as_f64().unwrap()).abs() < 1e-6
            );
        }
        assert_eq!(d["kept"]["brep"]["wires"][0].as_array().unwrap().len(), 2);
        assert_eq!(d["bore"]["hole_faces"].as_array().unwrap().len(), 4);
    }
}
#[test]
fn invalid_fit_precision_and_display_reject_then_resolved_micro_shape_recovers() {
    let good = input();
    for (i, v) in [
        (0, 0.),
        (3, 0.),
        (3, 30.),
        (6, 0.),
        (6, 30.),
        (4, 40.),
        (6, 1e-14),
        (11, 0.),
        (11, 1e-14),
        (12, 0.),
        (12, 1e-14),
        (8, 1e15),
        (0, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(
            arc_line_prism_bore_demo_json(&x).is_err(),
            "index{i}value{v}"
        );
    }
    assert!(arc_line_prism_bore_demo_json(&good[..12]).is_err());
    let mut micro = good.clone();
    for i in [0, 1, 2, 3, 4, 5, 6] {
        micro[i] *= 1e-3;
    }
    micro[11] = 1e-9;
    micro[12] = 1e-4;
    assert!(arc_line_prism_bore_demo_json(&micro).is_ok());
    assert_eq!(data(&good)["step_exact"], true);
}
