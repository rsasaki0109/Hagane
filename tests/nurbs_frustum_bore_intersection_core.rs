use hagane::*;
fn body() -> (NurbsBoredFrustumSolid, GeometryTolerance) {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    (
        NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
            .unwrap()
            .bore_coaxial(4., p)
            .unwrap(),
        p,
    )
}
#[test]
fn segment_crosses_actual_four_boundaries_and_two_material_intervals() {
    let (body, p) = body();
    let start = Point3::new(-20., 0., 12.);
    let end = Point3::new(20., 0., 12.);
    let result = body.intersect_segment(start, end, p).unwrap();
    assert_eq!(result.material_intervals.len(), 2);
    assert_eq!(result.hits.len(), 4);
    for (hit, t) in result.hits.iter().zip([0.2, 0.4, 0.6, 0.8]) {
        assert!((hit.parameter - t).abs() < 1e-12);
        assert_eq!(
            body.classify_point(hit.point, p).unwrap(),
            PointLocation::Boundary
        );
        for f in &hit.faces {
            let represented = body.solid().shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((represented - hit.point).norm() < 1e-10);
        }
    }
    assert!(result.hits[1..3]
        .iter()
        .all(|h| h.faces.iter().all(|f| f.face_id >= 6)));
    let reverse = body.intersect_segment(end, start, p).unwrap();
    for (a, b) in result.hits.iter().zip(reverse.hits.iter().rev()) {
        assert!((a.parameter + b.parameter - 1.).abs() < 1e-12);
        assert!((a.point - b.point).norm() < 1e-10);
    }
}
#[test]
fn cap_interval_cancellation_and_inside_material_are_not_fake_hits() {
    let (body, p) = body();
    let empty = body
        .intersect_segment(Point3::new(0., 0., -5.), Point3::new(0., 0., 29.), p)
        .unwrap();
    assert!(empty.hits.is_empty());
    assert!(empty.material_intervals.is_empty());
    let inside = body
        .intersect_segment(Point3::new(6., 0., 5.), Point3::new(8., 0., 10.), p)
        .unwrap();
    assert!(inside.hits.is_empty());
    assert_eq!(inside.material_intervals, vec![[0., 1.]]);
    assert!(body
        .intersect_segment(Point3::new(4., 0., 12.), Point3::new(8., 0., 12.), p)
        .is_err());
    assert!(body
        .intersect_segment(Point3::new(1e100, 0., 12.), Point3::new(-1e100, 0., 12.), p)
        .is_err());
}
