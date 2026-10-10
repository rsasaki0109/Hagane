use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn body(radii: [f64; 2], height: f64) -> NurbsFrustumSolid {
    NurbsFrustumSolid::new(Frame3::IDENTITY, radii, height, policy(1.)).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn witnesses(
    shape: &NurbsFrustumSolid,
    start: Point3,
    end: Point3,
    result: &NurbsFrustumSegmentIntersection,
    tol: f64,
) {
    let mut previous = -1.;
    for hit in &result.hits {
        assert!(hit.parameter > previous && hit.parameter > 0. && hit.parameter < 1.);
        previous = hit.parameter;
        assert!((hit.point - (start + (end - start) * hit.parameter)).norm() < tol);
        assert!(!hit.faces.is_empty());
        for face in &hit.faces {
            let actual = shape.solid().shell.faces[face.face_id]
                .surface
                .try_evaluate(face.uv[0], face.uv[1])
                .unwrap();
            assert!((actual - hit.point).norm() < tol);
        }
    }
}
#[test]
fn resolved_linear_and_constant_side_polynomials_keep_cap_intervals() {
    let shape = body([2., 4.], 4.);
    let start = Point3::new(-1., 0., -1.);
    let end = Point3::new(3., 0., 7.);
    let result = shape.intersect_segment(start, end, policy(1.)).unwrap();
    assert_eq!(result.hits.len(), 2);
    let interval = result.material_interval.unwrap();
    close(interval[0], 0.125);
    close(interval[1], 0.625);
    assert_eq!(result.hits[0].faces[0].face_id, 0);
    assert_eq!(result.hits[1].faces[0].face_id, 1);
    witnesses(&shape, start, end, &result, 1e-7);
    let cylinder = body([2., 2.], 4.);
    let a = Point3::new(0., 0., -1.);
    let b = Point3::new(0., 0., 5.);
    let result = cylinder.intersect_segment(a, b, policy(1.)).unwrap();
    let interval = result.material_interval.unwrap();
    close(interval[0], 1. / 6.);
    close(interval[1], 5. / 6.);
    witnesses(&cylinder, a, b, &result, 1e-7);
    // Both endpoints are strictly inside the positive extended cone; convexity
    // proves the entire segment misses the side even with near-linear coefficients.
    let perturbed_end = Point3::new(3. + 1e-12, 0., 7.);
    let result = shape
        .intersect_segment(start, perturbed_end, policy(1.))
        .unwrap();
    let interval = result.material_interval.unwrap();
    close(interval[0], 0.125);
    close(interval[1], 0.625);
    assert_eq!(result.hits.len(), 2);
    witnesses(&shape, start, perturbed_end, &result, 1e-7);
    // This nearly generator-parallel line actually crosses the side. Its small
    // nonzero quadratic coefficient cannot use the cone-inside proof or be dropped.
    assert!(matches!(
        shape.intersect_segment(
            Point3::new(-4., 0., -1.),
            Point3::new(1e-12, 0., 7.),
            policy(1.)
        ),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn unresolved_contacts_and_endpoint_precision_never_return_fake_empty() {
    let shape = body([2., 4.], 4.);
    let tol = policy(1.);
    for (start, end) in [
        (Point3::new(-1e12, 0., 2.), Point3::new(1e12, 0., 2.)),
        (Point3::new(0., 0., 2.), Point3::new(1e-8, 0., 2.)),
        (Point3::new(1.5, 0., -1.), Point3::new(4.5, 0., 5.)),
        (Point3::new(-5., 0., 0.), Point3::new(5., 0., 0.)),
        (Point3::new(0., 0., 2.), Point3::new(3., 0., 2.)),
        (Point3::new(-5., 3., 2.), Point3::new(5., 3., 2.)),
        (
            Point3::new(-5., 3. - 1e-7, 2.),
            Point3::new(5., 3. - 1e-7, 2.),
        ),
        (
            Point3::new(-5., 3. + 1e-7, 2.),
            Point3::new(5., 3. + 1e-7, 2.),
        ),
    ] {
        assert!(
            matches!(
                shape.intersect_segment(start, end, tol),
                Err(Error::Unsupported(_))
            ),
            "unexpected success for {start:?} -> {end:?}"
        );
    }
    assert!(matches!(
        shape.intersect_segment(Point3::new(0., 0., 2.), Point3::new(0., 0., 2.), tol),
        Err(Error::InvalidInput(_))
    ));
    assert!(shape
        .intersect_segment(Point3::new(f64::NAN, 0., 2.), Point3::new(1., 0., 2.), tol)
        .is_err());
    let result = shape
        .intersect_segment(Point3::new(-5., 0., 2.), Point3::new(5., 0., 2.), tol)
        .unwrap();
    let interval = result.material_interval.unwrap();
    close(interval[0], 0.2);
    close(interval[1], 0.8);
}
#[test]
fn contracting_and_expanding_stock_are_scale_and_rigid_covariant() {
    for scale in [1e-4, 1., 1e3] {
        for radii in [[2., 4.], [4., 2.]] {
            let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.37).unwrap();
            let frame = Frame3::new_with_tolerance(
                Point3::new(13. * scale, -7. * scale, 5. * scale),
                rotation.axes(),
                policy(scale),
            )
            .unwrap();
            let shape =
                NurbsFrustumSolid::new(frame, radii.map(|r| r * scale), 4. * scale, policy(scale))
                    .unwrap();
            let start = frame.point(Point3::new(-5. * scale, 0., 2. * scale));
            let end = frame.point(Point3::new(5. * scale, 0., 2. * scale));
            let result = shape.intersect_segment(start, end, policy(scale)).unwrap();
            let interval = result.material_interval.unwrap();
            close(interval[0], 0.2);
            close(interval[1], 0.8);
            witnesses(&shape, start, end, &result, 1e-7 * scale);
            let reverse = shape.intersect_segment(end, start, policy(scale)).unwrap();
            close(reverse.material_interval.unwrap()[0], 1. - interval[1]);
            close(reverse.material_interval.unwrap()[1], 1. - interval[0]);
            for (a, b) in result.hits.iter().zip(reverse.hits.iter().rev()) {
                assert!((a.point - b.point).norm() < 1e-7 * scale);
            }
        }
    }
}
#[test]
fn endpoint_material_membership_matches_returned_intervals_without_mesh() {
    let shape = body([2., 4.], 4.);
    let cases = [
        (
            Point3::new(0., 0., 2.),
            Point3::new(5., 0., 2.),
            Some([0., 0.6]),
            1,
        ),
        (
            Point3::new(-5., 0., 2.),
            Point3::new(0., 0., 2.),
            Some([0.4, 1.]),
            1,
        ),
        (
            Point3::new(-1., 0., 2.),
            Point3::new(1., 0., 2.),
            Some([0., 1.]),
            0,
        ),
        (Point3::new(-5., 5., 2.), Point3::new(5., 5., 2.), None, 0),
    ];
    for (start, end, expected, hits) in cases {
        let result = shape.intersect_segment(start, end, policy(1.)).unwrap();
        assert_eq!(result.hits.len(), hits);
        match (expected, result.material_interval) {
            (Some(a), Some(b)) => {
                close(a[0], b[0]);
                close(a[1], b[1]);
            }
            (None, None) => {}
            _ => panic!("wrong material interval"),
        };
        witnesses(&shape, start, end, &result, 1e-7);
    }
}

#[test]
fn steep_admitted_frusta_keep_cap_only_intersections_under_arbitrary_pose() {
    let tolerance = GeometryTolerance::new(1e-8, 1e-10, 0.).unwrap();
    for angle in [0., 0.37] {
        let rotation = Transform::rotation(Vec3::new(0., 1., 0.), angle).unwrap();
        let frame =
            Frame3::new_with_tolerance(Point3::new(13., -7., 5.), rotation.axes(), tolerance)
                .unwrap();
        let shape = NurbsFrustumSolid::new(frame, [16., 8.], 1e-4, tolerance).unwrap();
        let start = frame.point(Point3::new(0., 0., -0.5e-4));
        let end = frame.point(Point3::new(0., 0., 1.5e-4));
        let result = shape.intersect_segment(start, end, tolerance).unwrap();
        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.hits[0].faces[0].face_id, 0);
        assert_eq!(result.hits[1].faces[0].face_id, 1);
        let interval = result.material_interval.unwrap();
        close(interval[0], 0.25);
        close(interval[1], 0.75);
        witnesses(&shape, start, end, &result, 2.5e-9);
    }
}
