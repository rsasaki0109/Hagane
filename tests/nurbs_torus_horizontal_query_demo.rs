use hagane::*;
use serde_json::Value;
#[test]
fn actual_split_cache_is_transactional_and_child_probes_are_read_only() {
    let mut s = NurbsTorusDemoSession::new();
    let x = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&x).unwrap();
    assert!(s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .is_err());
    s.split_horizontal(1., 0.1).unwrap();
    let pair = s.accepted_horizontal_split().unwrap() as *const _;
    for (lower, p, expected) in [
        (true, Point3::new(16., 0., 0.), "inside"),
        (false, Point3::new(16., 0., 0.), "outside"),
        (true, Point3::new(16., 0., 1.), "boundary"),
        (false, Point3::new(16., 0., 1.), "boundary"),
        (false, Point3::new(16., 0., 2.), "inside"),
    ] {
        let v: Value = serde_json::from_str(&s.query_horizontal_point(lower, p).unwrap()).unwrap();
        assert_eq!(v["location"], expected);
        if expected == "boundary" {
            let body = if lower {
                &s.accepted_horizontal_split().unwrap().lower
            } else {
                &s.accepted_horizontal_split().unwrap().upper
            };
            let f = v["boundary"]["face"].as_u64().unwrap() as usize;
            let uv = [
                v["boundary"]["uv"][0].as_f64().unwrap(),
                v["boundary"]["uv"][1].as_f64().unwrap(),
            ];
            assert!(
                (body.solid().shell.faces[f]
                    .surface
                    .try_evaluate(uv[0], uv[1])
                    .unwrap()
                    - p)
                    .norm()
                    < 1e-9
            );
        }
        assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, pair);
    }
    assert!(s.split_horizontal(4., 0.1).is_err());
    assert!(s.split_horizontal(0., 1e-14).is_err());
    assert!(s.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert!(s
        .query_horizontal_point(true, Point3::new(f64::NAN, 0., 0.))
        .is_err());
    assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, pair);
    assert_eq!(
        serde_json::from_str::<Value>(&s.query_point(Point3::new(16., 0., 2.)).unwrap()).unwrap()
            ["location"],
        "inside"
    );
    s.apply(&x).unwrap();
    assert!(s.accepted_horizontal_split().is_none());
    assert!(s
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .is_err());
}
