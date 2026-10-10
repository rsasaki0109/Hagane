use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![80., 60., 20., 8., 0., 0., 0., 0., 0., 0., 1e-6, 0.1]
}
fn data(x: &[f64]) -> Value {
    serde_json::from_str(&arc_line_prism_split_demo_json(x).unwrap()).unwrap()
}
#[test]
fn actual_children_are_closed_on_correct_halfspaces_and_roundtrip() {
    for (offset, angle, pose) in [(0., 0., false), (34., 0., false), (0., 0.3, true)] {
        let mut x = input();
        x[4] = offset;
        x[5] = angle;
        if pose {
            x[6] = 0.4;
            x[7] = 10.;
            x[8] = -20.;
            x[9] = 4.;
        }
        let d = data(&x);
        let source = d["source"]["volume"].as_f64().unwrap();
        assert!(
            (d["negative"]["volume"].as_f64().unwrap() + d["positive"]["volume"].as_f64().unwrap()
                - source)
                .abs()
                < 1e-6
        );
        if offset == 0. {
            assert!((d["negative"]["volume"].as_f64().unwrap() - source / 2.).abs() < 1e-6);
        }
        let xyz = |v: &Value| {
            Point3::new(
                v[0].as_f64().unwrap(),
                v[1].as_f64().unwrap(),
                v[2].as_f64().unwrap(),
            )
        };
        let origin = xyz(&d["cut"]["plane"]["origin"]);
        let normal = xyz(&d["cut"]["normal"]);
        for (name, sign) in [("negative", -1.), ("positive", 1.)] {
            let b = &d[name]["brep"];
            assert_eq!(b["closed"], true);
            let mut incidence = vec![Vec::new(); b["edges"].as_u64().unwrap() as usize];
            for p in b["vertices"].as_array().unwrap() {
                assert!((xyz(p) - origin).dot(normal) * sign >= -1e-6);
            }
            for face in b["wires"].as_array().unwrap() {
                for wire in face.as_array().unwrap() {
                    for c in wire.as_array().unwrap() {
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
                d[format!("{name}_step")].as_str().unwrap(),
                Tolerance::new(1e-6).unwrap(),
            )
            .unwrap();
            assert!(
                (imported.volume().unwrap() - d[name]["volume"].as_f64().unwrap()).abs() < 1e-6
            );
        }
        assert_eq!(d["cut"]["section"]["rings"][0].as_array().unwrap().len(), 4);
    }
}
#[test]
fn invalid_outside_contact_conditioning_and_display_requests_reject_then_recover() {
    let good = input();
    for (i, v) in [
        (0, 0.),
        (3, 0.),
        (3, 30.),
        (4, 40.),
        (4, 50.),
        (10, 0.),
        (10, 1e-14),
        (11, 0.),
        (11, 1e-14),
        (7, 1e15),
        (4, f64::NAN),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(
            arc_line_prism_split_demo_json(&x).is_err(),
            "index{i}value{v}"
        );
    }
    assert!(arc_line_prism_split_demo_json(&good[..11]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(arc_line_prism_split_demo_json(&long).is_err());
    assert_eq!(data(&good)["step_exact"], true);
}
