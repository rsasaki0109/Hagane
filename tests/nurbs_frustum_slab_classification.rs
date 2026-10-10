use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn plane(f: Frame3, s: f64, d: f64, tilt: bool) -> Surface {
    let n = Vec3::new(
        if tilt { 0.1 } else { 0. },
        if tilt { 0.05 } else { 0. },
        1.,
    )
    .normalized()
    .unwrap();
    let u = Vec3::new(0., 1., 0.);
    let u = (u - n * u.dot(n)).normalized().unwrap();
    let vector = |p: Vec3| f.axes()[0] * p.x + f.axes()[1] * p.y + f.axes()[2] * p.z;
    Surface::Plane {
        origin: f.point(Point3::new(0., 0., d * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn next(p: &Surface, f: Frame3, d: f64) -> Surface {
    let Surface::Plane { origin, u, v } = p else {
        panic!("plane")
    };
    Surface::Plane {
        origin: *origin + f.axes()[2] * d,
        u: *u,
        v: *v,
    }
}
fn not_false_boundary(result: Result<PointLocation>) {
    match result {
        Ok(location) => assert_eq!(location, PointLocation::Outside),
        Err(Error::Unsupported(_)) => {}
        Err(e) => panic!("{e:?}"),
    }
}
#[test]
fn finite_horizontal_slab_and_annular_rims_never_classify_removed_stock_as_material() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 24., p).unwrap();
    let lower = plane(f, 1., 8., false);
    let initial = source.split_by_plane(&lower, p).unwrap();
    let slab = initial
        .upper
        .split_by_parallel_plane(&next(&lower, f, 4.), p)
        .unwrap()
        .lower;
    let bored = source.bore_coaxial(4., p).unwrap();
    let initial = bored.split_by_plane(&lower, p).unwrap();
    let hole = initial
        .upper
        .split_by_parallel_plane(&next(&lower, f, 4.), p)
        .unwrap()
        .lower;
    for (point, plain, annular) in [
        (
            Point3::new(0., 0., 10.),
            PointLocation::Inside,
            PointLocation::Outside,
        ),
        (
            Point3::new(6., 0., 10.),
            PointLocation::Inside,
            PointLocation::Inside,
        ),
        (
            Point3::new(6., 0., 8.),
            PointLocation::Boundary,
            PointLocation::Boundary,
        ),
        (
            Point3::new(6., 0., 12.),
            PointLocation::Boundary,
            PointLocation::Boundary,
        ),
        (
            Point3::new(12., 0., 10.),
            PointLocation::Boundary,
            PointLocation::Boundary,
        ),
        (
            Point3::new(4., 0., 10.),
            PointLocation::Inside,
            PointLocation::Boundary,
        ),
        (
            Point3::new(0., 0., 8.),
            PointLocation::Boundary,
            PointLocation::Outside,
        ),
        (
            Point3::new(0., 0., 0.),
            PointLocation::Outside,
            PointLocation::Outside,
        ),
        (
            Point3::new(6., 0., 24.),
            PointLocation::Outside,
            PointLocation::Outside,
        ),
        (
            Point3::new(20., 0., 8.),
            PointLocation::Outside,
            PointLocation::Outside,
        ),
    ] {
        assert_eq!(slab.classify_point(point, p).unwrap(), plain);
        assert_eq!(hole.classify_point(point, p).unwrap(), annular);
    }
    for z in [8., 12.] {
        let sign = if z == 8. { -1. } else { 1. };
        not_false_boundary(hole.classify_point(Point3::new(4. - 0.8e-6, 0., z + sign * 0.8e-6), p));
        not_false_boundary(
            hole.classify_point(Point3::new(12. + 0.8e-6, 0., z + sign * 0.8e-6), p),
        );
        not_false_boundary(
            slab.classify_point(Point3::new(12. + 0.8e-6, 0., z + sign * 0.8e-6), p),
        );
        for result in [
            hole.classify_point(Point3::new(4. - 0.6e-6, 0., z + sign * 0.6e-6), p),
            slab.classify_point(Point3::new(12. + 0.6e-6, 0., z + sign * 0.6e-6), p),
        ] {
            assert!(matches!(
                result,
                Ok(PointLocation::Boundary) | Err(Error::Unsupported(_))
            ));
        }
    }
}
#[test]
fn actual_posed_repeated_slabs_reuse_source_policy_and_preserve_geometry() {
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let rot = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let f = Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), p).unwrap();
        let source = NurbsFrustumSolid::new(f, [16. * s, 8. * s], 24. * s, p)
            .unwrap()
            .bore_coaxial(4. * s, p)
            .unwrap();
        let lower = plane(f, s, 8., true);
        let initial = source.split_by_plane(&lower, p).unwrap();
        let middle = next(&lower, f, 4. * s);
        let upper = next(&lower, f, 8. * s);
        let slab = initial
            .upper
            .split_by_parallel_plane(&middle, p)
            .unwrap()
            .lower;
        let reverse = initial
            .upper
            .split_by_parallel_plane(&upper, p)
            .unwrap()
            .lower
            .split_by_parallel_plane(&middle, p)
            .unwrap()
            .lower;
        let before = format!("{:?}", slab.solid());
        let step = slab.export_step_mm(p).unwrap();
        let world = |point: Point3| f.point(point * s);
        for (point, expected) in [
            (Point3::new(6., 0., 10.), PointLocation::Inside),
            (Point3::new(0., 0., 10.), PointLocation::Outside),
            (Point3::new(6., 0., 7.4), PointLocation::Boundary),
            (Point3::new(6., 0., 11.4), PointLocation::Boundary),
            (Point3::new(6., 0., 6.), PointLocation::Outside),
            (Point3::new(6., 0., 16.), PointLocation::Outside),
            (Point3::new(4., 0., 10.), PointLocation::Boundary),
        ] {
            assert_eq!(slab.classify_point(world(point), p).unwrap(), expected);
            assert_eq!(reverse.classify_point(world(point), p).unwrap(), expected);
        }
        for point in [
            Point3::new(f64::NAN, 0., 0.),
            Point3::new(f64::INFINITY, 0., 0.),
            Point3::new(1e12 * s, 1e12 * s, 1e12 * s),
        ] {
            assert!(slab.classify_point(point, p).is_err());
        }
        let relative = GeometryTolerance::new(1e-6 * s, 1e-10, 0.001).unwrap();
        let band = relative.length_at_scale(32_f64.hypot(24.) * s).unwrap();
        let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
        let cap = Point3::new(6. * s, 0., 11.4 * s);
        assert_eq!(
            slab.classify_point(f.point(cap - n * (0.5 * band)), relative)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(before, format!("{:?}", slab.solid()));
        assert_eq!(step, slab.export_step_mm(p).unwrap());
    }
}

#[test]
fn steep_thin_slab_cap_bands_follow_actual_finite_euclidean_distance() {
    let p = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 480., p).unwrap();
    let normal = Vec3::new(10., 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let lower = Surface::Plane {
        origin: Point3::new(0., 0., 160.),
        u,
        v: normal.cross(u),
    };
    let initial = source.split_by_plane(&lower, p).unwrap();
    let slab = initial
        .upper
        .split_by_parallel_plane(&next(&lower, Frame3::IDENTITY, 8.), p)
        .unwrap()
        .lower;
    let broad = GeometryTolerance::new(0.5, 1e-10, 0.).unwrap();
    let result = slab.classify_point(Point3::new(0., 0., 164.), broad);
    assert_eq!(result.unwrap(), PointLocation::Boundary);
    // The normal gap to the upper finite cap is greater than the query band.
    not_false_boundary(slab.classify_point(Point3::new(0., 0., 176.), broad));
    assert_eq!(
        slab.classify_point(Point3::new(0., 0., 164.), p).unwrap(),
        PointLocation::Inside
    );
}
