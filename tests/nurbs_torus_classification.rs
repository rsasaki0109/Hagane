use hagane::*;
#[test]
fn tube_distance_uses_actual_finite_boundary_witnesses() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], policy).unwrap();
    let original = body.export_step_mm(policy).unwrap();
    for (p, expected) in [
        (Point3::new(0., 0., 0.), PointLocation::Outside),
        (Point3::new(16., 0., 0.), PointLocation::Inside),
        (Point3::new(17., 0., 1.), PointLocation::Inside),
        (Point3::new(21., 0., 0.), PointLocation::Outside),
    ] {
        let q = body.point_query(p, policy).unwrap();
        assert_eq!(q.location, expected);
        assert!(q.boundary.is_none());
    }
    for face in &body.solid().shell.faces {
        for (u, v) in [(0., 0.), (0.27, 0.63), (1., 1.)] {
            let p = face.surface.try_evaluate(u, v).unwrap();
            let q = body.point_query(p, policy).unwrap();
            assert_eq!(q.location, PointLocation::Boundary);
            let w = q.boundary.unwrap();
            let actual = body.solid().shell.faces[w.face]
                .surface
                .try_evaluate(w.uv[0], w.uv[1])
                .unwrap();
            assert!((actual - w.point).norm() < 1e-12);
            assert!(w.distance < 1e-12);
        }
    }
    assert!(body
        .classify_point(Point3::new(f64::NAN, 0., 0.), policy)
        .is_err());
    assert!(body
        .classify_point(Point3::new(1e14, 0., 0.), policy)
        .is_err());
    assert_eq!(original, body.export_step_mm(policy).unwrap());
}
