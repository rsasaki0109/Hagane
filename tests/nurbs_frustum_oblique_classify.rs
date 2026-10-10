use hagane::*;

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn plane(frame: Frame3, c: f64, a: f64) -> Surface {
    let u = Vec3::new(1., 0., -a).normalized().unwrap();
    let n = Vec3::new(a, 0., 1.).normalized().unwrap();
    Surface::Plane {
        origin: frame.point(Point3::new(0., 0., c)),
        u: frame.vector(u),
        v: frame.vector(n.cross(u)),
    }
}
fn outside_or_unresolved(result: Result<PointLocation>) {
    assert!(matches!(
        result,
        Ok(PointLocation::Outside) | Err(Error::Unsupported(_))
    ));
}

#[test]
fn retained_material_signs_actual_faces_and_removed_source_boundary() {
    for radii in [[12., 12.], [16., 8.], [8., 16.]] {
        let p = policy(1.);
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., p).unwrap();
        let split = source
            .split_by_plane(&plane(Frame3::IDENTITY, 12., 0.1), p)
            .unwrap();
        for (body, inside, removed) in [
            (
                &split.lower,
                Point3::new(0., 0., 6.),
                Point3::new(0., 0., 18.),
            ),
            (
                &split.upper,
                Point3::new(0., 0., 18.),
                Point3::new(0., 0., 6.),
            ),
        ] {
            assert_eq!(
                body.classify_point(inside, p).unwrap(),
                PointLocation::Inside
            );
            assert_eq!(
                body.classify_point(removed, p).unwrap(),
                PointLocation::Outside
            );
            assert_eq!(
                body.classify_point(Point3::new(0., 0., 12.), p).unwrap(),
                PointLocation::Boundary
            );
            assert_eq!(
                body.classify_point(Point3::new(50., 0., 12.), p).unwrap(),
                PointLocation::Outside
            );
        }
        assert_eq!(
            split
                .lower
                .classify_point(Point3::new(0., 0., 24.), p)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            split
                .upper
                .classify_point(Point3::new(0., 0., 0.), p)
                .unwrap(),
            PointLocation::Outside
        );
        for z in [4., 20.] {
            let r = radii[0] + (radii[1] - radii[0]) * z / 24.;
            let body = if z < 12. { &split.lower } else { &split.upper };
            assert_eq!(
                body.classify_point(Point3::new(0., r, z), p).unwrap(),
                PointLocation::Boundary
            );
        }
    }
}

#[test]
fn finite_cylinder_rim_uses_euclidean_distance() {
    let p = policy(1.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let split = source
        .split_by_plane(&plane(Frame3::IDENTITY, 12., 0.), p)
        .unwrap();
    for (body, sign) in [(&split.lower, 1.), (&split.upper, -1.)] {
        assert!(matches!(
            body.classify_point(Point3::new(12. + 0.6e-6, 0., 12. + sign * 0.6e-6), p),
            Ok(PointLocation::Boundary) | Err(Error::Unsupported(_))
        ));
        outside_or_unresolved(
            body.classify_point(Point3::new(12. + 0.8e-6, 0., 12. + sign * 0.8e-6), p),
        );
        assert_eq!(
            body.classify_point(Point3::new(12. + 2e-6, 0., 12. + sign * 2e-6), p)
                .unwrap(),
            PointLocation::Outside
        );
    }
}

#[test]
fn pose_scale_normal_offsets_and_invalid_query_guards() {
    for scale in [0.001, 1., 1000.] {
        let p = policy(scale);
        let frame = Transform::translation(Vec3::new(100. * scale, -30. * scale, 5. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap();
        let source =
            NurbsFrustumSolid::new(frame, [12. * scale, 12. * scale], 24. * scale, p).unwrap();
        let split = source
            .split_by_plane(&plane(frame, 12. * scale, 0.1), p)
            .unwrap();
        let center = frame.point(Point3::new(0., 0., 12. * scale));
        let normal = frame.vector(Vec3::new(0.1, 0., 1.).normalized().unwrap());
        assert_eq!(
            split
                .lower
                .classify_point(center + normal * (0.6e-6 * scale), p)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            split
                .lower
                .classify_point(center + normal * (2e-6 * scale), p)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            split
                .lower
                .classify_point(center - normal * (2e-6 * scale), p)
                .unwrap(),
            PointLocation::Inside
        );
        assert!(split
            .lower
            .classify_point(Point3::new(f64::NAN, 0., 0.), p)
            .is_err());
        assert!(matches!(
            split
                .lower
                .classify_point(Point3::new(1e16 * scale, 0., 0.), p),
            Err(Error::Unsupported(_))
        ));
    }
}
