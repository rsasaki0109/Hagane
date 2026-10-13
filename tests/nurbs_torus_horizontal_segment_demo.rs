use hagane::*;
use serde_json::Value;
#[test]
fn actual_child_segments_report_retained_cap_uv_and_preserve_cached_pair() {
    let mut s = NurbsTorusDemoSession::new();
    let x = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&x).unwrap();
    assert!(s
        .intersect_horizontal_segment(true, Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .is_err());
    s.split_horizontal(1., 0.1).unwrap();
    let ptr = s.accepted_horizontal_split().unwrap() as *const _;
    let query_before = s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .unwrap();
    for (lower, expected) in [(true, [0.25, 0.5625]), (false, [0.5625, 0.75])] {
        let v: Value = serde_json::from_str(
            &s.intersect_horizontal_segment(
                lower,
                Point3::new(16., 0., -8.),
                Point3::new(16., 0., 8.),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["hits"].as_array().unwrap().len(), 2);
        assert_eq!(v["material_intervals"].as_array().unwrap().len(), 1);
        for (j, t) in expected.iter().enumerate() {
            assert!(
                (v["material_intervals"][0][j].as_f64().unwrap() - t).abs()
                    <= 1e-6 / (16. * 16.) + 64. * f64::EPSILON
            );
        }
        for hit in v["hits"].as_array().unwrap() {
            let b = &hit["boundary"];
            let f = b["face"].as_u64().unwrap() as usize;
            let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
            let pair = s.accepted_horizontal_split().unwrap();
            let body = if lower { &pair.lower } else { &pair.upper };
            let actual = body.solid().shell.faces[f]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            let point = Point3::new(
                hit["point"][0].as_f64().unwrap(),
                hit["point"][1].as_f64().unwrap(),
                hit["point"][2].as_f64().unwrap(),
            );
            assert!((actual - point).norm() <= 1e-6);
        }
    }
    for (a, b) in [
        (Point3::new(16., 0., 1.), Point3::new(16., 0., 2.)),
        (Point3::new(16., 0., 0.), Point3::new(16., 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), Point3::new(24., 0., 0.)),
    ] {
        assert!(s.intersect_horizontal_segment(true, a, b).is_err());
        assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
    }
    assert_eq!(
        s.query_horizontal_point(true, Point3::new(16., 0., 0.))
            .unwrap(),
        query_before
    );
    assert!(s.split_horizontal(4., 0.1).is_err());
    assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
    s.apply(&x).unwrap();
    assert!(s
        .intersect_horizontal_segment(true, Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .is_err());
}
