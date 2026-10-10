use hagane::*;

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn body(radii: [f64; 2], scale: f64, pose: bool) -> NurbsFrustumSolid {
    let frame = if pose {
        Transform::translation(Vec3::new(12. * scale, -3. * scale, 5. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap()
    } else {
        Frame3::IDENTITY
    };
    NurbsFrustumSolid::new(frame, radii.map(|r| r * scale), 24. * scale, policy(scale)).unwrap()
}
fn world(body: &NurbsFrustumSolid, local: Point3, scale: f64) -> Point3 {
    body.frame().point(local * scale)
}
fn radial(r: f64, z: f64) -> Point3 {
    Point3::new(r * 0.37_f64.cos(), r * 0.37_f64.sin(), z)
}
fn check(
    body: &NurbsFrustumSolid,
    a: Point3,
    b: Point3,
    expected: &[f64],
    interval: Option<[f64; 2]>,
    scale: f64,
) {
    let result = body
        .intersect_segment(a, b, policy(scale))
        .unwrap_or_else(|e| panic!("segment {a:?} -> {b:?}, scale{scale}: {e}"));
    assert_eq!(result.hits.len(), expected.len());
    for (hit, t) in result.hits.iter().zip(expected) {
        assert!(
            (hit.parameter - t).abs() < 1e-10,
            "{} vs {t}",
            hit.parameter
        );
        assert!((hit.point - (a + (b - a) * *t)).norm() < 1e-8 * scale);
        assert!(!hit.faces.is_empty());
        for witness in &hit.faces {
            let surface = &body.solid().shell.faces[witness.face_id].surface;
            assert!(
                (surface.try_evaluate(witness.uv[0], witness.uv[1]).unwrap() - hit.point).norm()
                    < 1e-8 * scale
            );
        }
    }
    match (result.material_interval, interval) {
        (Some(actual), Some(expected)) => {
            for i in 0..2 {
                assert!((actual[i] - expected[i]).abs() < 1e-10);
            }
            let t = (actual[0] + actual[1]) * 0.5;
            assert_eq!(
                body.classify_point(a + (b - a) * t, policy(scale)).unwrap(),
                PointLocation::Inside
            );
        }
        (None, None) => (),
        _ => panic!("material interval disagreement"),
    }
    let reverse = body.intersect_segment(b, a, policy(scale)).unwrap();
    assert_eq!(reverse.hits.len(), result.hits.len());
    for (forward, backward) in result.hits.iter().rev().zip(&reverse.hits) {
        assert!((forward.parameter + backward.parameter - 1.).abs() < 1e-10);
        assert!((forward.point - backward.point).norm() < 1e-8 * scale);
    }
}

#[test]
fn independent_side_cap_and_material_intervals_under_pose_and_scale() {
    for scale in [1e-4, 1., 1e4] {
        for pose in [false, true] {
            let body = body([16., 8.], scale, pose);
            let w = |p| world(&body, p, scale);
            check(
                &body,
                w(radial(-30., 12.)),
                w(radial(30., 12.)),
                &[0.3, 0.7],
                Some([0.3, 0.7]),
                scale,
            );
            check(
                &body,
                w(Point3::new(0., 0., -5.)),
                w(Point3::new(0., 0., 29.)),
                &[5. / 34., 29. / 34.],
                Some([5. / 34., 29. / 34.]),
                scale,
            );
            check(
                &body,
                w(radial(20., 12.)),
                w(radial(0., 30.)),
                &[4. / 7., 2. / 3.],
                Some([4. / 7., 2. / 3.]),
                scale,
            );
            check(
                &body,
                w(radial(0., 12.)),
                w(radial(30., 12.)),
                &[0.4],
                Some([0., 0.4]),
                scale,
            );
            check(
                &body,
                w(Point3::new(1., 1., 10.)),
                w(Point3::new(2., 2., 14.)),
                &[],
                Some([0., 1.]),
                scale,
            );
            check(
                &body,
                w(Point3::new(-20., 30., 12.)),
                w(Point3::new(20., 30., 12.)),
                &[],
                None,
                scale,
            );
        }
    }
}

#[test]
fn cylinder_swapped_taper_and_actual_periodic_seam_provenance() {
    for radii in [[12., 12.], [8., 16.]] {
        let body = body(radii, 1., false);
        let radius = radii[0] + (radii[1] - radii[0]) * 0.25;
        check(
            &body,
            radial(-30., 6.),
            radial(30., 6.),
            &[(30. - radius) / 60., (30. + radius) / 60.],
            Some([(30. - radius) / 60., (30. + radius) / 60.]),
            1.,
        );
    }
    let body = body([16., 8.], 1., false);
    let result = body
        .intersect_segment(
            Point3::new(-30., 0., 12.),
            Point3::new(30., 0., 12.),
            policy(1.),
        )
        .unwrap();
    assert_eq!(result.hits.len(), 2);
    for hit in result.hits {
        assert_eq!(hit.faces.len(), 2);
        assert!(hit.faces.iter().all(|w| (2..6).contains(&w.face_id)));
    }
}

#[test]
fn ambiguity_nonfinite_and_unresolved_queries_are_explicit() {
    let body = body([16., 8.], 1., false);
    for (a, b) in [
        (Point3::new(-20., 12., 12.), Point3::new(20., 12., 12.)),
        (Point3::new(-30., 0., 0.), Point3::new(30., 0., 0.)),
        (radial(16., 0.), radial(8., 24.)),
        (Point3::new(0., 0., 12.), Point3::new(0., 0., 12.)),
        (Point3::new(-1e15, 0., 12.), Point3::new(1e15, 0., 12.)),
        (Point3::new(f64::NAN, 0., 0.), Point3::new(0., 0., 1.)),
    ] {
        assert!(body.intersect_segment(a, b, policy(1.)).is_err());
    }
    check(
        &body,
        radial(-30., 12.),
        radial(30., 12.),
        &[0.3, 0.7],
        Some([0.3, 0.7]),
        1.,
    );
}
