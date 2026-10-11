use hagane::*;
use serde_json::Value;
#[test]
fn actual_accepted_line_ray_preserve_nonunit_parameters_and_recover() {
    let mut s = NurbsTorusDemoSession::new();
    assert!(s
        .intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
        .is_err());
    let report = s.apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1]).unwrap();
    let ptr = s.accepted_body().unwrap() as *const _;
    let line: Value = serde_json::from_str(
        &s.intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
            .unwrap(),
    )
    .unwrap();
    let expected = [-10., -6., 6., 10.];
    assert_eq!(line["hits"].as_array().unwrap().len(), 4);
    for (h, t) in line["hits"].as_array().unwrap().iter().zip(expected) {
        assert!((h["parameter"].as_f64().unwrap() - t).abs() <= 1e-6 / 32. + 64. * f64::EPSILON);
    }
    let ray: Value = serde_json::from_str(
        &s.intersect_ray(Point3::new(16., 0., 0.), Vec3::new(2., 0., 0.))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ray["query_kind"], "ray");
    assert_eq!(ray["material_intervals"][0][0], 0.);
    assert!((ray["material_intervals"][0][1].as_f64().unwrap() - 2.).abs() < 1e-6 / 32.);
    for (o, d) in [
        (Point3::new(0., 0., 0.), Vec3::new(0., 0., 0.)),
        (Point3::new(0., 0., 4.), Vec3::new(2., 0., 0.)),
        (Point3::new(1e100, 0., 0.), Vec3::new(2., 0., 0.)),
    ] {
        assert!(s.intersect_line(o, d).is_err());
        assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    }
    assert!(s
        .intersect_ray(Point3::new(20., 0., 0.), Vec3::new(2., 0., 0.))
        .is_err());
    assert_eq!(s.accepted_body().unwrap() as *const _, ptr);
    assert_eq!(
        s.apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1]).unwrap(),
        report
    );
}
