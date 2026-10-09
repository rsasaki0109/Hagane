use hagane::*;
fn tol() -> GeometryTolerance {
    GeometryTolerance::new(1e-4, 1e-12, 0.).unwrap()
}
#[test]
fn actual_material_and_removed_cap_witnesses_are_distinguished() {
    let t = tol();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t.absolute()).unwrap();
    let body = source
        .through_xy_circle([4., 3.], 1., t.absolute())
        .unwrap();
    for p in [
        Point3::new(4., 3., 0.),
        Point3::new(4., 3., 2.),
        Point3::new(4., 3., 1.),
        Point3::new(4.9, 3., -0.5 * t.linear()),
        Point3::new(4.9, 3., 2. + 0.5 * t.linear()),
    ] {
        assert_eq!(body.classify_point(p, t).unwrap(), PointLocation::Outside);
    }
    for p in [Point3::new(2., 2., 1.), Point3::new(5.5, 3., 1.)] {
        assert_eq!(body.classify_point(p, t).unwrap(), PointLocation::Inside);
    }
    for p in [
        Point3::new(5., 3., 1.),
        Point3::new(3., 3., 1.),
        Point3::new(4., 4., 1.),
        Point3::new(4., 2., 1.),
        Point3::new(5.5, 3., 0.),
        Point3::new(5.5, 3., 2.),
    ] {
        assert_eq!(body.classify_point(p, t).unwrap(), PointLocation::Boundary);
    }
}
#[test]
fn circular_rim_uses_three_dimensional_euclidean_distance() {
    let t = tol();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t.absolute()).unwrap();
    let body = source
        .through_xy_circle([4., 3.], 1., t.absolute())
        .unwrap();
    for d in [0.4, 0.8] {
        let point = Point3::new(5. - d * t.linear(), 3., 2. + d * t.linear());
        let expected = if d < 0.5 {
            PointLocation::Boundary
        } else {
            PointLocation::Outside
        };
        assert_eq!(body.classify_point(point, t).unwrap(), expected);
    }
    let point = Point3::new(-0.8 * t.linear(), -0.8 * t.linear(), 1.);
    assert_eq!(
        body.classify_point(point, t).unwrap(),
        PointLocation::Outside
    );
}
#[test]
fn steep_roof_and_rigid_trimmed_signed_geometry() {
    let t = tol();
    let source = NurbsGraphSolid::new([8., 6., 2.], 50., t.absolute()).unwrap();
    let body = source
        .through_xy_circle([4., 3.], 1., t.absolute())
        .unwrap();
    let Surface::Nurbs(roof) = &body.brep().shell.faces[1].surface else {
        panic!()
    };
    let p = roof.evaluate(0.1, 0.5).unwrap();
    let normal = roof.normal(0.1, 0.5).unwrap();
    for sign in [-1., 1.] {
        assert_eq!(
            body.classify_point(p + normal * (sign * 0.6 * t.linear()), t)
                .unwrap(),
            PointLocation::Boundary
        );
    }
    let pose = Transform::translation(Vec3::new(12., -7., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([8., 6., 3.], -2., t.absolute())
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], t.absolute())
        .unwrap()
        .transformed(pose, t.absolute())
        .unwrap();
    let body = source
        .through_xy_circle([4., 3.], 1., t.absolute())
        .unwrap();
    for (p, location) in [
        (Point3::new(2., 2., 1.), PointLocation::Inside),
        (Point3::new(4., 3., 1.), PointLocation::Outside),
        (Point3::new(5., 3., 1.), PointLocation::Boundary),
    ] {
        assert_eq!(body.classify_point(pose.point(p), t).unwrap(), location);
    }
}
#[test]
fn precision_and_public_mutation_never_succeed_as_classification() {
    let t = tol();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t.absolute()).unwrap();
    let body = source
        .through_xy_circle([4., 3.], 1., t.absolute())
        .unwrap();
    assert!(body
        .classify_point(
            Point3::new(2., 2., 1.),
            GeometryTolerance::new(1e-20, 1e-12, 0.).unwrap()
        )
        .is_err());
    assert!(body
        .classify_point(Point3::new(f64::NAN, 0., 0.), t)
        .is_err());
    assert!(body.classify_point(Point3::new(1e100, 0., 0.), t).is_err());
    let mut bad = body;
    bad.solid.vertices[8].point.x = bad.solid.vertices[8].point.x.next_up();
    assert!(bad.classify_point(Point3::new(-100., 0., 0.), t).is_err());
}
