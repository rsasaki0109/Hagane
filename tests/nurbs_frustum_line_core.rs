use hagane::*;
#[test]
fn physical_line_parameter_nonunit_axis_and_ray_intervals() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    for magnitude in [1e-200, 2., 1e200] {
        let origin = Point3::new(1., 1., -5.);
        let direction = Vec3::new(0., 0., magnitude);
        let hit = body.intersect_line(origin, direction, p).unwrap();
        assert_eq!(hit.hits.len(), 2);
        let i = hit.material_interval.unwrap();
        assert!((i[0] * magnitude - 5.).abs() < 1e-10);
        assert!((i[1] * magnitude - 29.).abs() < 1e-10);
        for h in hit.hits {
            assert_eq!(h.point, origin + direction * h.parameter);
            for f in h.faces {
                assert!(
                    (body.solid().shell.faces[f.face_id]
                        .surface
                        .try_evaluate(f.uv[0], f.uv[1])
                        .unwrap()
                        - h.point)
                        .norm()
                        < p.linear()
                );
            }
        }
    }
    let inside = body
        .intersect_ray(Point3::new(0., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    let interval = inside.material_interval.unwrap();
    assert_eq!(interval[0], 0.);
    assert!((interval[1] - 6.).abs() < 1e-12);
    assert_eq!(inside.hits.len(), 1);
    let behind = body
        .intersect_ray(Point3::new(30., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert!(behind.material_interval.is_none());
    assert!(behind.hits.is_empty());
    let fromboundary = body
        .intersect_line(Point3::new(12., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert!(fromboundary.hits[1].parameter.abs() < 1e-12);
    assert!(body
        .intersect_ray(Point3::new(12., 0., 12.), Vec3::new(2., 0., 0.), p)
        .is_err());
}
#[test]
fn pose_and_large_anchor_guard_are_preserved_before_cropping() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let r = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), r.axes(), p).unwrap();
    let body = NurbsFrustumSolid::new(frame, [16., 8.], 24., p).unwrap();
    let hit = body
        .intersect_line(
            frame.point(Point3::new(0., 0., 12.)),
            frame.vector(Vec3::new(0., 0., -3.)),
            p,
        )
        .unwrap();
    let i = hit.material_interval.unwrap();
    assert!((i[0] + 4.).abs() < 1e-10);
    assert!((i[1] - 4.).abs() < 1e-10);
    assert!(body
        .intersect_line(Point3::new(1e14, 0., 0.), Vec3::new(-1., 0., 0.), p)
        .is_err());
    assert!(body
        .intersect_line(Point3::new(0., 0., 0.), Vec3::new(0., 0., 0.), p)
        .is_err());
}
