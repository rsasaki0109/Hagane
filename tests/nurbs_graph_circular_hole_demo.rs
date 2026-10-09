use hagane::*;
use serde_json::Value;
fn input() -> Vec<f64> {
    vec![
        80., 60., 20., 30., 1., 0., 0., 0., 0., 0., 1., 0., 1., 40., 30., 12.,
    ]
}
#[test]
fn actual_circular_bore_has_closed_topology_and_analytic_volume() {
    for (bulge, angle) in [(0., 0.), (30., 0.), (-12., 0.), (30., 0.3)] {
        let mut x = input();
        x[3] = bulge;
        x[5] = angle;
        if angle != 0. {
            x[6] = 10.;
            x[7] = -20.;
            x[8] = 4.;
        }
        let data: Value =
            serde_json::from_str(&nurbs_graph_circular_hole_demo_json(&x).unwrap()).unwrap();
        assert_eq!(data["brep"]["closed"], true);
        assert_eq!(data["brep"]["faces"], 10);
        assert_eq!(data["brep"]["edges"], 24);
        assert_eq!(data["brep"]["vertices"].as_array().unwrap().len(), 16);
        for f in 0..2 {
            assert_eq!(data["brep"]["wire_edges"][f].as_array().unwrap().len(), 2);
            assert_eq!(
                data["brep"]["wire_edges"][f][1].as_array().unwrap().len(),
                4
            );
            for edge in data["brep"]["wire_edges"][f][1].as_array().unwrap() {
                let meta = &data["brep"]["curves"][edge.as_u64().unwrap() as usize];
                let curve = NurbsCurve::new(
                    meta["degree"].as_u64().unwrap() as usize,
                    meta["knots"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect(),
                    meta["control_points"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| {
                            Point3::new(
                                p[0].as_f64().unwrap(),
                                p[1].as_f64().unwrap(),
                                p[2].as_f64().unwrap(),
                            )
                        })
                        .collect(),
                    meta["weights"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect(),
                )
                .unwrap();
                for j in 0..=16 {
                    let p = curve.evaluate(j as f64 / 16.).unwrap();
                    let shifted = p - Point3::new(x[6], x[7], x[8]);
                    let p = Point3::new(
                        angle.cos() * shifted.x - angle.sin() * shifted.z,
                        shifted.y,
                        angle.sin() * shifted.x + angle.cos() * shifted.z,
                    );
                    assert!(((p.x - 40.).powi(2) + (p.y - 30.).powi(2) - 144.).abs() < 1e-8);
                }
            }
        }
        let r = 12_f64;
        let disk = std::f64::consts::PI * r * r;
        let integral = disk
            * (0.0625 - r * r / (16. * 80_f64.powi(2)) - r * r / (16. * 60_f64.powi(2))
                + r.powi(4) / (24. * 80_f64.powi(2) * 60_f64.powi(2)));
        let expected = disk * 20. + 4. * bulge * integral;
        assert!((data["removed_volume"].as_f64().unwrap() - expected).abs() < 1e-7);
        assert!(data["inertia_error"].is_null());
        assert!(!data["mesh"]["triangles"].as_array().unwrap().is_empty());
    }
}
#[test]
fn circular_bore_rejects_contact_unresolved_radius_and_display_failure_then_recovers() {
    let good = input();
    for (i, v) in [
        (15, 0.),
        (15, -1.),
        (15, 30.),
        (15, 1e-14),
        (13, 0.),
        (15, f64::NAN),
        (4, 1e-14),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(nurbs_graph_circular_hole_demo_json(&x).is_err());
    }
    assert!(nurbs_graph_circular_hole_demo_json(&good[..15]).is_err());
    assert!(nurbs_graph_circular_hole_demo_json(&good).is_ok());
}
