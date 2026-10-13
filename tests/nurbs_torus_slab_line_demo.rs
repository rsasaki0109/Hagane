use hagane::*;
use serde_json::Value;
#[test]
fn actual_slab_line_ray_preserve_original_parameters_and_all_cached_queries() {
    let mut s = NurbsTorusDemoSession::new();
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    s.apply(&model).unwrap();
    assert!(s
        .intersect_slab_line(Point3::new(16., 0., -8.), Vec3::new(0., 0., 2.))
        .is_err());
    s.split_horizontal(1., 0.1).unwrap();
    s.slab_horizontal(-1., 1., 0.1).unwrap();
    let ptr = s.accepted_slab().unwrap() as *const _;
    let step = s.slab_step().unwrap();
    let source = s.query_point(Point3::new(16., 0., 2.)).unwrap();
    let child = s
        .query_horizontal_point(false, Point3::new(16., 0., 2.))
        .unwrap();
    let point = s.query_slab_point(Point3::new(16., 0., 0.)).unwrap();
    let segment = s
        .intersect_slab_segment(Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .unwrap();
    for ray in [false, true] {
        let report = if ray {
            s.intersect_slab_ray(Point3::new(16., 0., -8.), Vec3::new(0., 0., 2.))
        } else {
            s.intersect_slab_line(Point3::new(16., 0., -8.), Vec3::new(0., 0., 2.))
        };
        let data: Value = serde_json::from_str(&report.unwrap()).unwrap();
        assert_eq!(data["material_intervals"].as_array().unwrap().len(), 1);
        for (j, expected) in [3.5, 4.5].iter().enumerate() {
            assert!(
                (data["material_intervals"][0][j].as_f64().unwrap() - expected).abs()
                    <= 1e-6 / 32. + 64. * f64::EPSILON
            );
        }
        assert_eq!(data["hits"].as_array().unwrap().len(), 2);
        assert_eq!(data["direction"], serde_json::json!([0., 0., 2.]));
        for hit in data["hits"].as_array().unwrap() {
            let b = &hit["boundary"];
            let f = b["face"].as_u64().unwrap() as usize;
            let uv = [b["uv"][0].as_f64().unwrap(), b["uv"][1].as_f64().unwrap()];
            let actual = s.accepted_slab().unwrap().solid().shell.faces[f]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            let p = Point3::new(
                hit["point"][0].as_f64().unwrap(),
                hit["point"][1].as_f64().unwrap(),
                hit["point"][2].as_f64().unwrap(),
            );
            assert!((actual - p).norm() < 1e-9);
        }
    }
    let clipped: Value = serde_json::from_str(
        &s.intersect_slab_ray(Point3::new(16., 0., 0.), Vec3::new(0., 0., 2.))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(clipped["material_intervals"][0][0], 0.);
    assert!(
        (clipped["material_intervals"][0][1].as_f64().unwrap() - 0.5).abs()
            <= 1e-6 / 32. + 64. * f64::EPSILON
    );
    for (o, d) in [
        (Point3::new(16., 0., 0.), Vec3::new(0., 0., 0.)),
        (Point3::new(1e100, 0., 0.), Vec3::new(0., 0., 2.)),
        (Point3::new(f64::NAN, 0., 0.), Vec3::new(0., 0., 2.)),
    ] {
        assert!(s.intersect_slab_line(o, d).is_err());
        assert_eq!(s.accepted_slab().unwrap() as *const _, ptr);
        assert_eq!(s.slab_step().unwrap(), step);
    }
    assert!(s
        .intersect_slab_ray(Point3::new(16., 0., 1.), Vec3::new(0., 0., 2.))
        .is_err());
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
    assert_eq!(
        s.intersect_slab_segment(Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
            .unwrap(),
        segment
    );
    s.apply(&model).unwrap();
    assert!(s
        .intersect_slab_line(Point3::new(16., 0., -8.), Vec3::new(0., 0., 2.))
        .is_err());
}
