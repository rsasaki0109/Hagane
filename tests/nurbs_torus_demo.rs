use hagane::*;
use serde_json::Value;

#[test]
fn unresolved_optional_inertia_keeps_actual_geometry_volume_mesh_and_step() {
    let report: Value = serde_json::from_str(
        &nurbs_torus_demo_json(&[1e70, 1e69, 0., 0., 0., 0., 1e63, 1e68]).unwrap(),
    )
    .unwrap();
    assert_eq!(report["mass_properties"]["ok"], false);
    assert!(report["mass_properties"]["error"]
        .as_str()
        .unwrap()
        .contains("unresolved"));
    let expected = 2. * std::f64::consts::PI.powi(2) * 1e70 * 1e69 * 1e69;
    assert!((report["volume"].as_f64().unwrap() / expected - 1.).abs() < 1e-12);
    assert_eq!(report["brep"]["faces"], 16);
    assert!(report["mesh"]["triangles"].as_array().unwrap().len() > 32);
    assert!(report["step"].as_str().unwrap().contains("CLOSED_SHELL"));
    assert_eq!(report["step_exact"], true);
}
#[test]
fn actual_torus_demo_has_closed_rational_body_and_recovers_invalid_input() {
    for angle in [0., 0.37] {
        let v: Value = serde_json::from_str(
            &nurbs_torus_demo_json(&[16., 4., angle, 12., -5., 8., 1e-6, 0.1]).unwrap(),
        )
        .unwrap();
        assert!((v["volume"].as_f64().unwrap() - 512. * std::f64::consts::PI.powi(2)).abs() < 1e-8);
        assert_eq!(v["brep"]["faces"], 16);
        assert_eq!(v["brep"]["edges"], 32);
        assert_eq!(v["genus"], 1);
        assert_eq!(v["mass_properties"]["ok"], true);
        assert!(v["step"].as_str().unwrap().contains("CLOSED_SHELL"));
    }
    for x in [
        vec![4., 4., 0., 0., 0., 0., 1e-6, 0.1],
        vec![16., 0., 0., 0., 0., 0., 1e-6, 0.1],
        vec![16., 4., 0., 0., 0., 0., 1e-6, 0.],
        vec![16., 4., f64::NAN, 0., 0., 0., 1e-6, 0.1],
    ] {
        assert!(nurbs_torus_demo_json(&x).is_err());
    }
    assert!(nurbs_torus_demo_json(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_ok());
}
