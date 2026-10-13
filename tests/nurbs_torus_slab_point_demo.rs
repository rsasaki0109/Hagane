use hagane::*;
use serde_json::Value;
#[test]
fn actual_slab_point_witnesses_preserve_all_accepted_caches_and_exports() {
    let mut s = NurbsTorusDemoSession::new();
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&model).unwrap();
    assert!(s.query_slab_point(Point3::new(16., 0., 0.)).is_err());
    s.split_horizontal(1., 0.1).unwrap();
    s.slab_horizontal(-1., 1., 0.1).unwrap();
    let ptr = s.accepted_slab().unwrap() as *const _;
    let step = s.slab_step().unwrap();
    let source = s.query_point(Point3::new(16., 0., 2.)).unwrap();
    let child = s
        .query_horizontal_point(false, Point3::new(16., 0., 2.))
        .unwrap();
    for (point, expected) in [
        (Point3::new(16., 0., 0.), "inside"),
        (Point3::new(16., 0., 2.), "outside"),
        (Point3::new(0., 0., 0.), "outside"),
        (Point3::new(16., 0., 1.), "boundary"),
        (Point3::new(16., 0., -1.), "boundary"),
    ] {
        let data: Value = serde_json::from_str(&s.query_slab_point(point).unwrap()).unwrap();
        assert_eq!(data["location"], expected);
        if expected == "boundary" {
            let b = &data["boundary"];
            let f = b["face"].as_u64().unwrap() as usize;
            let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
            let actual = s.accepted_slab().unwrap().solid().shell.faces[f]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            assert!((actual - point).norm() < 1e-9);
        }
        assert_eq!(s.accepted_slab().unwrap() as *const _, ptr);
        assert_eq!(s.slab_step().unwrap(), step);
    }
    for point in [Point3::new(f64::NAN, 0., 0.), Point3::new(1e100, 0., 0.)] {
        assert!(s.query_slab_point(point).is_err());
        assert_eq!(s.slab_step().unwrap(), step);
    }
    assert!(s.slab_horizontal(-4., 1., 0.1).is_err());
    assert_eq!(s.slab_step().unwrap(), step);
    assert_eq!(s.query_point(Point3::new(16., 0., 2.)).unwrap(), source);
    assert_eq!(
        s.query_horizontal_point(false, Point3::new(16., 0., 2.))
            .unwrap(),
        child
    );
    s.apply(&model).unwrap();
    assert!(s.query_slab_point(Point3::new(16., 0., 0.)).is_err());
}
