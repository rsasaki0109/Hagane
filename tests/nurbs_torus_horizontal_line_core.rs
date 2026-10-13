use hagane::*;
#[test]
fn actual_child_nonunit_line_and_ray_keep_wall_and_annular_cap_hits() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let split = source.split_by_local_z(1., p).unwrap();
    for child in [split.lower, split.upper] {
        let before = child.export_step_mm(p).unwrap();
        let origin = Point3::new(16., 0., 0.);
        let direction = Vec3::new(0., 0., 2.);
        let line = child.intersect_line(origin, direction, p).unwrap();
        let expected = if child.is_lower() {
            [-2., 0.5]
        } else {
            [0.5, 2.]
        };
        assert_eq!(line.hits.len(), 2);
        assert_eq!(line.material_intervals.len(), 1);
        for (t, expected) in line.material_intervals[0].into_iter().zip(expected) {
            assert!((t - expected).abs() < 1e-7);
        }
        for hit in &line.hits {
            assert!((hit.point - (origin + direction * hit.parameter)).norm() < 1e-12);
            let w = &hit.boundary;
            assert!(
                (child.solid().shell.faces[w.face]
                    .surface
                    .try_evaluate(w.uv[0], w.uv[1])
                    .unwrap()
                    - w.point)
                    .norm()
                    < 1e-12
            );
        }
        let ray = child.intersect_ray(origin, direction, p).unwrap();
        assert_eq!(ray.material_intervals.len(), 1);
        assert!(
            (ray.material_intervals[0][0] - if child.is_lower() { 0. } else { 0.5 }).abs() < 1e-7
        );
        let z = if child.is_lower() { -1. } else { 2. };
        let hole_ray = child
            .intersect_ray(Point3::new(0., 0., z), Vec3::new(2., 0., 0.), p)
            .unwrap();
        assert_eq!(hole_ray.material_intervals.len(), 1);
        assert_eq!(hole_ray.hits.len(), 2);
        assert!(child
            .intersect_ray(Point3::new(16., 0., 1.), direction, p)
            .is_err());
        assert!(child
            .intersect_line(Point3::new(1e16, 0., 0.), direction, p)
            .is_err());
        assert_eq!(before, child.export_step_mm(p).unwrap());
    }
}
