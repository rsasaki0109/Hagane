use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn fixture(s: f64, posed: bool) -> (NurbsBoredFrustumPlaneSplit, Frame3) {
    let rotation =
        Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), policy(s))
            .unwrap();
    let source = NurbsFrustumSolid::new(frame, [12. * s, 12. * s], 24. * s, policy(s))
        .unwrap()
        .bore_coaxial(4. * s, policy(s))
        .unwrap();
    let n = Vec3::new(0.2, 0.1, 1.).normalized().unwrap();
    let u = Vec3::new(-n.y, n.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    let plane = Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 12. * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    };
    (source.split_by_plane(&plane, policy(s)).unwrap(), frame)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witnesses(
    child: &NurbsBoredObliqueFrustumSolid,
    a: Point3,
    b: Point3,
    result: &NurbsBoredFrustumSegmentIntersection,
    s: f64,
) {
    for h in &result.hits {
        assert!((h.point - (a + (b - a) * h.parameter)).norm() < 1e-7 * s);
        assert!(!h.faces.is_empty());
        for w in &h.faces {
            let face = &child.solid().shell.faces[w.face_id];
            let actual = face.surface.try_evaluate(w.uv[0], w.uv[1]).unwrap();
            assert!((actual - h.point).norm() < 1e-7 * s);
            let n = face.surface.normal_at(w.uv[0], w.uv[1]).unwrap() * face.orientation as f64;
            for interval in &result.material_intervals {
                if (h.parameter - interval[0]).abs() < 1e-9 {
                    assert!(n.dot(b - a) < 0.);
                }
                if (h.parameter - interval[1]).abs() < 1e-9 {
                    assert!(n.dot(b - a) > 0.);
                }
            }
        }
    }
}
#[test]
fn analytic_diameter_void_cut_and_annular_cut_hits_use_actual_retained_faces() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let (split, frame) = fixture(s, posed);
            let world = |p: Point3| frame.point(p * s);
            for (z, expected_lower, expected_upper) in [
                (6., vec![[0.2, 0.4], [0.6, 0.8]], vec![]),
                (12., vec![[0.2, 0.4]], vec![[0.6, 0.8]]),
            ] {
                for (child, expected) in [
                    (&split.lower, expected_lower),
                    (&split.upper, expected_upper),
                ] {
                    for reverse in [false, true] {
                        let mut a = world(Point3::new(-20., 0., z));
                        let mut b = world(Point3::new(20., 0., z));
                        if reverse {
                            std::mem::swap(&mut a, &mut b);
                        }
                        let before = format!("{:?}", child.solid());
                        let step = child.export_step_mm(policy(s)).unwrap();
                        let r = child.intersect_segment(a, b, policy(s)).unwrap();
                        let expected: Vec<_> = if reverse {
                            expected
                                .iter()
                                .rev()
                                .map(|i| [1. - i[1], 1. - i[0]])
                                .collect()
                        } else {
                            expected.clone()
                        };
                        assert_eq!(r.material_intervals.len(), expected.len());
                        for (actual, expected) in r.material_intervals.iter().zip(expected) {
                            close(actual[0], expected[0]);
                            close(actual[1], expected[1]);
                        }
                        witnesses(child, a, b, &r, s);
                        assert_eq!(before, format!("{:?}", child.solid()));
                        assert_eq!(step, child.export_step_mm(policy(s)).unwrap());
                    }
                }
            }
            // A transverse slanted material query meets the true annular cut cap.
            let a = world(Point3::new(5., 0., -5.));
            let b = world(Point3::new(7., 0., 30.));
            let r = split.lower.intersect_segment(a, b, policy(s)).unwrap();
            assert_eq!(r.material_intervals.len(), 1);
            close(r.material_intervals[0][0], 5. / 35.);
            close(r.material_intervals[0][1], 16. / 35.4);
            assert!(r
                .hits
                .iter()
                .any(|h| h.faces.iter().any(|f| f.face_id == 1)));
            witnesses(&split.lower, a, b, &r, s);
        }
    }
}
#[test]
fn unresolved_interval_end_contacts_and_original_precision_never_guess_empty() {
    let (split, frame) = fixture(1., false);
    let child = &split.lower;
    let world = |p| frame.point(p);
    let before = format!("{:?}", child.solid());
    for (a, b) in [
        (Point3::new(-20., 4., 6.), Point3::new(20., 4., 6.)),
        (Point3::new(4., 0., 6.), Point3::new(20., 0., 6.)),
        (Point3::new(-20., 0., 0.), Point3::new(20., 0., 0.)),
        (
            Point3::new(-20., 0., 9.6 + 1e-10),
            Point3::new(20., 0., 9.6 + 1e-10),
        ),
        (
            Point3::new(-20., 0., 9.6 - 1e-10),
            Point3::new(20., 0., 9.6 - 1e-10),
        ),
        (Point3::new(6., 0., 6.), Point3::new(6. + 1e-8, 0., 6.)),
        (Point3::new(-1e12, 0., 6.), Point3::new(1e12, 0., 6.)),
        (Point3::new(f64::NAN, 0., 6.), Point3::new(20., 0., 6.)),
        (
            Point3::new(-20., 0., 6.),
            Point3::new(f64::INFINITY, 0., 6.),
        ),
    ] {
        assert!(
            child
                .intersect_segment(world(a), world(b), policy(1.))
                .is_err(),
            "{a:?} {b:?}"
        );
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(child
        .intersect_segment(
            world(Point3::new(-20., 4. + 0.01, 6.)),
            world(Point3::new(20., 4. + 0.01, 6.)),
            relative
        )
        .is_err());
    assert_eq!(before, format!("{:?}", child.solid()));
    assert_eq!(
        child
            .intersect_segment(
                world(Point3::new(5., 0., 6.)),
                world(Point3::new(6., 0., 6.)),
                policy(1.)
            )
            .unwrap()
            .material_intervals,
        vec![[0., 1.]]
    );
}
