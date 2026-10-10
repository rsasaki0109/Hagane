use hagane::*;
#[test]
fn bored_original_parameters_and_forward_hole_and_material_origins() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let line = body
        .intersect_line(Point3::new(0., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(line.material_intervals.len(), 2);
    assert_eq!(line.hits.len(), 4);
    for (hit, t) in line.hits.iter().zip([-6., -2., 2., 6.]) {
        assert!((hit.parameter - t).abs() < 1e-10);
    }
    let hole_ray = body
        .intersect_ray(Point3::new(0., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(hole_ray.hits.len(), 2);
    assert!((hole_ray.material_intervals[0][0] - 2.).abs() < 1e-10);
    let material_ray = body
        .intersect_ray(Point3::new(8., 0., 12.), Vec3::new(-2., 0., 0.), p)
        .unwrap();
    assert_eq!(material_ray.material_intervals.len(), 2);
    assert_eq!(material_ray.material_intervals[0][0], 0.);
    assert_eq!(material_ray.hits.len(), 3);
    let away = body
        .intersect_ray(Point3::new(20., 0., 12.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert!(away.hits.is_empty());
    assert!(away.material_intervals.is_empty());
}
#[test]
fn oblique_caps_and_cut_use_actual_surfaces_and_original_line_parameter() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let split = source
        .split_by_plane(
            &Surface::Plane {
                origin: Point3::new(0., 0., 12.),
                u,
                v: n.cross(u),
            },
            p,
        )
        .unwrap();
    let line = split
        .lower
        .intersect_line(Point3::new(2., 1., 4.), Vec3::new(0., 0., 2.), p)
        .unwrap();
    assert_eq!(line.hits.len(), 2);
    assert_eq!(line.material_intervals.len(), 1);
    assert!((line.material_intervals[0][0] + 2.).abs() < 1e-10);
    assert!((line.material_intervals[0][1] - 3.9).abs() < 1e-10);
    for hit in &line.hits {
        for f in &hit.faces {
            let q = split.lower.solid().shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((q - hit.point).norm() < 1e-10);
        }
    }
    let ray = split
        .lower
        .intersect_ray(Point3::new(2., 1., 4.), Vec3::new(0., 0., 2.), p)
        .unwrap();
    assert_eq!(ray.material_intervals[0][0], 0.);
    assert_eq!(ray.hits.len(), 1);
    assert!(split
        .lower
        .intersect_line(Point3::new(1e100, 0., 0.), Vec3::new(1., 0., 0.), p)
        .is_err());
}
