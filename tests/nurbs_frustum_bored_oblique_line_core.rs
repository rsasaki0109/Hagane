use hagane::*;
fn parts() -> (NurbsBoredFrustumPlaneSplit, GeometryTolerance) {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let n = Vec3::new(0.2, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    (
        NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
            .unwrap()
            .bore_coaxial(4., p)
            .unwrap()
            .split_by_plane(
                &Surface::Plane {
                    origin: Point3::new(0., 0., 12.),
                    u,
                    v: n.cross(u),
                },
                p,
            )
            .unwrap(),
        p,
    )
}
#[test]
fn actual_two_intervals_and_nonunit_ray_parameters_survive_bored_oblique_dispatch() {
    let (parts, p) = parts();
    let origin = Point3::new(0., 0., 6.);
    let direction = Vec3::new(2., 0., 0.);
    let result = parts.lower.intersect_line(origin, direction, p).unwrap();
    assert_eq!(result.material_intervals.len(), 2);
    assert_eq!(result.hits.len(), 4);
    for (hit, t) in result.hits.iter().zip([-6., -2., 2., 6.]) {
        assert!((hit.parameter - t).abs() < 1e-10);
        for f in &hit.faces {
            let q = parts.lower.solid().shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((q - hit.point).norm() < 1e-10);
        }
    }
    let ray = parts.lower.intersect_ray(origin, direction, p).unwrap();
    assert_eq!(ray.hits.len(), 2);
    assert_eq!(ray.material_intervals.len(), 1);
    assert!((ray.material_intervals[0][0] - 2.).abs() < 1e-10);
    let inside = parts
        .lower
        .intersect_ray(Point3::new(8., 0., 6.), Vec3::new(-2., 0., 0.), p)
        .unwrap();
    assert_eq!(inside.material_intervals.len(), 2);
    assert_eq!(inside.material_intervals[0][0], 0.);
    assert_eq!(inside.hits.len(), 3);
}
#[test]
fn actual_annular_cut_cap_and_explicit_precision_refusals() {
    let (parts, p) = parts();
    let origin = Point3::new(6., 0., 4.);
    let direction = Vec3::new(0.05, 0., 2.);
    let r = parts.lower.intersect_line(origin, direction, p).unwrap();
    assert_eq!(r.material_intervals.len(), 1);
    assert_eq!(r.hits.len(), 2);
    assert!(r
        .hits
        .iter()
        .any(|h| h.faces.iter().any(|f| f.face_id == 1)));
    let reverse = parts
        .lower
        .intersect_line(origin, direction * -1., p)
        .unwrap();
    for (a, b) in r.hits.iter().zip(reverse.hits.iter().rev()) {
        assert!((a.parameter + b.parameter).abs() < 1e-10);
    }
    assert!(parts
        .lower
        .intersect_ray(Point3::new(4., 0., 6.), Vec3::new(1., 0., 0.), p)
        .is_err());
    assert!(parts
        .lower
        .intersect_line(Point3::new(1e100, 0., 0.), direction, p)
        .is_err());
    assert!(parts
        .lower
        .intersect_ray(Point3::new(20., 0., 6.), Vec3::new(2., 0., 0.), p)
        .unwrap()
        .material_intervals
        .is_empty());
}
