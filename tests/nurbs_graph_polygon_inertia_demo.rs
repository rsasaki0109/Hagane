use hagane::*;
use serde_json::Value;

fn rectangle() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
fn polygon_demo(scale: f64, angle: f64, translation: [f64; 3]) -> Value {
    serde_json::from_str(
        &nurbs_graph_polygon_demo_json(
            8. * scale,
            6. * scale,
            2. * scale,
            0.,
            0.2 * scale,
            angle,
            translation[0],
            translation[1],
            translation[2],
            0.,
            1.,
            0.,
            1.,
            rectangle(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn hole_demo(scale: f64, angle: f64, translation: [f64; 3]) -> Value {
    let mut payload = vec![
        8. * scale,
        6. * scale,
        2. * scale,
        0.,
        0.2 * scale,
        angle,
        translation[0],
        translation[1],
        translation[2],
        0.,
        1.,
        0.,
        1.,
        0.,
        4.,
    ];
    payload.extend(
        [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]]
            .into_iter()
            .flatten(),
    );
    serde_json::from_str(&nurbs_graph_polygon_hole_demo_json(&payload).unwrap()).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 3e-11 * expected.abs().max(1.),
        "{actual} != {expected}"
    );
}
fn check_tensor(data: &Value, diagonal: [f64; 3], angle: f64) {
    let properties = &data["inertia_properties"];
    assert!(data["inertia_error"].is_null());
    assert_eq!(properties["inertia_units"], "mm5");
    assert_eq!(properties["volume_units"], "mm3");
    assert_eq!(properties["centroid_units"], "mm");
    assert_eq!(properties["reference"], "centroid");
    assert_eq!(properties["axes"], "world");
    assert_eq!(properties["density"], "uniform");
    near(
        properties["volume"].as_f64().unwrap(),
        data["mass_properties"]["volume"].as_f64().unwrap(),
    );
    for axis in 0..3 {
        near(
            properties["centroid"][axis].as_f64().unwrap(),
            data["mass_properties"]["centroid"][axis].as_f64().unwrap(),
        );
    }
    let (s, c) = angle.sin_cos();
    let expected = [
        [
            diagonal[0] * c * c + diagonal[2] * s * s,
            0.,
            (diagonal[2] - diagonal[0]) * s * c,
        ],
        [0., diagonal[1], 0.],
        [
            (diagonal[2] - diagonal[0]) * s * c,
            0.,
            diagonal[0] * s * s + diagonal[2] * c * c,
        ],
    ];
    for (row, values) in expected.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            near(properties["inertia"][row][column].as_f64().unwrap(), *value);
        }
    }
}
#[test]
fn polygon_box_reports_uniform_world_centroidal_tensor() {
    // Uniform 8 x 6 x 2 box, volume 96; analytic box inertias V*(a²+b²)/12.
    let data = polygon_demo(1., 0., [10., -4., 7.]);
    check_tensor(&data, [320., 544., 800.], 0.);
    near(data["volume"].as_f64().unwrap(), 96.);
    assert_eq!(
        data["mass_properties"]["method"],
        "polynomial fan integration"
    );
}
#[test]
fn polygon_opening_reports_retained_material_tensor() {
    // Centered 4 x 3 x 2 opening: inertias [26,40,50], same centroid.
    let data = hole_demo(1., 0., [10., -4., 7.]);
    check_tensor(&data, [294., 504., 750.], 0.);
    near(data["volume"].as_f64().unwrap(), 72.);
    assert_eq!(
        data["mass_properties"]["method"],
        "positive material-triangle polynomial integration"
    );
    assert!(!data["scope"].as_str().unwrap().contains("inertia"));
    assert!(data["scope"]
        .as_str()
        .unwrap()
        .contains("generic Booleans remain unsupported"));
}
#[test]
fn placed_polygon_and_opening_report_rotated_full_world_tensors() {
    let angle = std::f64::consts::FRAC_PI_4;
    check_tensor(
        &polygon_demo(1., angle, [10., -4., 7.]),
        [320., 544., 800.],
        angle,
    );
    check_tensor(
        &hole_demo(1., angle, [10., -4., 7.]),
        [294., 504., 750.],
        angle,
    );
}
