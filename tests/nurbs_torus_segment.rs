use hagane::*;
#[test]
fn diameter_returns_four_actual_crossings_and_two_intervals() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], policy).unwrap();
    let start = Point3::new(-24., 0., 0.);
    let end = Point3::new(24., 0., 0.);
    let result = body.intersect_segment(start, end, policy).unwrap();
    assert_eq!(result.hits.len(), 4);
    assert_eq!(result.material_intervals.len(), 2);
    for (hit, x) in result.hits.iter().zip([-20., -12., 12., 20.]) {
        assert!((hit.point.x - x).abs() < 1e-7);
        assert!((hit.parameter - (x + 24.) / 48.).abs() < 1e-8);
        let surface = &body.solid().shell.faces[hit.boundary.face].surface;
        assert!(
            (surface
                .try_evaluate(hit.boundary.uv[0], hit.boundary.uv[1])
                .unwrap()
                - hit.boundary.point)
                .norm()
                < 1e-12
        );
    }
    let reverse = body.intersect_segment(end, start, policy).unwrap();
    for (a, b) in result.hits.iter().zip(reverse.hits.iter().rev()) {
        assert!((a.parameter + b.parameter - 1.).abs() < 1e-9);
    }
    let inside = body
        .intersect_segment(Point3::new(16., 0., 0.), Point3::new(16., 0., 1.), policy)
        .unwrap();
    assert_eq!(inside.material_intervals, vec![[0., 1.]]);
    let hole = body
        .intersect_segment(Point3::new(0., 0., -5.), Point3::new(0., 0., 5.), policy)
        .unwrap();
    assert!(hole.hits.is_empty() && hole.material_intervals.is_empty());
    assert!(body
        .intersect_segment(
            Point3::new(-24., 20., 0.),
            Point3::new(24., 20., 0.),
            policy
        )
        .is_err());
}
