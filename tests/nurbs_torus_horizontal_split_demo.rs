use hagane::*;
use serde_json::Value;
#[test]
fn actual_horizontal_reports_preserve_source_and_recover_errors() {
    let mut s = NurbsTorusDemoSession::new();
    assert!(s.split_horizontal(1., 0.1).is_err());
    s.apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1]).unwrap();
    let ptr = s.accepted_body().unwrap() as *const _;
    let before = s
        .intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
        .unwrap();
    for c in [-1., 0., 1.] {
        let v: Value = serde_json::from_str(&s.split_horizontal(c, 0.1).unwrap()).unwrap();
        assert_eq!(v["source_radii"], serde_json::json!([16., 4.]));
        assert_eq!(v["cut_plane"]["point"], serde_json::json!([0., 0., c]));
        assert_eq!(v["cut_plane"]["normal"], serde_json::json!([0., 0., 1.]));
        let lower = 32.
            * std::f64::consts::PI
            * (c * (16. - c * c).sqrt() + 16. * ((c / 4.).asin() + std::f64::consts::FRAC_PI_2));
        assert!((v["lower"]["volume"].as_f64().unwrap() - lower).abs() < 1e-8);
        assert!(
            (v["lower"]["volume"].as_f64().unwrap() + v["upper"]["volume"].as_f64().unwrap()
                - 512. * std::f64::consts::PI.powi(2))
            .abs()
                < 1e-8
        );
        assert_eq!(v["lower"]["is_lower"], true);
        assert_eq!(v["upper"]["is_lower"], false);
        assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    }
    for (c, chord) in [
        (4., 0.1),
        (-4., 0.1),
        (1., 0.),
        (1., 1e-14),
        (f64::NAN, 0.1),
    ] {
        assert!(s.split_horizontal(c, chord).is_err());
        assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    }
    assert_eq!(
        s.intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
            .unwrap(),
        before
    );
}
