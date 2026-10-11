use hagane::*;
use serde_json::Value;
#[test]
fn accepted_torus_queries_are_read_only_and_failed_apply_retains_actual_body() {
    let mut s = NurbsTorusDemoSession::new();
    assert!(s.query_point(Point3::new(16., 0., 0.)).is_err());
    let x = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    let report = s.apply(&x).unwrap();
    let pointer = s.accepted_body().unwrap() as *const _;
    for (p, expected) in [
        (Point3::new(0., 0., 0.), "outside"),
        (Point3::new(16., 0., 0.), "inside"),
        (Point3::new(20., 0., 0.), "boundary"),
    ] {
        let v: Value = serde_json::from_str(&s.query_point(p).unwrap()).unwrap();
        assert_eq!(v["location"], expected);
        if expected == "boundary" {
            let b = &v["boundary"];
            let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
            let f = b["face"].as_u64().unwrap() as usize;
            let actual = s.accepted_body().unwrap().solid().shell.faces[f]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            assert!((actual - Point3::new(20., 0., 0.)).norm() < 1e-9);
        }
    }
    for bad in [
        [16., 4., 0., 0., 0., 0., 1e-6, 1e-14],
        [4., 4., 0., 0., 0., 0., 1e-6, 0.1],
    ] {
        assert!(s.apply(&bad).is_err());
        assert_eq!(s.accepted_body().unwrap() as *const _, pointer);
    }
    assert!(s.query_point(Point3::new(f64::NAN, 0., 0.)).is_err());
    assert!(s.query_point(Point3::new(1e100, 0., 0.)).is_err());
    assert_eq!(s.accepted_body().unwrap() as *const _, pointer);
    assert_eq!(s.apply(&x).unwrap(), report);
}
