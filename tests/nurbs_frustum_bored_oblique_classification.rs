use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn fixture(s: f64, posed: bool) -> (NurbsBoredFrustumPlaneSplit, Frame3) {
    let rotation =
        Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), policy(s))
            .unwrap();
    let body = NurbsFrustumSolid::new(frame, [16. * s, 8. * s], 24. * s, policy(s))
        .unwrap()
        .bore_coaxial(4. * s, policy(s))
        .unwrap();
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    let plane = Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 12. * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    };
    (body.split_by_plane(&plane, policy(s)).unwrap(), frame)
}
#[test]
fn retained_annular_material_and_actual_boundaries_classify_under_pose_scale() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let (split, frame) = fixture(scale, posed);
            let world = |p: Point3| frame.point(p * scale);
            for child in [&split.lower, &split.upper] {
                let before = format!("{:?}", child.solid());
                let step = child.export_step_mm(policy(scale)).unwrap();
                let volume = child.volume(policy(scale)).unwrap();
                let z = if child.is_lower() { 6. } else { 18. };
                let removed = if child.is_lower() { 18. } else { 6. };
                let r = 16. - z / 3.;
                for (p, expected) in [
                    (Point3::new(0., 0., z), PointLocation::Outside),
                    (Point3::new(6., 0., z), PointLocation::Inside),
                    (Point3::new(4., 0., z), PointLocation::Boundary),
                    (Point3::new(r, 0., z), PointLocation::Boundary),
                    (Point3::new(6., 0., 11.4), PointLocation::Boundary),
                    (Point3::new(0., 0., 12.), PointLocation::Outside),
                    (Point3::new(20., 0., 10.), PointLocation::Outside),
                    (Point3::new(6., 0., removed), PointLocation::Outside),
                    (
                        Point3::new(0., 0., if child.is_lower() { 0. } else { 24. }),
                        PointLocation::Outside,
                    ),
                ] {
                    assert_eq!(
                        child.classify_point(world(p), policy(scale)).unwrap(),
                        expected,
                        "point{p:?}, lower{}",
                        child.is_lower()
                    );
                }
                assert_eq!(before, format!("{:?}", child.solid()));
                assert_eq!(step, child.export_step_mm(policy(scale)).unwrap());
                assert_eq!(volume, child.volume(policy(scale)).unwrap());
            }
        }
    }
}
#[test]
fn finite_inner_rim_diagonal_never_uses_independent_plane_and_cylinder_bands() {
    let (split, frame) = fixture(1., false);
    let child = &split.lower;
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let rim = Point3::new(4., 0., 11.6);
    for policy in [
        policy(1.),
        GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap(),
    ] {
        let band = policy.length_at_scale(32_f64.hypot(24.)).unwrap();
        // Hole-ward radial and removed-side axial gaps each below the band.
        // At this tilt the combined .8/.8 displacement remains
        // beyond the actual finite rim distance threshold.
        let local = rim + Vec3::new(-0.8 * band, 0., 0.) + n * (0.8 * band);
        match child.classify_point(frame.point(local), policy) {
            Ok(location) => assert_eq!(location, PointLocation::Outside),
            Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("{e:?}"),
        }
        let cap = Point3::new(6., 0., 11.4);
        let within = cap - n * (0.5 * band);
        assert_eq!(
            child.classify_point(frame.point(within), policy).unwrap(),
            PointLocation::Boundary
        );
    }
}
#[test]
fn original_world_precision_and_nonfinite_queries_refuse_without_geometry_changes() {
    let (split, frame) = fixture(1., true);
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(policy(1.)).unwrap();
        for point in [
            Point3::new(f64::NAN, 0., 0.),
            Point3::new(f64::INFINITY, 0., 0.),
            Point3::new(1e12, 1e12, 1e12),
        ] {
            assert!(child.classify_point(point, policy(1.)).is_err());
        }
        let z = if child.is_lower() { 6. } else { 18. };
        assert_eq!(
            child
                .classify_point(frame.point(Point3::new(6., 0., z)), policy(1.))
                .unwrap(),
            PointLocation::Inside
        );
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(policy(1.)).unwrap());
    }
}
