use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![
        80., 60., 20., 8., 0., 0., 8., 0., 0.37, 0., 0., 0., 0., 1e-6, 0.1,
    ]
}
fn data(x: &[f64]) -> Value {
    serde_json::from_str(&arc_line_prism_split_components_demo_json(x).unwrap()).unwrap()
}
#[test]
fn partition_across_opening_retains_actual_closed_children_and_section_intervals() {
    for (offset, pose) in [(0., false), (3., false), (3., true)] {
        let mut x = input();
        x[7] = offset;
        if pose {
            x[9] = 0.4;
            x[10] = 15.;
            x[11] = -10.;
            x[12] = 7.;
        }
        let d = data(&x);
        let mut total = 0.;
        let mut counts = [0, 0];
        for (side_index, side) in ["negative", "positive"].iter().enumerate() {
            let bodies = d[*side].as_array().unwrap();
            counts[side_index] = bodies.len();
            for (i, body) in bodies.iter().enumerate() {
                assert_eq!(body["brep"]["closed"], true);
                total += body["volume"].as_f64().unwrap();
                let b = &body["brep"];
                let mut incidence = vec![Vec::new(); b["edges"].as_u64().unwrap() as usize];
                for f in b["wires"].as_array().unwrap() {
                    for w in f.as_array().unwrap() {
                        for c in w.as_array().unwrap() {
                            incidence[c["edge"].as_u64().unwrap() as usize]
                                .push(c["forward"].as_bool().unwrap());
                        }
                    }
                }
                for uses in incidence {
                    assert_eq!(uses.len(), 2);
                    assert_ne!(uses[0], uses[1]);
                }
                let imported = import_step_bounded_analytic_mm(
                    d[format!("{side}_steps")][i].as_str().unwrap(),
                    Tolerance::new(1e-6).unwrap(),
                )
                .unwrap();
                assert!(
                    (imported.volume().unwrap() - body["volume"].as_f64().unwrap()).abs() < 1e-6
                );
            }
        }
        assert!(counts.iter().all(|n| *n > 0));
        assert!((total - d["source"]["volume"].as_f64().unwrap()).abs() < 1e-6);
        assert_eq!(d["cut"]["sections"].as_array().unwrap().len(), 2);
        if offset == 0. {
            assert!((d["negative"][0]["volume"].as_f64().unwrap() - total / 2.).abs() < 1e-6);
        }
    }
}
#[test]
fn invalid_nonfinite_outside_contact_and_precision_reject_then_recover() {
    let good = input();
    for (i, v) in [
        (0, 0.),
        (3, 0.),
        (6, 0.),
        (6, 30.),
        (7, 50.),
        (13, 0.),
        (13, 1e-14),
        (14, 0.),
        (14, 1e-14),
        (10, 1e15),
        (7, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(
            arc_line_prism_split_components_demo_json(&x).is_err(),
            "index{i}value{v}"
        );
    }
    assert!(arc_line_prism_split_components_demo_json(&good[..14]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(arc_line_prism_split_components_demo_json(&long).is_err());
    assert_eq!(data(&good)["step_exact"], true);
}
