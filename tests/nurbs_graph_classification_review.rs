use hagane::*;
fn tol() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn placement() -> Transform {
    Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap()
}
fn roof(u: f64, v: f64) -> f64 {
    3. - 8. * u * (1. - u) * v * (1. - v)
}
fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], -2., Tolerance::default())
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], Tolerance::default())
        .unwrap()
        .transformed(placement(), Tolerance::default())
        .unwrap()
}
fn world(u: f64, v: f64, z: f64) -> Point3 {
    placement().point(Point3::new(20. * u, 12. * v, z))
}
#[test]
fn signed_trimmed_placed_material_and_boundary_probes_match_analytic_membership() {
    let s = source();
    for (u, v) in [(0.3, 0.2), (0.5, 0.4), (0.7, 0.6)] {
        assert_eq!(
            s.classify_point(world(u, v, roof(u, v) / 2.), tol())
                .unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            s.classify_point(world(u, v, roof(u, v) + 0.5), tol())
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            s.classify_point(world(u, v, 0.), tol()).unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            s.classify_point(world(u, v, roof(u, v)), tol()).unwrap(),
            PointLocation::Boundary
        );
    }
    for (u, v) in [(0.1, 0.4), (0.9, 0.4), (0.5, 0.), (0.5, 0.8)] {
        assert_eq!(
            s.classify_point(world(u, v, 1.), tol()).unwrap(),
            PointLocation::Outside
        );
    }
    for (u, v) in [(0.2, 0.4), (0.8, 0.4), (0.5, 0.1), (0.5, 0.7)] {
        assert_eq!(
            s.classify_point(world(u, v, 1.), tol()).unwrap(),
            PointLocation::Boundary
        );
    }
}
#[test]
fn opening_void_has_no_false_cap_and_all_inner_walls_are_boundary() {
    let source = source();
    let s = NurbsGraphHoledSolid::new(&source, [[0.35, 0.5], [0.25, 0.4]], Tolerance::default())
        .unwrap();
    for z in [0., 1., roof(0.425, 0.325)] {
        assert_eq!(
            s.classify_point(world(0.425, 0.325, z), tol()).unwrap(),
            PointLocation::Outside
        );
    }
    for (u, v) in [(0.35, 0.325), (0.5, 0.325), (0.425, 0.25), (0.425, 0.4)] {
        assert_eq!(
            s.classify_point(world(u, v, 1.), tol()).unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            s.classify_point(world(u, v, 0.), tol()).unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        s.classify_point(world(0.6, 0.5, 1.), tol()).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn boundary_band_is_euclidean_on_steep_roof_and_around_outer_corners() {
    let s = NurbsGraphSolid::new([1., 1., 1.], 100., Tolerance::default()).unwrap();
    let point = Point3::new(0.1, 0.5, 10.);
    let normal = Vec3::new(-80., 0., 1.).normalized().unwrap();
    for direction in [-1., 1.] {
        let p = point + normal * (direction * 0.5e-6);
        match s.classify_point(p, tol()) {
            Ok(location) => assert_eq!(location, PointLocation::Boundary),
            Err(Error::Unsupported(_)) => {}
            Err(error) => panic!("unexpected {error:?}"),
        }
    }
    assert_eq!(
        s.classify_point(point, tol()).unwrap(),
        PointLocation::Boundary
    );
    let flat = NurbsGraphSolid::new([1.; 3], 0., Tolerance::default()).unwrap();
    assert_eq!(
        flat.classify_point(Point3::new(-0.5e-6, 0.5, 0.5), tol())
            .unwrap(),
        PointLocation::Boundary
    );
    match flat.classify_point(Point3::new(-0.8e-6, -0.8e-6, 0.5), tol()) {
        Ok(location) => assert_eq!(location, PointLocation::Outside),
        Err(Error::Unsupported(_)) => {}
        Err(error) => panic!("unexpected {error:?}"),
    };
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.1).unwrap();
    assert_eq!(
        flat.classify_point(Point3::new(1e3, 1e3, 1e3), relative)
            .unwrap(),
        PointLocation::Outside
    );
    match flat.classify_point(Point3::new(1e100, 1e100, 1e100), relative) {
        Ok(location) => assert_eq!(location, PointLocation::Outside),
        Err(Error::Unsupported(_)) => {}
        Err(error) => panic!("unexpected {error:?}"),
    };
}
#[test]
fn classification_rejects_nonfinite_queries_and_public_geometry_corruption() {
    let source = source();
    for point in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(0., f64::INFINITY, 0.),
    ] {
        assert!(source.classify_point(point, tol()).is_err());
    }
    let mut dirty = source.clone();
    dirty.solid.vertices[0].point.x += 0.1;
    assert!(dirty.classify_point(world(0.6, 0.5, 1.), tol()).is_err());
    let mut holed =
        NurbsGraphHoledSolid::new(&source, [[0.35, 0.5], [0.25, 0.4]], Tolerance::default())
            .unwrap();
    holed.solid.shell.faces[1].wires[1].coedges[0].edge = 999;
    assert!(holed.classify_point(world(0.6, 0.5, 1.), tol()).is_err());
}
