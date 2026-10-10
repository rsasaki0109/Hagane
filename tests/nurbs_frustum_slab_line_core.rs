use hagane::*;
#[test]
fn slab_lines_use_actual_two_caps_and_rays_preserve_accepted_geometry() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let plane = |z| Surface::Plane {
        origin: Point3::new(0., 0., z),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let slab = source
        .split_by_plane(&plane(6.), p)
        .unwrap()
        .upper
        .split_by_parallel_plane(&plane(18.), p)
        .unwrap()
        .lower;
    let bored = source
        .bore_coaxial(4., p)
        .unwrap()
        .split_by_plane(&plane(6.), p)
        .unwrap()
        .upper
        .split_by_parallel_plane(&plane(18.), p)
        .unwrap()
        .lower;
    let before = format!("{:?}", bored.solid());
    let line = slab
        .intersect_line(Point3::new(6., 0., 0.), Vec3::new(0., 0., 3.), p)
        .unwrap();
    assert_eq!(line.material_intervals.len(), 1);
    for (got, want) in line.material_intervals[0].into_iter().zip([2., 6.]) {
        assert!((got - want).abs() < 1e-12);
    }
    assert_eq!(line.hits.len(), 2);
    for (hit, id) in line.hits.iter().zip([0, 1]) {
        assert_eq!(hit.faces[0].face_id, id);
        let witness = &hit.faces[0];
        let point = slab.solid().shell.faces[id]
            .surface
            .try_evaluate(witness.uv[0], witness.uv[1])
            .unwrap();
        assert!((point - hit.point).norm() < 1e-12);
    }
    let ray = bored
        .intersect_ray(Point3::new(6., 0., 10.), Vec3::new(0., 0., 2.), p)
        .unwrap();
    assert_eq!(ray.material_intervals.len(), 1);
    assert_eq!(ray.material_intervals[0][0], 0.);
    assert!((ray.material_intervals[0][1] - 4.).abs() < 1e-12);
    assert_eq!(ray.hits.len(), 1);
    assert_eq!(ray.hits[0].faces[0].face_id, 1);
    assert!(matches!(
        bored.intersect_ray(Point3::new(4., 0., 10.), Vec3::new(1., 0., 0.), p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        slab.intersect_line(Point3::new(0., 0., 0.), Vec3::new(0., 0., 0.), p),
        Err(Error::InvalidInput(_))
    ));
    assert_eq!(before, format!("{:?}", bored.solid()));
}
