use hagane::*;
use serde_json::Value;
#[test]
fn cached_child_line_ray_keep_actual_boundaries_original_parameters_and_state() {
    let mut s = NurbsTorusDemoSession::new();
    let x = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&x).unwrap();
    assert!(s
        .intersect_horizontal_line(true, Point3::new(16., 0., 0.), Vec3::new(0., 0., 2.))
        .is_err());
    s.split_horizontal(1., 0.1).unwrap();
    let ptr = s.accepted_horizontal_split().unwrap() as *const _;
    let before = s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .unwrap();
    for (lower, interval) in [(true, [-2., 0.5]), (false, [0.5, 2.])] {
        let v: Value = serde_json::from_str(
            &s.intersect_horizontal_line(lower, Point3::new(16., 0., 0.), Vec3::new(0., 0., 2.))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(v["query_kind"], "line");
        assert_eq!(v["direction"], serde_json::json!([0., 0., 2.]));
        assert_eq!(v["hits"].as_array().unwrap().len(), 2);
        for (j, t) in interval.iter().enumerate() {
            assert!(
                (v["material_intervals"][0][j].as_f64().unwrap() - t).abs()
                    <= 1e-6 / 32. + 64. * f64::EPSILON
            );
        }
        for hit in v["hits"].as_array().unwrap() {
            let b = &hit["boundary"];
            let pair = s.accepted_horizontal_split().unwrap();
            let body = if lower { &pair.lower } else { &pair.upper };
            let face = b["face"].as_u64().unwrap() as usize;
            let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
            let point = Point3::new(
                hit["point"][0].as_f64().unwrap(),
                hit["point"][1].as_f64().unwrap(),
                hit["point"][2].as_f64().unwrap(),
            );
            assert!(
                (body.solid().shell.faces[face]
                    .surface
                    .try_evaluate(uv[0], uv[1])
                    .unwrap()
                    - point)
                    .norm()
                    <= 1e-6
            );
        }
    }
    let ray: Value = serde_json::from_str(
        &s.intersect_horizontal_ray(true, Point3::new(16., 0., 0.), Vec3::new(0., 0., 2.))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ray["query_kind"], "ray");
    assert_eq!(ray["material_intervals"][0][0], 0.);
    assert!((ray["material_intervals"][0][1].as_f64().unwrap() - 0.5).abs() < 1e-6 / 32.);
    for (o, d) in [
        (Point3::new(16., 0., 0.), Vec3::new(0., 0., 0.)),
        (Point3::new(1e100, 0., 0.), Vec3::new(0., 0., 2.)),
        (Point3::new(f64::NAN, 0., 0.), Vec3::new(0., 0., 2.)),
    ] {
        assert!(s.intersect_horizontal_line(true, o, d).is_err());
        assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
    }
    assert!(s
        .intersect_horizontal_ray(true, Point3::new(16., 0., 1.), Vec3::new(0., 0., 2.))
        .is_err());
    assert!(s.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
    assert_eq!(
        s.query_horizontal_point(true, Point3::new(16., 0., 0.))
            .unwrap(),
        before
    );
    s.apply(&x).unwrap();
    assert!(s
        .intersect_horizontal_ray(true, Point3::new(16., 0., 0.), Vec3::new(0., 0., 2.))
        .is_err());
}
