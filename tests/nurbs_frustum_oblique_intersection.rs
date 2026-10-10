use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn fixture(scale: f64, posed: bool) -> (NurbsFrustumSolid, NurbsFrustumPlaneSplit, Frame3) {
    let frame = if posed {
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        Frame3::new_with_tolerance(
            Point3::new(13., -7., 5.) * scale,
            rotation.axes(),
            policy(scale),
        )
        .unwrap()
    } else {
        Frame3::IDENTITY
    };
    let source =
        NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
            .unwrap();
    let normal = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
    let u = Vec3::new(-normal.y, normal.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    let plane = Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 12. * scale)),
        u: vector(u),
        v: vector(normal.cross(u)),
    };
    let split = source.split_by_plane(&plane, policy(scale)).unwrap();
    (source, split, frame)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witnesses(
    child: &NurbsObliqueFrustumSolid,
    a: Point3,
    b: Point3,
    result: &NurbsFrustumSegmentIntersection,
    scale: f64,
) {
    for hit in &result.hits {
        assert!((hit.point - (a + (b - a) * hit.parameter)).norm() < 1e-7 * scale);
        for witness in &hit.faces {
            let face = &child.solid().shell.faces[witness.face_id];
            let exact = face
                .surface
                .try_evaluate(witness.uv[0], witness.uv[1])
                .unwrap();
            assert!(
                (exact - hit.point).norm() < 1e-7 * scale,
                "face {} UV {:?}",
                witness.face_id,
                witness.uv
            );
            let normal = face
                .surface
                .normal_at(witness.uv[0], witness.uv[1])
                .unwrap()
                * face.orientation as f64;
            let interval = result.material_interval.unwrap();
            let crossing = normal.dot(b - a);
            if (hit.parameter - interval[0]).abs() < 1e-9 {
                assert!(crossing < 0.);
            }
            if (hit.parameter - interval[1]).abs() < 1e-9 {
                assert!(crossing > 0.);
            }
        }
    }
}
#[test]
fn complementary_section_hits_and_intervals_match_stock_under_pose_scale_and_reversal() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let (source, split, frame) = fixture(scale, posed);
            let world = |p: Point3| frame.point(p * scale);
            for (a, b) in [
                (
                    world(Point3::new(2., 3., -5.)),
                    world(Point3::new(2., 3., 30.)),
                ),
                (
                    world(Point3::new(2., 3., 30.)),
                    world(Point3::new(2., 3., -5.)),
                ),
            ] {
                let stock = source.intersect_segment(a, b, policy(scale)).unwrap();
                let lower = split.lower.intersect_segment(a, b, policy(scale)).unwrap();
                let upper = split.upper.intersect_segment(a, b, policy(scale)).unwrap();
                witnesses(&split.lower, a, b, &lower, scale);
                witnesses(&split.upper, a, b, &upper, scale);
                let mut intervals = [
                    lower.material_interval.unwrap(),
                    upper.material_interval.unwrap(),
                ];
                intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
                close(intervals[0][0], stock.material_interval.unwrap()[0]);
                close(intervals[0][1], intervals[1][0]);
                close(intervals[1][1], stock.material_interval.unwrap()[1]);
                let left = lower
                    .hits
                    .iter()
                    .find(|h| h.faces.iter().any(|f| f.face_id == 1))
                    .unwrap();
                let right = upper
                    .hits
                    .iter()
                    .find(|h| h.faces.iter().any(|f| f.face_id == 0))
                    .unwrap();
                close(left.parameter, right.parameter);
                assert!((left.point - right.point).norm() < 1e-7 * scale);
                close(
                    left.parameter,
                    if (b - a).dot(frame.axes()[2]) > 0. {
                        16.65 / 35.
                    } else {
                        18.35 / 35.
                    },
                );
            }
        }
    }
}
#[test]
fn nonaxis_side_hits_use_actual_oblique_rational_v_and_retain_brep_and_step() {
    let (_, split, _) = fixture(1., false);
    for child in [&split.lower, &split.upper] {
        let z = if child.is_lower() { 8. } else { 16. };
        let d = Vec3::new(0.37_f64.cos(), 0.37_f64.sin(), 0.);
        let a = Point3::new(0., 0., z) - d * 20.;
        let b = Point3::new(0., 0., z) + d * 20.;
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(policy(1.)).unwrap();
        let result = child.intersect_segment(a, b, policy(1.)).unwrap();
        assert_eq!(result.hits.len(), 2);
        witnesses(child, a, b, &result, 1.);
        for hit in &result.hits {
            for witness in &hit.faces {
                assert!((2..6).contains(&witness.face_id));
                assert!(
                    (witness.uv[1] - z / 24.).abs() > 0.05,
                    "canonical v accidentally reused"
                );
            }
        }
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(policy(1.)).unwrap());
        let inside_a = Point3::new(0., 0., z);
        let inside_b = Point3::new(1., 1., z);
        assert_eq!(
            child
                .intersect_segment(inside_a, inside_b, policy(1.))
                .unwrap()
                .material_interval,
            Some([0., 1.])
        );
        let removed_z = if child.is_lower() { 18. } else { 6. };
        let empty = child
            .intersect_segment(
                Point3::new(-1., 0., removed_z),
                Point3::new(1., 0., removed_z),
                policy(1.),
            )
            .unwrap();
        assert!(empty.hits.is_empty());
        assert!(empty.material_interval.is_none());
    }
}
#[test]
fn precision_contacts_and_invalid_coordinates_refuse_and_recover() {
    let (_, split, _) = fixture(1., false);
    let child = &split.lower;
    let angle = 0.37_f64;
    let slope = 0.1 * angle.cos() + 0.05 * angle.sin();
    let rim_z = (12. - 16. * slope) / (1. - slope / 3.);
    let rim_r = 16. - rim_z / 3.;
    let rim = Point3::new(rim_r * angle.cos(), rim_r * angle.sin(), rim_z);
    for (a, b) in [
        (
            Point3::new(-20., 40. / 3., 8.),
            Point3::new(20., 40. / 3., 8.),
        ),
        (rim, Point3::new(0., 0., 8.)),
        (
            Point3::new(-1., 0., 12.1 + 1e-9),
            Point3::new(1., 0., 11.9 + 1e-9),
        ),
        (Point3::new(0., 0., 12.), Point3::new(0., 0., 8.)),
        (Point3::new(-20., 0., 0.), Point3::new(20., 0., 0.)),
        (Point3::new(-1e12, 0., 8.), Point3::new(1e12, 0., 8.)),
        (Point3::new(0., 0., 8.), Point3::new(1e-8, 0., 8.)),
        (Point3::new(f64::NAN, 0., 8.), Point3::new(1., 0., 8.)),
        (Point3::new(0., 0., 8.), Point3::new(f64::INFINITY, 0., 8.)),
    ] {
        assert!(
            child.intersect_segment(a, b, policy(1.)).is_err(),
            "{a:?} {b:?}"
        );
    }
    assert_eq!(
        child
            .intersect_segment(Point3::new(0., 0., 5.), Point3::new(0., 0., 8.), policy(1.))
            .unwrap()
            .material_interval,
        Some([0., 1.])
    );
}

#[test]
fn unresolved_cut_root_just_outside_source_exit_cannot_guess_empty_material() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy(1.)).unwrap();
    let normal = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u,
        v: normal.cross(u),
    };
    let split = source.split_by_plane(&plane, policy(1.)).unwrap();
    let rim_x = 12. / (1. - 0.1 / 3.);
    let rim_z = 12. - 0.1 * rim_x;
    for offset in [-1e-10, 1e-10] {
        let z = rim_z + offset;
        let start = Point3::new(-20., 0., z);
        let end = Point3::new(20., 0., z);
        // Below the rim the cut root lies just beyond the stock exit; above
        // it only an unresolved sliver remains. Neither admits an empty guess.
        assert!(matches!(
            split.upper.intersect_segment(start, end, policy(1.)),
            Err(Error::Unsupported(_))
        ));
    }
}
