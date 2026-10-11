use hagane::*;
use serde_json::Value;
#[test]
fn segment_reports_actual_roots_and_keeps_accepted_body_after_failures() {
    let mut s = NurbsTorusDemoSession::new();
    assert!(s
        .intersect_segment(Point3::new(-24., 0., 0.), Point3::new(24., 0., 0.))
        .is_err());
    let x = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    let report = s.apply(&x).unwrap();
    let ptr = s.accepted_body().unwrap() as *const _;
    let v: Value = serde_json::from_str(
        &s.intersect_segment(Point3::new(-24., 0., 0.), Point3::new(24., 0., 0.))
            .unwrap(),
    )
    .unwrap();
    let expected = [1. / 12., 1. / 4., 3. / 4., 11. / 12.];
    for (hit, t) in v["hits"].as_array().unwrap().iter().zip(expected) {
        assert!((hit["parameter"].as_f64().unwrap() - t).abs() <= 1e-6 / (16. * 48.));
    }
    assert_eq!(v["material_intervals"].as_array().unwrap().len(), 2);
    assert_eq!(v["hits"].as_array().unwrap().len(), 4);
    for (a, b) in [
        (Point3::new(20., 0., 0.), Point3::new(24., 0., 0.)),
        (Point3::new(-24., 0., 4.), Point3::new(24., 0., 4.)),
        (Point3::new(f64::NAN, 0., 0.), Point3::new(24., 0., 0.)),
    ] {
        assert!(s.intersect_segment(a, b).is_err());
        assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    }
    assert!(s.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    assert_eq!(s.apply(&x).unwrap(), report);
}
