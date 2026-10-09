use hagane::*;
fn square(a: f64, b: f64) -> Vec<[f64; 2]> {
    vec![[a, a], [b, a], [b, b], [a, b]]
}
fn tolerance(linear: f64) -> GeometryTolerance {
    GeometryTolerance::new(linear, 1e-10, 0.).unwrap()
}
#[test]
fn actual_corner_band_is_euclidean_and_does_not_extend_side_planes() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, square(0.25, 0.75), t).unwrap();
    let band = 1e-5;
    for (delta, expected) in [
        (0.6, PointLocation::Boundary),
        (0.8, PointLocation::Outside),
    ] {
        assert_eq!(
            body.classify_point(
                Point3::new(0.25 - delta * band, 0.25 - delta * band, 0.5),
                tolerance(band)
            )
            .unwrap(),
            expected
        );
    }
    assert_eq!(
        body.classify_point(Point3::new(0.25, 0.1, 0.5), tolerance(band))
            .unwrap(),
        PointLocation::Outside
    );
}
#[test]
fn roof_and_base_over_open_void_are_not_material_boundary() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], -0.5, t).unwrap();
    let body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    for z in [0., 0.3, 0.875] {
        assert_eq!(
            body.classify_point(Point3::new(0.5, 0.5, z), tolerance(1e-6))
                .unwrap(),
            PointLocation::Outside
        );
    }
    let u = 0.125;
    let v = 0.5;
    let h = 1. - 2. * u * (1. - u) * v * (1. - v);
    for z in [0., h] {
        assert_eq!(
            body.classify_point(Point3::new(u, v, z), tolerance(1e-6))
                .unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        body.classify_point(Point3::new(0.25, 0.5, 0.5), tolerance(1e-6))
            .unwrap(),
        PointLocation::Boundary
    );
}
#[test]
fn relative_band_scales_with_local_geometry_and_thin_frame_is_inside() {
    for scale in [1e-6, 1., 100.] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([scale; 3], 0., t).unwrap();
        let body = NurbsGraphPolygonSolid::new(&source, square(0.25, 0.75), t).unwrap();
        let gt = GeometryTolerance::new(scale * 1e-8, 1e-10, 1e-5).unwrap();
        assert_eq!(
            body.classify_point(
                Point3::new(scale * (0.25 - 5e-6), scale * 0.5, scale * 0.5),
                gt
            )
            .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            body.classify_point(
                Point3::new(scale * (0.25 - 5e-5), scale * 0.5, scale * 0.5),
                gt
            )
            .unwrap(),
            PointLocation::Outside
        );
        let holed = source
            .through_uv_polygon(square(1e-4, 1. - 1e-4), t)
            .unwrap();
        assert_eq!(
            holed
                .classify_point(
                    Point3::new(scale * 5e-5, scale * 0.5, scale * 0.5),
                    tolerance(scale * 1e-6)
                )
                .unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn public_mutation_nonfinite_query_and_invalid_tolerance_fail() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let mut body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    assert!(body
        .classify_point(Point3::new(f64::NAN, 0.5, 0.5), tolerance(1e-6))
        .is_err());
    assert!(body
        .classify_point(Point3::new(0.5, f64::INFINITY, 0.5), tolerance(1e-6))
        .is_err());
    body.solid.shell.faces[0].wires[1].coedges[0].edge = 0;
    assert!(body
        .classify_point(Point3::new(0.1, 0.5, 0.5), tolerance(1e-6))
        .is_err());
    for value in [f64::NAN, f64::INFINITY, 0., -1.] {
        assert!(GeometryTolerance::new(value, 1e-10, 0.).is_err());
    }
}

#[test]
fn arbitrary_placement_preserves_membership_and_unresolved_query_is_rejected() {
    let t = Tolerance::default();
    let frame = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([1.; 3], 0., t)
        .unwrap()
        .transformed(frame, t)
        .unwrap();
    let body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    for (point, expected) in [
        (Point3::new(0.1, 0.5, 0.5), PointLocation::Inside),
        (Point3::new(0.5, 0.5, 0.5), PointLocation::Outside),
        (Point3::new(0.25, 0.5, 0.5), PointLocation::Boundary),
    ] {
        assert_eq!(
            body.classify_point(frame.point(point), tolerance(1e-6))
                .unwrap(),
            expected
        );
    }
    // Query roundoff also consumes the arithmetic allowance, even when the
    // local source itself has a well-resolved placement.
    match body.classify_point(Point3::new(1e15, 1e15, 1e15), tolerance(1e-6)) {
        Ok(location) => assert_eq!(location, PointLocation::Outside),
        Err(Error::Unsupported(_)) => {}
        Err(error) => panic!("unexpected error: {error:?}"),
    }
}

#[test]
fn steep_roof_normal_offsets_use_surface_distance_instead_of_vertical_gap() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 100., t).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, square(0.125, 0.875), t).unwrap();
    let u = 0.25;
    let point = Point3::new(u, 0.5, 1. + 100. * u * (1. - u));
    let normal = Vec3::new(-50., 0., 1.).normalized().unwrap();
    assert_eq!(
        body.classify_point(point, tolerance(1e-5)).unwrap(),
        PointLocation::Boundary
    );
    for sign in [-1., 1.] {
        match body.classify_point(point + normal * (sign * 0.5e-5), tolerance(1e-5)) {
            Ok(location) => assert_eq!(location, PointLocation::Boundary),
            Err(Error::Unsupported(_)) => {}
            Err(error) => panic!("unexpected error: {error:?}"),
        }
    }
}
