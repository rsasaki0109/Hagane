use hagane::*;
use serde_json::Value;
fn diamond(cx: f64, cy: f64, r: f64) -> Vec<[f64; 2]> {
    vec![[cx - r, cy], [cx, cy - r], [cx + r, cy], [cx, cy + r]]
}
fn payload(holes: &[Vec<[f64; 2]>], bulge: f64, angle: f64) -> Vec<f64> {
    let mut values = vec![
        80.,
        60.,
        20.,
        bulge,
        0.5,
        angle,
        0.,
        0.,
        0.,
        0.,
        1.,
        0.,
        1.,
        0.,
        holes.len() as f64,
    ];
    for hole in holes {
        values.push(hole.len() as f64);
        values.extend(hole.iter().flatten());
    }
    values
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 3e-10 * b.abs().max(1.), "{a} != {b}");
}
#[test]
fn two_and_three_holes_report_exact_closed_genus_and_flat_volume() {
    for holes in [
        vec![diamond(0.3, 0.5, 0.1), diamond(0.7, 0.5, 0.1)],
        vec![
            diamond(0.25, 0.3, 0.07),
            diamond(0.75, 0.3, 0.07),
            diamond(0.5, 0.7, 0.07),
        ],
    ] {
        let data: Value = serde_json::from_str(
            &nurbs_graph_polygon_multi_hole_demo_json(&payload(&holes, 0., 0.)).unwrap(),
        )
        .unwrap();
        assert_eq!(data["genus"], holes.len());
        assert_eq!(data["openings"].as_array().unwrap().len(), holes.len());
        assert_eq!(data["brep"]["closed"], true);
        let area_removed = if holes.len() == 2 {
            0.04
        } else {
            3. * 2. * 0.07 * 0.07
        };
        near(
            data["volume"].as_f64().unwrap(),
            80. * 60. * 20. * (1. - area_removed),
        );
        near(
            data["removed_volume"].as_f64().unwrap(),
            80. * 60. * 20. * area_removed,
        );
        assert_eq!(
            data["removed_volumes"].as_array().unwrap().len(),
            holes.len()
        );
        assert!(data["inertia_error"].is_null());
        assert_eq!(data["inertia_properties"]["inertia_units"], "mm5");
        assert!(!data["mesh"]["triangles"].as_array().unwrap().is_empty());
        assert!(data["positions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x.as_f64().unwrap().is_finite()));
        let wires = data["brep"]["wire_edges"].as_array().unwrap();
        assert_eq!(wires[0].as_array().unwrap().len(), holes.len() + 1);
        assert_eq!(wires[1].as_array().unwrap().len(), holes.len() + 1);
    }
}
#[test]
fn numeric_payload_rejects_bad_geometry_then_valid_input_recovers() {
    let holes = vec![diamond(0.3, 0.5, 0.1), diamond(0.7, 0.5, 0.1)];
    let good = payload(&holes, 30., 0.2);
    for (index, value) in [(13, 3.5), (14, 0.), (14, 5.), (15, 4.5), (0, f64::NAN)] {
        let mut bad = good.clone();
        bad[index] = value;
        assert!(nurbs_graph_polygon_multi_hole_demo_json(&bad).is_err());
    }
    let mut bad = good.clone();
    bad.push(0.);
    assert!(nurbs_graph_polygon_multi_hole_demo_json(&bad).is_err());
    let coincident = vec![holes[0].clone(), holes[0].clone()];
    assert!(nurbs_graph_polygon_multi_hole_demo_json(&payload(&coincident, 0., 0.)).is_err());
    let touching = vec![diamond(0.4, 0.5, 0.1), diamond(0.6, 0.5, 0.1)];
    assert!(nurbs_graph_polygon_multi_hole_demo_json(&payload(&touching, 0., 0.)).is_err());
    let data: Value =
        serde_json::from_str(&nurbs_graph_polygon_multi_hole_demo_json(&good).unwrap()).unwrap();
    assert_eq!(data["genus"], 2);
    assert_eq!(data["brep"]["closed"], true);
}
