use hagane::*;

#[test]
fn relative_band_uses_source_extent_for_both_retained_pieces() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 1e-5).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let cut = Surface::Plane {
        origin: Point3::new(0., 0., 6.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let split = source.split_by_plane(&cut, p).unwrap();
    let band = p.length_at_scale(24f64.hypot(24.)).unwrap();
    for dz in [-0.7 * band, 0., 0.7 * band] {
        let query = Point3::new(0., 0., 6. + dz);
        assert_eq!(
            split.lower.classify_point(query, p).unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            split.upper.classify_point(query, p).unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        split
            .lower
            .classify_point(Point3::new(0., 0., 3.), p)
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        split
            .upper
            .classify_point(Point3::new(0., 0., 3.), p)
            .unwrap(),
        PointLocation::Outside
    );
}

#[test]
fn finite_cut_extensions_do_not_create_boundary_witnesses() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let split = source
        .split_by_plane(
            &Surface::Plane {
                origin: Point3::new(0., 0., 12.),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.),
            },
            p,
        )
        .unwrap();
    let query = Point3::new(12. + 0.8e-6, 0., 12. + 0.8e-6);
    assert!(matches!(
        split.lower.classify_point(query, p),
        Ok(PointLocation::Outside) | Err(Error::Unsupported(_))
    ));
    assert_eq!(
        split
            .lower
            .classify_point(Point3::new(12., 0., 18.), p)
            .unwrap(),
        PointLocation::Outside
    );
    assert!(matches!(
        split.lower.classify_point(Point3::new(1e100, 0., 0.), p),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        split
            .lower
            .classify_point(Point3::new(0., 0., 3.), p)
            .unwrap(),
        PointLocation::Inside
    );
}
