use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn plane(depth: f64, a: f64, b: f64) -> Surface {
    let n = Vec3::new(a, b, 1.).normalized().unwrap();
    let u = (Vec3::new(0., 1., 0.) - n * n.y).normalized().unwrap();
    Surface::Plane {
        origin: Point3::new(0., 0., depth),
        u,
        v: n.cross(u),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witnesses(solid: &Solid, hits: &[NurbsFrustumSegmentHit], delta: Vec3) {
    for hit in hits {
        assert!(!hit.faces.is_empty());
        for witness in &hit.faces {
            let face = &solid.shell.faces[witness.face_id];
            let point = face
                .surface
                .try_evaluate(witness.uv[0], witness.uv[1])
                .unwrap();
            let error = point - hit.point;
            assert!(error.x.hypot(error.y).hypot(error.z) < 1e-8);
            let crossing = (face
                .surface
                .normal_at(witness.uv[0], witness.uv[1])
                .unwrap()
                * f64::from(face.orientation))
            .dot(delta);
            assert!(crossing.abs() > 1e-3);
        }
    }
}

#[test]
fn skew_finite_caps_and_original_parameter_queries_use_actual_bodies() {
    let p = policy();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12.; 2], 40., p).unwrap();
    let upper = plane(24., -0.1, 0.15);
    let lower = plane(12., 0.2, 0.1);
    let body = source
        .split_by_plane(&upper, p)
        .unwrap()
        .lower
        .split_by_interior_plane(&lower, p)
        .unwrap()
        .upper;
    for (point, expected) in [
        (Point3::new(6., 0., 18.), PointLocation::Inside),
        (Point3::new(6., 0., 10.), PointLocation::Outside),
        (Point3::new(6., 0., 28.), PointLocation::Outside),
        (Point3::new(6., 0., 10.8), PointLocation::Boundary),
        (Point3::new(6., 0., 24.6), PointLocation::Boundary),
        (Point3::new(12., 0., 18.), PointLocation::Boundary),
        (Point3::new(14., 0., 9.2), PointLocation::Outside),
        (Point3::new(6., 0., 0.), PointLocation::Outside),
    ] {
        assert_eq!(body.classify_point(point, p).unwrap(), expected);
    }
    assert!(matches!(
        body.classify_point(Point3::new(12., 0., 9.6), p),
        Err(Error::Unsupported(_))
    ));
    let start = Point3::new(6., 1., -2.);
    let end = Point3::new(6., 1., 42.);
    let segment = body.intersect_segment(start, end, p).unwrap();
    let interval = segment.material_interval.unwrap();
    close(interval[0], 12.7 / 44.);
    close(interval[1], 26.45 / 44.);
    assert_eq!(segment.hits.len(), 2);
    assert_eq!(segment.hits[0].faces[0].face_id, 0);
    assert_eq!(segment.hits[1].faces[0].face_id, 1);
    witnesses(body.solid(), &segment.hits, end - start);
    let line = body
        .intersect_line(Point3::new(6., 1., 0.), Vec3::new(0., 0., 2.), p)
        .unwrap();
    close(line.material_intervals[0][0], 10.7 / 2.);
    close(line.material_intervals[0][1], 24.45 / 2.);
    assert_eq!(line.hits.len(), 2);
}

#[test]
fn skew_annular_queries_preserve_the_void_face_witnesses_and_geometry_on_errors() {
    let p = policy();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12.; 2], 40., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let body = source
        .split_by_plane(&plane(24., -0.1, 0.15), p)
        .unwrap()
        .lower
        .split_by_interior_plane(&plane(12., 0.2, 0.1), p)
        .unwrap()
        .upper;
    assert_eq!(
        body.classify_point(Point3::new(0., 0., 12.), p).unwrap(),
        PointLocation::Outside
    );
    assert!(matches!(
        body.classify_point(Point3::new(4., 0., 11.2), p),
        Err(Error::Unsupported(_))
    ));
    let start = Point3::new(-20., 1., 18.);
    let end = Point3::new(20., 1., 18.);
    let segment = body.intersect_segment(start, end, p).unwrap();
    assert_eq!(segment.hits.len(), 4);
    assert_eq!(segment.material_intervals.len(), 2);
    let outer = 143f64.sqrt();
    let inner = 15f64.sqrt();
    for (actual, expected) in segment.material_intervals.iter().zip([
        [(20. - outer) / 40., (20. - inner) / 40.],
        [(20. + inner) / 40., (20. + outer) / 40.],
    ]) {
        close(actual[0], expected[0]);
        close(actual[1], expected[1]);
    }
    witnesses(body.solid(), &segment.hits, end - start);
    let origin = Point3::new(0., 1., 18.);
    let ray = body
        .intersect_ray(origin, Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(ray.hits.len(), 2);
    assert_eq!(ray.material_intervals.len(), 1);
    close(ray.material_intervals[0][0], inner / 2.);
    close(ray.material_intervals[0][1], outer / 2.);
    let before = format!("{:?}", body.solid());
    let step = body.export_step_mm(p).unwrap();
    for (anchor, direction) in [
        (Point3::new(0., 12., 18.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 1., 11.9), Vec3::new(1., 0., -0.2)),
        (Point3::new(1e15, 1., 18.), Vec3::new(1., 0., 0.)),
        (origin, Vec3::new(0., 0., 0.)),
    ] {
        assert!(body.intersect_line(anchor, direction, p).is_err());
    }
    assert!(body
        .intersect_segment(Point3::new(6., 1., 0.), Point3::new(6., 1., 40.), p)
        .is_err());
    assert_eq!(before, format!("{:?}", body.solid()));
    assert_eq!(step, body.export_step_mm(p).unwrap());
    assert_eq!(body.intersect_segment(start, end, p).unwrap().hits.len(), 4);
}
