use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn body(frame: Frame3, scale: f64) -> NurbsBoredFrustumSolid {
    NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
        .unwrap()
        .bore_coaxial(4. * scale, policy(scale))
        .unwrap()
}
#[test]
fn actual_hole_and_annular_rim_use_euclidean_diagonal_distance() {
    let shape = body(Frame3::IDENTITY, 1.);
    let tol = policy(1.);
    for point in [
        Point3::new(0., 0., 12.),
        Point3::new(3., 0., 12.),
        Point3::new(0., 0., 0.),
        Point3::new(0., 0., 24.),
    ] {
        assert_eq!(
            shape.classify_point(point, tol).unwrap(),
            PointLocation::Outside
        );
    }
    for point in [
        Point3::new(4., 0., 12.),
        Point3::new(4., 0., 0.),
        Point3::new(4., 0., 24.),
        Point3::new(6., 0., 0.),
    ] {
        assert_eq!(
            shape.classify_point(point, tol).unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        shape.classify_point(Point3::new(6., 0., 12.), tol).unwrap(),
        PointLocation::Inside
    );
    for z in [0., 24.] {
        let direction = if z == 0. { -1. } else { 1. };
        assert_eq!(
            shape
                .classify_point(Point3::new(4. - 0.6e-6, 0., z + direction * 0.6e-6), tol)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            shape
                .classify_point(Point3::new(4. - 0.8e-6, 0., z + direction * 0.8e-6), tol)
                .unwrap(),
            PointLocation::Outside
        );
    }
    assert!(shape
        .classify_point(Point3::new(4. - 1e-6, 0., 12.), tol)
        .is_err());
}
#[test]
fn tapered_outer_normals_and_scaled_posed_inner_rims_are_metric_correct() {
    for scale in [1e-4, 1., 10.] {
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let frame = Frame3::new_with_tolerance(
            Point3::new(13. * scale, -7. * scale, 5. * scale),
            rotation.axes(),
            policy(scale),
        )
        .unwrap();
        let shape = body(frame, scale);
        let side = Point3::new(12. * scale, 0., 12. * scale);
        let normal = Vec3::new(24., 0., 8.).normalized().unwrap();
        for (offset, expected) in [
            (0.5, PointLocation::Boundary),
            (1.5, PointLocation::Outside),
            (-1.5, PointLocation::Inside),
        ] {
            assert_eq!(
                shape
                    .classify_point(
                        frame.point(side + normal * (offset * 1e-6 * scale)),
                        policy(scale)
                    )
                    .unwrap(),
                expected
            );
        }
        for (factor, expected) in [
            (0.6, PointLocation::Boundary),
            (0.8, PointLocation::Outside),
        ] {
            let point = frame.point(Point3::new(
                4. * scale - factor * 1e-6 * scale,
                0.,
                -factor * 1e-6 * scale,
            ));
            assert_eq!(
                shape.classify_point(point, policy(scale)).unwrap(),
                expected
            );
        }
    }
}
#[test]
fn full_relative_band_and_query_precision_reject_ambiguity_without_fake_locations() {
    let shape = body(Frame3::IDENTITY, 1.);
    let relative = GeometryTolerance::new(1e-6, 1e-10, 1e-3).unwrap();
    let band = relative.length_at_scale(40.).unwrap();
    assert_eq!(
        shape
            .classify_point(Point3::new(4. - 0.5 * band, 0., 12.), relative)
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        shape
            .classify_point(Point3::new(4. - 1.5 * band, 0., 12.), relative)
            .unwrap(),
        PointLocation::Outside
    );
    assert!(shape
        .classify_point(Point3::new(4. - band, 0., 12.), relative)
        .is_err());
    for point in [
        Point3::new(1e12, 0., 0.),
        Point3::new(f64::MAX, f64::MAX, 0.),
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(0., f64::INFINITY, 0.),
    ] {
        assert!(shape.classify_point(point, policy(1.)).is_err());
    }
    assert_eq!(
        shape
            .classify_point(Point3::new(5., 0., 12.), policy(1.))
            .unwrap(),
        PointLocation::Inside
    );
}
