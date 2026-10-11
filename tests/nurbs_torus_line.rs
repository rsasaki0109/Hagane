use hagane::*;
#[test]
fn original_nonunit_line_and_ray_preserve_all_material_intervals() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let origin = Point3::new(0., 0., 0.);
    let direction = Vec3::new(2., 0., 0.);
    let line = body.intersect_line(origin, direction, p).unwrap();
    assert_eq!(line.hits.len(), 4);
    assert_eq!(line.material_intervals.len(), 2);
    for (hit, t) in line.hits.iter().zip([-10., -6., 6., 10.]) {
        assert!((hit.parameter - t).abs() < 1e-7);
        assert!((hit.point - (origin + direction * hit.parameter)).norm() < 1e-12);
    }
    let ray = body.intersect_ray(origin, direction, p).unwrap();
    assert_eq!(ray.hits.len(), 2);
    assert!((ray.material_intervals[0][0] - 6.).abs() < 1e-7);
    let inside = body
        .intersect_ray(Point3::new(-16., 0., 0.), direction, p)
        .unwrap();
    assert_eq!(inside.material_intervals.len(), 2);
    assert_eq!(inside.material_intervals[0][0], 0.);
    assert!((inside.material_intervals[0][1] - 2.).abs() < 1e-7);
    assert!((inside.material_intervals[1][0] - 14.).abs() < 1e-7);
    assert!(body
        .intersect_line(Point3::new(0., 30., 0.), direction, p)
        .unwrap()
        .hits
        .is_empty());
    assert!(body
        .intersect_ray(Point3::new(20., 0., 0.), direction, p)
        .is_err());
    assert!(body
        .intersect_line(Point3::new(1e16, 0., 0.), direction, p)
        .is_err());
    assert!(body
        .intersect_line(origin, Vec3::new(0., 0., 0.), p)
        .is_err());
}
