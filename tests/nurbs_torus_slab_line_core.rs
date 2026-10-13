use hagane::*;
#[test]
fn original_nonunit_slab_lines_keep_two_caps_and_all_forward_material_intervals() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let slab = source.slab_between_local_z([-1., 1.], p).unwrap();
    let before = slab.export_step_mm(p).unwrap();
    let origin = Point3::new(16., 0., -8.);
    let direction = Vec3::new(0., 0., 2.);
    let line = slab.intersect_line(origin, direction, p).unwrap();
    assert_eq!(line.material_intervals.len(), 1);
    assert_eq!(line.hits.len(), 2);
    for (actual, expected) in line.material_intervals[0].into_iter().zip([3.5, 4.5]) {
        assert!((actual - expected).abs() < 1e-7);
    }
    for hit in &line.hits {
        let w = &hit.boundary;
        assert!((origin + direction * hit.parameter - hit.point).norm() < 1e-12);
        assert!(
            (slab.solid().shell.faces[w.face]
                .surface
                .try_evaluate(w.uv[0], w.uv[1])
                .unwrap()
                - w.point)
                .norm()
                < 1e-12
        );
        assert!(w.face >= slab.solid().shell.faces.len() - 2);
    }
    let across = slab
        .intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(across.hits.len(), 4);
    assert_eq!(across.material_intervals.len(), 2);
    let ray = slab
        .intersect_ray(Point3::new(-16., 0., 0.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(ray.material_intervals.len(), 2);
    assert_eq!(ray.material_intervals[0][0], 0.);
    assert_eq!(ray.hits.len(), 3);
    assert!(slab
        .intersect_line(Point3::new(16., 0., -1.), direction, p)
        .is_ok());
    assert!(slab
        .intersect_ray(Point3::new(16., 0., -1.), direction, p)
        .is_err());
    assert!(slab
        .intersect_line(Point3::new(1e16, 0., 0.), direction, p)
        .is_err());
    assert!(slab
        .intersect_line(origin, Vec3::new(0., 0., 0.), p)
        .is_err());
    assert_eq!(before, slab.export_step_mm(p).unwrap());
}
