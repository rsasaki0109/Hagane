use hagane::*;
fn triangle() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [0., 1.]]
}
#[test]
fn actual_trimmed_caps_and_euclidean_corners() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([10., 10., 2.], 0., tol).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, triangle(), tol).unwrap();
    let gt = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    for (p, want) in [
        (Point3::new(2., 2., 1.), PointLocation::Inside),
        (Point3::new(8., 8., 0.), PointLocation::Outside),
        (Point3::new(8., 8., 2.), PointLocation::Outside),
        (Point3::new(2., 2., 0.), PointLocation::Boundary),
        (Point3::new(5., 5., 1.), PointLocation::Boundary),
        (Point3::new(-0.8e-5, -0.8e-5, 1.), PointLocation::Outside),
        (Point3::new(-0.5e-5, -0.5e-5, 1.), PointLocation::Boundary),
    ] {
        assert_eq!(body.classify_point(p, gt).unwrap(), want);
    }
}
#[test]
fn hole_excluded_witnesses_and_true_inner_walls() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([10., 10., 2.], 0., tol).unwrap();
    let body = source
        .through_uv_polygon(vec![[0.3, 0.3], [0.7, 0.3], [0.7, 0.7], [0.3, 0.7]], tol)
        .unwrap();
    let gt = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    for (p, want) in [
        (Point3::new(5., 5., 0.), PointLocation::Outside),
        (Point3::new(5., 5., 2.), PointLocation::Outside),
        (Point3::new(5., 5., 1.), PointLocation::Outside),
        (Point3::new(3., 5., 1.), PointLocation::Boundary),
        (Point3::new(2., 5., 1.), PointLocation::Inside),
    ] {
        assert_eq!(body.classify_point(p, gt).unwrap(), want);
    }
}
#[test]
fn steep_roof_and_canonical_failure() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([10., 10., 2.], 20., tol).unwrap();
    let mut body = NurbsGraphPolygonSolid::new(&source, triangle(), tol).unwrap();
    let Surface::Nurbs(roof) = &body.solid.shell.faces[1].surface else {
        panic!()
    };
    let p = roof.evaluate(0.2, 0.3).unwrap();
    let n = roof.normal(0.2, 0.3).unwrap();
    let gt = GeometryTolerance::new(1e-4, 1e-10, 0.).unwrap();
    assert_eq!(
        body.classify_point(p + n * 0.5e-4, gt).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        body.classify_point(p + n * 2e-4, gt).unwrap(),
        PointLocation::Outside
    );
    body.solid.vertices[0].point.x += tol.linear / 10.;
    assert!(body.classify_point(Point3::new(1., 1., 1.), gt).is_err());
}
