use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![
        80., 60., 20., 30., 1., 0., 0., 0., 0., 0., 1., 0., 1., 40., 30., 12.,
    ]
}
#[test]
fn rational_quarters_follow_circle_and_actual_placed_trimmed_roof() {
    for placed in [false, true] {
        let mut x = input();
        if placed {
            x[5] = 0.3;
            x[6] = 10.;
            x[7] = -20.;
            x[8] = 4.;
            x[9] = 0.1;
            x[10] = 0.9;
            x[11] = 0.1;
            x[12] = 0.9;
        }
        let data: Value =
            serde_json::from_str(&nurbs_graph_rational_roof_circle_demo_json(&x).unwrap()).unwrap();
        let path = &data["rational_roof_path"];
        assert_eq!(path["stock_unchanged"], true);
        assert_eq!(path["bore_created"], false);
        let quarters = path["quarters"].as_array().unwrap();
        assert_eq!(quarters.len(), 4);
        for (i, q) in quarters.iter().enumerate() {
            assert_eq!(q["curve"]["degree"], 8);
            let weights = q["curve"]["weights"].as_array().unwrap();
            assert!(weights.iter().any(|w| w != &weights[0]));
            for witness in q["witnesses"].as_array().unwrap() {
                let uv = &witness["uv"];
                let dx = 80. * uv[0].as_f64().unwrap() - 40.;
                let dy = 60. * uv[1].as_f64().unwrap() - 30.;
                assert!((dx * dx + dy * dy - 144.).abs() < 1e-9);
                for k in 0..3 {
                    assert!(
                        (witness["point"][k].as_f64().unwrap()
                            - witness["surface_point"][k].as_f64().unwrap())
                        .abs()
                            < 1e-8
                    );
                }
            }
            assert_eq!(
                q["pcurve"]["control_points"][2],
                quarters[(i + 1) % 4]["pcurve"]["control_points"][0]
            );
            let samples = q["points"].as_array().unwrap();
            assert_eq!(samples.last().unwrap(), &quarters[(i + 1) % 4]["points"][0]);
            for bound in q["error_bounds"].as_array().unwrap() {
                assert!(bound.as_f64().unwrap() <= x[4]);
            }
        }
    }
}
#[test]
fn invalid_radius_clearance_transport_and_display_precision_reject_then_recover() {
    let good = input();
    for (index, value) in [
        (15, 0.),
        (15, -1.),
        (15, 1e-14),
        (15, 30.),
        (13, 0.),
        (15, f64::NAN),
        (4, 1e-14),
    ] {
        let mut x = good.clone();
        x[index] = value;
        assert!(nurbs_graph_rational_roof_circle_demo_json(&x).is_err());
    }
    assert!(nurbs_graph_rational_roof_circle_demo_json(&good[..15]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(nurbs_graph_rational_roof_circle_demo_json(&long).is_err());
    assert!(nurbs_graph_rational_roof_circle_demo_json(&good).is_ok());
}
