use hagane::*;
fn pieces(p: GeometryTolerance) -> NurbsBoredFrustumPlaneSplit {
    NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap()
        .split_by_plane(
            &Surface::Plane {
                origin: Point3::new(0., 0., 12.),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.),
            },
            p,
        )
        .unwrap()
}
#[test]
fn real_annular_cut_foot_and_original_inner_wall_witnesses() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let s = pieces(p);
    assert_eq!(
        s.lower.classify_point(Point3::new(8., 0., 12.), p).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        s.upper.classify_point(Point3::new(8., 0., 12.), p).unwrap(),
        PointLocation::Boundary
    );
    for child in [&s.lower, &s.upper] {
        assert_eq!(
            child.classify_point(Point3::new(0., 0., 12.), p).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            child.classify_point(Point3::new(20., 0., 12.), p).unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        s.lower.classify_point(Point3::new(4., 0., 6.), p).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        s.upper.classify_point(Point3::new(4., 0., 6.), p).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        s.lower.classify_point(Point3::new(8., 0., 6.), p).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        s.upper.classify_point(Point3::new(8., 0., 18.), p).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn finite_inner_rim_never_becomes_infinite_plane_boundary() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let s = pieces(p);
    let query = Point3::new(4. - 0.8e-6, 0., 12. + 0.8e-6);
    assert!(matches!(
        s.lower.classify_point(query, p),
        Ok(PointLocation::Outside) | Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        s.lower.classify_point(Point3::new(1e100, 0., 12.), p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        s.lower.classify_point(Point3::new(f64::NAN, 0., 12.), p),
        Err(Error::InvalidInput(_))
    ));
    assert_eq!(
        s.lower.classify_point(Point3::new(8., 0., 6.), p).unwrap(),
        PointLocation::Inside
    );
}
