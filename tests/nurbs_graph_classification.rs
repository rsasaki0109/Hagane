use hagane::*;
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-4, 1e-10, 1e-12).unwrap()
}
#[test]
fn graph_classification_uses_euclidean_roof_band_and_rigid_placement() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([8., 6., 2.], 50., tol.absolute()).unwrap();
    let Surface::Nurbs(roof) = &source.brep().shell.faces[1].surface else {
        panic!()
    };
    let point = roof.evaluate(0.1, 0.5).unwrap();
    let normal = roof.normal(0.1, 0.5).unwrap();
    for sign in [-1., 1.] {
        let query = point + normal * (sign * tol.linear() * 0.5);
        assert_eq!(
            source.classify_point(query, tol).unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        source
            .classify_point(point + normal * (tol.linear() * 2.), tol)
            .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        source.classify_point(Point3::new(4., 3., 1.), tol).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        source
            .classify_point(Point3::new(-1., 0., 0.), tol)
            .unwrap(),
        PointLocation::Outside
    );
    let transform = Transform::translation(Vec3::new(12., -7., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let placed = source.transformed(transform, tol.absolute()).unwrap();
    assert_eq!(
        placed.classify_point(transform.point(point), tol).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        placed
            .classify_point(transform.point(Point3::new(4., 3., 1.)), tol)
            .unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn opening_void_has_no_roof_or_floor_boundary_and_every_query_checks_brep() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol.absolute()).unwrap();
    let solid =
        NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.2, 0.6]], tol.absolute()).unwrap();
    for z in [0., 1., 2.25] {
        assert_eq!(
            solid.classify_point(Point3::new(4., 2.4, z), tol).unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        solid
            .classify_point(Point3::new(2.4, 2.4, 1.), tol)
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        solid.classify_point(Point3::new(1., 1., 1.), tol).unwrap(),
        PointLocation::Inside
    );
    let mut changed = solid.clone();
    changed.solid.edges[0].vertices[0] = 15;
    assert!(changed
        .classify_point(Point3::new(100., 100., 100.), tol)
        .is_err());
    assert!(source
        .classify_point(Point3::new(f64::NAN, 0., 0.), tol)
        .is_err());
    assert!(source
        .classify_point(Point3::new(1e200, 0., 0.), tol)
        .is_err());
    let unresolved = GeometryTolerance::new(1e-20, 1e-10, 0.).unwrap();
    assert!(source
        .classify_point(Point3::new(4., 3., 1e-22), unresolved)
        .is_err());
}
#[test]
fn relative_band_depends_on_local_shape_not_world_query_distance() {
    let tol = GeometryTolerance::new(1e-9, 1e-10, 1e-4).unwrap();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol.absolute()).unwrap();
    let budget = tol.length_at_scale(8f64.hypot(6.).hypot(2.)).unwrap();
    assert_eq!(
        source
            .classify_point(Point3::new(4., 3., budget * 0.5), tol)
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        source
            .classify_point(Point3::new(4., 3., budget * 2.), tol)
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        source
            .classify_point(Point3::new(1000., 3., 0.), tol)
            .unwrap(),
        PointLocation::Outside
    );
}
