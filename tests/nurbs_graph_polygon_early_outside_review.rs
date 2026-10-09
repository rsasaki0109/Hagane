use hagane::*;
fn band(linear: f64) -> GeometryTolerance {
    GeometryTolerance::new(linear, 1e-10, 0.).unwrap()
}
fn square(a: f64, b: f64) -> Vec<[f64; 2]> {
    vec![[a, a], [b, a], [b, b], [a, b]]
}

#[test]
fn placed_removed_roof_has_resolved_outside_without_subdivision() {
    let t = Tolerance::default();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.4).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., t)
        .unwrap()
        .transformed(frame, t)
        .unwrap();
    let body = source
        .through_uv_polygon(vec![[0.3, 0.5], [0.5, 0.3], [0.7, 0.5], [0.5, 0.7]], t)
        .unwrap();
    for z in [0., 10., 27.5] {
        assert_eq!(
            body.classify_point(frame.point(Point3::new(40., 30., z)), band(1e-8))
                .unwrap(),
            PointLocation::Outside
        );
    }
}

#[test]
fn near_opening_wall_and_outer_corner_keep_actual_euclidean_boundary_band() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    let epsilon = 1e-5;
    for delta in [-0.6, 0., 0.6] {
        assert_eq!(
            body.classify_point(Point3::new(0.25 + delta * epsilon, 0.5, 0.5), band(epsilon))
                .unwrap(),
            PointLocation::Boundary
        );
    }
    let outer = NurbsGraphPolygonSolid::new(&source, square(0.25, 0.75), t).unwrap();
    assert_eq!(
        outer
            .classify_point(
                Point3::new(0.25 - 0.6 * epsilon, 0.25 - 0.6 * epsilon, 0.5),
                band(epsilon)
            )
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        outer
            .classify_point(
                Point3::new(0.25 - 0.8 * epsilon, 0.25 - 0.8 * epsilon, 0.5),
                band(epsilon)
            )
            .unwrap(),
        PointLocation::Outside
    );
}

#[test]
fn source_and_query_arithmetic_or_mutation_cannot_bypass_validation() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let mut body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    assert!(body
        .classify_point(Point3::new(1e15, 1e15, 0.5), band(1e-8))
        .is_err());
    body.solid.vertices[0].point.x += t.linear / 100.;
    assert!(body
        .classify_point(Point3::new(0.5, 0.5, 0.5), band(1e-5))
        .is_err());
}
