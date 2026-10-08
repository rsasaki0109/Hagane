use hagane::*;
fn plane() -> Surface {
    Surface::Plane {
        origin: Vec3::new(0.0, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    }
}
#[test]
fn orientation_recovers_a_sign_lost_by_naive_arithmetic() {
    let n = 134217728.0;
    assert_eq!((n + 1.0) * (n - 1.0) - n * n, 0.0);
    assert_eq!(
        orient2d([0.0, 0.0], [n + 1.0, n], [n, n - 1.0]).unwrap(),
        Orientation::Clockwise
    );
    for exponent in [-1000, -500, 0, 500, 900] {
        let s = 2.0f64.powi(exponent);
        assert_eq!(
            orient2d([0.0, 0.0], [(n + 1.0) * s, n * s], [n * s, (n - 1.0) * s]).unwrap(),
            Orientation::Clockwise
        );
    }
}
#[test]
fn orientation_handles_full_f64_exponent_range_and_signed_zero() {
    let tiny = f64::from_bits(1);
    for s in [tiny, f64::MIN_POSITIVE, 1e-200, 1.0, 1e200, f64::MAX] {
        assert_eq!(
            orient2d([0.0, -0.0], [s, 0.0], [0.0, s]).unwrap(),
            Orientation::CounterClockwise
        );
        assert_eq!(
            orient2d([0.0, 0.0], [0.0, s], [s, 0.0]).unwrap(),
            Orientation::Clockwise
        );
        assert_eq!(
            orient2d([s, 0.0], [s, 0.0], [0.0, s]).unwrap(),
            Orientation::Collinear
        );
    }
    assert_eq!(
        orient2d(
            [-f64::MAX, -f64::MAX],
            [f64::MAX, -f64::MAX],
            [f64::MAX, f64::MAX]
        )
        .unwrap(),
        Orientation::CounterClockwise
    );
    assert_eq!(
        orient2d([tiny, -f64::MAX], [2.0 * tiny, -f64::MAX], [tiny, f64::MAX]).unwrap(),
        Orientation::CounterClockwise
    );
    assert!(orient2d([f64::NAN, 0.0], [0.0, 0.0], [1.0, 1.0]).is_err());
    assert!(orient2d([0.0, 0.0], [f64::INFINITY, 0.0], [1.0, 1.0]).is_err());
}
#[test]
fn orientation_matches_independent_i128_determinants() {
    let mut seed = 0x13ec42u64;
    for _ in 0..10000 {
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 12) as i128) - (1i128 << 51)
        };
        let (a, b, c) = ([next(), next()], [next(), next()], [next(), next()]);
        let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        assert_eq!(
            orient2d(
                a.map(|v| v as f64),
                b.map(|v| v as f64),
                c.map(|v| v as f64)
            )
            .unwrap()
            .sign(),
            det.signum() as i32
        );
    }
}
#[test]
fn exact_segment_contact_overlap_crossing_and_degenerate_points() {
    let cases = [
        ([[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]], true),
        ([[0.0, 0.0], [2.0, 0.0], [2.0, 0.0], [3.0, 1.0]], true),
        ([[0.0, 0.0], [3.0, 0.0], [1.0, 0.0], [2.0, 0.0]], true),
        ([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]], false),
        ([[0.0, 0.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]], true),
        ([[0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0]], false),
        ([[1.0, 1.0], [1.0, 1.0], [0.0, 0.0], [2.0, 2.0]], true),
        (
            [
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0 + f64::EPSILON, 0.0],
                [2.0, 0.0],
            ],
            false,
        ),
    ];
    for ([a, b, c, d], expected) in cases {
        assert_eq!(segments_intersect2d(a, b, c, d).unwrap(), expected);
        assert_eq!(segments_intersect2d(d, c, b, a).unwrap(), expected);
    }
    assert!(segments_intersect2d([0.0, 0.0], [1.0, 1.0], [f64::NAN, 0.0], [2.0, 0.0]).is_err());
}
#[test]
fn point_location_distinguishes_boundary_and_uses_half_open_crossings() {
    let polygon = [
        [0.0, 0.0],
        [4.0, 0.0],
        [4.0, 2.0],
        [2.0, 2.0],
        [2.0, 4.0],
        [0.0, 4.0],
    ];
    for (p, expected) in [
        ([1.0, 1.0], PointLocation::Inside),
        ([1.0, 2.0], PointLocation::Inside),
        ([3.0, 3.0], PointLocation::Outside),
        ([-1.0, 2.0], PointLocation::Outside),
        ([4.0, 1.0], PointLocation::Boundary),
        ([2.0, 2.0], PointLocation::Boundary),
        ([0.0, 0.0], PointLocation::Boundary),
        ([4.0, 3.0], PointLocation::Outside),
    ] {
        assert_eq!(locate_point_in_polygon(p, &polygon).unwrap(), expected);
        let reverse: Vec<_> = polygon.into_iter().rev().collect();
        assert_eq!(locate_point_in_polygon(p, &reverse).unwrap(), expected);
    }
    for s in [f64::from_bits(4), 1e-200, 1e200] {
        let triangle = [[0.0, 0.0], [s, 0.0], [0.0, s]];
        assert_eq!(
            locate_point_in_polygon([s * 0.25, s * 0.25], &triangle).unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            locate_point_in_polygon([s, s], &triangle).unwrap(),
            PointLocation::Outside
        );
    }
    assert!(locate_point_in_polygon([0.0, 0.0], &[]).is_err());
    assert!(locate_point_in_polygon([f64::NAN, 0.0], &polygon).is_err());
}
#[test]
fn tolerance_budgets_are_validated_and_have_distinct_units() {
    let t = GeometryTolerance::new(1e-6, 1e-4, 1e-8).unwrap();
    assert_eq!(t.length_at_scale(0.0).unwrap(), 1e-6);
    assert_eq!(t.length_at_scale(1e6).unwrap(), 0.01);
    assert_eq!(t.absolute().linear, 1e-6);
    assert!(t
        .coincident(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.001, 0.0, 0.0), 1e6)
        .unwrap());
    assert!(!t
        .coincident(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.001, 0.0, 0.0), 1.0)
        .unwrap());
    for (linear, angular, relative) in [
        (0.0, 1e-4, 0.0),
        (1.0, 0.0, 0.0),
        (1.0, 2.0, 0.0),
        (1.0, 1e-4, -1.0),
        (1.0, 1e-4, 1.0),
        (1.0, f64::NAN, 0.0),
        (1.0, 1e-4, f64::INFINITY),
    ] {
        assert!(GeometryTolerance::new(linear, angular, relative).is_err());
    }
    assert!(t.length_at_scale(-1.0).is_err());
    assert!(t.length_at_scale(f64::INFINITY).is_err());
    assert!(GeometryTolerance::try_from(Tolerance { linear: f64::NAN }).is_err());
    assert!(t
        .coincident(
            Vec3::new(f64::MAX, 0.0, 0.0),
            Vec3::new(-f64::MAX, 0.0, 0.0),
            1.0
        )
        .is_err());
}
#[test]
fn angular_tests_are_independent_of_vector_magnitude_and_length_units() {
    for magnitude in [f64::from_bits(1), f64::MAX] {
        let unit = Vec3::new(magnitude, magnitude, magnitude)
            .normalized()
            .unwrap();
        assert!((unit.norm() - 1.0).abs() < 1e-14);
    }
    let t = GeometryTolerance::new(1e8, 1e-4, 0.0).unwrap();
    for scale in [1e-200, 1.0, 1e200] {
        assert!(t
            .parallel(
                Vec3::new(scale, 0.0, 0.0),
                Vec3::new(-scale, scale * 1e-5, 0.0)
            )
            .unwrap());
        assert!(!t
            .parallel(
                Vec3::new(scale, 0.0, 0.0),
                Vec3::new(scale, scale * 1e-3, 0.0)
            )
            .unwrap());
        assert!(t
            .perpendicular(
                Vec3::new(scale, 0.0, 0.0),
                Vec3::new(scale * 1e-5, scale, 0.0)
            )
            .unwrap());
    }
    assert!(t
        .parallel(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
        .is_err());
    let axes = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1e-12, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    for linear in [1e-18, 1.0, 1e10] {
        assert!(Transform::new(
            Vec3::new(0.0, 0.0, 0.0),
            axes,
            Tolerance::new(linear).unwrap()
        )
        .is_ok());
    }
    assert!(Transform::new_with_tolerance(
        Vec3::new(0.0, 0.0, 0.0),
        axes,
        GeometryTolerance::new(1.0, 1e-14, 0.0).unwrap()
    )
    .is_err());
}
#[test]
fn line_plane_distinguishes_parallel_coincident_and_unique_intersection() {
    let p = plane();
    let t = GeometryTolerance::default();
    assert_eq!(
        intersect_line_plane(Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0), &p, t).unwrap(),
        LinePlaneIntersection::Parallel
    );
    assert_eq!(
        intersect_line_plane(Vec3::new(0.0, 0.0, 1e-9), Vec3::new(1.0, 0.0, 0.0), &p, t).unwrap(),
        LinePlaneIntersection::Coincident
    );
    for magnitude in [1e-200, 1.0, 1e200] {
        let LinePlaneIntersection::Point { point, parameter } = intersect_line_plane(
            Vec3::new(2.0, 3.0, 4.0),
            Vec3::new(0.0, 0.0, -magnitude),
            &p,
            t,
        )
        .unwrap() else {
            panic!("expected point")
        };
        assert_eq!(point, Vec3::new(2.0, 3.0, 0.0));
        assert!((parameter * magnitude - 4.0).abs() < 1e-14);
    }
}
#[test]
fn near_parallel_classification_respects_configured_angle_and_relative_distance() {
    let p = plane();
    let d = Vec3::new(1.0, 0.0, -1e-12);
    assert_eq!(
        intersect_line_plane(
            Vec3::new(0.0, 0.0, 1.0),
            d,
            &p,
            GeometryTolerance::default()
        )
        .unwrap(),
        LinePlaneIntersection::Parallel
    );
    let precise = GeometryTolerance::new(1e-8, 1e-14, 0.0).unwrap();
    let LinePlaneIntersection::Point { point, .. } =
        intersect_line_plane(Vec3::new(0.0, 0.0, 1.0), d, &p, precise).unwrap()
    else {
        panic!("expected point")
    };
    assert!((point.x - 1e12).abs() < 0.01);
    assert!(point.z.abs() < 1e-8);
    assert!(line_plane(Vec3::new(0.0, 0.0, 1.0), d, &p).is_err());
    let a = Vec3::new(1e8, 0.0, 0.001);
    assert_eq!(
        intersect_line_plane(a, d, &p, GeometryTolerance::default()).unwrap(),
        LinePlaneIntersection::Coincident
    );
    assert_eq!(
        intersect_line_plane(a, d, &p, GeometryTolerance::new(1e-8, 1e-10, 0.0).unwrap()).unwrap(),
        LinePlaneIntersection::Parallel
    );
}
#[test]
fn invalid_or_unrepresentable_intersections_are_errors() {
    let p = plane();
    let t = GeometryTolerance::default();
    assert!(intersect_line_plane(
        Vec3::new(f64::NAN, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        &p,
        t
    )
    .is_err());
    assert!(
        intersect_line_plane(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0), &p, t).is_err()
    );
    assert!(intersect_line_plane(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -f64::from_bits(1)),
        &p,
        t
    )
    .is_err());
    assert!(intersect_line_plane(
        Vec3::new(0.0, 0.0, f64::MAX),
        Vec3::new(1.0, 0.0, -0.1),
        &p,
        t
    )
    .is_err());
    let invalid = Surface::Plane {
        origin: Vec3::new(0.0, 0.0, 0.0),
        u: Vec3::new(2.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    assert!(intersect_line_plane(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        &invalid,
        t
    )
    .is_err());
}
#[test]
fn polygon_modeling_keeps_metric_clearance_separate_from_exact_signs() {
    let outer = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
    let t = Tolerance::new(1e-6).unwrap();
    for (gap, success) in [(5e-7, false), (1e-6, false), (2e-6, true)] {
        let profile = PolygonProfile {
            origin: Vec3::new(0.0, 0.0, 0.0),
            outer: outer.clone(),
            holes: vec![vec![[gap, 1.0], [1.0, 1.0], [1.0, 2.0], [gap, 2.0]]],
        };
        assert_eq!(
            extrude_polygon(&profile, Vec3::new(0.0, 0.0, 1.0), t).is_ok(),
            success
        );
    }
    let crossing = PolygonProfile {
        origin: Vec3::new(0.0, 0.0, 0.0),
        outer: vec![[0.0, 0.0], [4.0, 4.0], [0.0, 4.0], [4.0, 0.0]],
        holes: vec![],
    };
    assert!(extrude_polygon(&crossing, Vec3::new(0.0, 0.0, 1.0), t).is_err());
    let far = PolygonProfile {
        origin: Vec3::new(0.0, 0.0, 0.0),
        outer: vec![[-f64::MAX, 0.0], [f64::MAX, 0.0], [0.0, 1.0]],
        holes: vec![],
    };
    assert!(extrude_polygon(&far, Vec3::new(0.0, 0.0, 1.0), t).is_err());
}

#[test]
fn intersection_normalization_handles_overflowing_norms_and_rejects_parameter_underflow() {
    let p = plane();
    let t = GeometryTolerance::default();
    let LinePlaneIntersection::Point { point, parameter } = intersect_line_plane(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(f64::MAX, 0.0, -f64::MAX),
        &p,
        t,
    )
    .unwrap() else {
        panic!("expected point")
    };
    assert!((point.x - 1.0).abs() < 1e-14 && point.z.abs() < 1e-14);
    assert!(parameter > 0.0);
    assert!(intersect_line_plane(
        Vec3::new(0.0, 0.0, -f64::from_bits(1)),
        Vec3::new(0.0, 0.0, f64::MAX),
        &p,
        t
    )
    .is_err());
}
