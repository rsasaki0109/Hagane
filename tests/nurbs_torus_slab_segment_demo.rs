use hagane::*;
use serde_json::Value;
#[test]
fn accepted_slab_segment_reports_actual_caps_and_preserves_all_state() {
    let mut s = NurbsTorusDemoSession::new();
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&model).unwrap();
    assert!(s
        .intersect_slab_segment(Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .is_err());
    s.split_horizontal(1., 0.1).unwrap();
    s.slab_horizontal(-1., 1., 0.1).unwrap();
    let pointer = s.accepted_slab().unwrap() as *const _;
    let step = s.slab_step().unwrap();
    let source = s.query_point(Point3::new(16., 0., 2.)).unwrap();
    let child = s
        .query_horizontal_point(false, Point3::new(16., 0., 2.))
        .unwrap();
    let point = s.query_slab_point(Point3::new(16., 0., 0.)).unwrap();
    let data: Value = serde_json::from_str(
        &s.intersect_slab_segment(Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(data["hits"].as_array().unwrap().len(), 2);
    assert_eq!(
        data["material_intervals"],
        serde_json::json!([[0.4375, 0.5625]])
    );
    for h in data["hits"].as_array().unwrap() {
        let b = &h["boundary"];
        let f = b["face"].as_u64().unwrap() as usize;
        let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
        let actual = s.accepted_slab().unwrap().solid().shell.faces[f]
            .surface
            .try_evaluate(uv[0], uv[1])
            .unwrap();
        let p = Point3::new(
            h["point"][0].as_f64().unwrap(),
            h["point"][1].as_f64().unwrap(),
            h["point"][2].as_f64().unwrap(),
        );
        assert!((actual - p).norm() < 1e-9);
    }
    for (a, b) in [
        (Point3::new(16., 0., 1.), Point3::new(16., 0., 2.)),
        (Point3::new(16., 0., 0.), Point3::new(16., 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), Point3::new(24., 0., 0.)),
    ] {
        assert!(s.intersect_slab_segment(a, b).is_err());
        assert_eq!(s.accepted_slab().unwrap() as *const _, pointer);
        assert_eq!(s.slab_step().unwrap(), step);
    }
    assert!(s.slab_horizontal(-4., 1., 0.1).is_err());
    assert!(s.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert_eq!(s.slab_step().unwrap(), step);
    assert_eq!(s.query_point(Point3::new(16., 0., 2.)).unwrap(), source);
    assert_eq!(
        s.query_horizontal_point(false, Point3::new(16., 0., 2.))
            .unwrap(),
        child
    );
    assert_eq!(s.query_slab_point(Point3::new(16., 0., 0.)).unwrap(), point);
    s.apply(&model).unwrap();
    assert!(s
        .intersect_slab_segment(Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .is_err());
}
