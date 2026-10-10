use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn body(frame: Frame3, scale: f64) -> NurbsBoredFrustumSolid {
    NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
        .unwrap()
        .bore_coaxial(4. * scale, policy(scale))
        .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[test]
fn diameter_returns_two_material_intervals_and_actual_outer_inner_face_witnesses() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
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
            let shape = body(frame, scale);
            let before = format!("{:?}", shape.solid());
            let step = shape.export_step_mm(policy(scale)).unwrap();
            let volume = shape.volume(policy(scale)).unwrap();
            let world = |x| {
                frame.origin() + frame.axes()[0] * (x * scale) + frame.axes()[2] * (12. * scale)
            };
            for (a, b) in [(world(-20.), world(20.)), (world(20.), world(-20.))] {
                let result = shape.intersect_segment(a, b, policy(scale)).unwrap();
                assert_eq!(result.hits.len(), 4);
                assert_eq!(result.material_intervals.len(), 2);
                for (interval, expected) in result
                    .material_intervals
                    .iter()
                    .zip([[0.2, 0.4], [0.6, 0.8]])
                {
                    close(interval[0], expected[0]);
                    close(interval[1], expected[1]);
                }
                for (i, hit) in result.hits.iter().enumerate() {
                    close(hit.parameter, [0.2, 0.4, 0.6, 0.8][i]);
                    assert!((hit.point - (a + (b - a) * hit.parameter)).norm() < 1e-7 * scale);
                    assert!(!hit.faces.is_empty());
                    for witness in &hit.faces {
                        assert!(if i == 0 || i == 3 {
                            (2..6).contains(&witness.face_id)
                        } else {
                            (6..10).contains(&witness.face_id)
                        });
                        let actual = shape.solid().shell.faces[witness.face_id]
                            .surface
                            .try_evaluate(witness.uv[0], witness.uv[1])
                            .unwrap();
                        assert!((actual - hit.point).norm() < 1e-7 * scale);
                    }
                }
            }
            assert_eq!(before, format!("{:?}", shape.solid()));
            assert_eq!(step, shape.export_step_mm(policy(scale)).unwrap());
            assert_eq!(volume, shape.volume(policy(scale)).unwrap());
        }
    }
}
#[test]
fn strict_material_endpoints_and_empty_hole_axis_do_not_fill_the_void() {
    let shape = body(Frame3::IDENTITY, 1.);
    for (a, b, expected) in [
        (
            Point3::new(-8., 0., 12.),
            Point3::new(8., 0., 12.),
            vec![[0., 0.25], [0.75, 1.]],
        ),
        (
            Point3::new(5., 0., 12.),
            Point3::new(8., 0., 12.),
            vec![[0., 1.]],
        ),
        (Point3::new(0., 0., -5.), Point3::new(0., 0., 30.), vec![]),
    ] {
        let result = shape.intersect_segment(a, b, policy(1.)).unwrap();
        assert_eq!(result.material_intervals.len(), expected.len());
        for (actual, expected) in result.material_intervals.iter().zip(expected) {
            close(actual[0], expected[0]);
            close(actual[1], expected[1]);
        }
        if result.material_intervals.is_empty() {
            assert!(result.hits.is_empty());
        }
    }
}
#[test]
fn contacts_precision_and_nonfinite_queries_reject_with_recovery() {
    let shape = body(Frame3::IDENTITY, 1.);
    let before = format!("{:?}", shape.solid());
    for (a, b) in [
        (Point3::new(-20., 4., 12.), Point3::new(20., 4., 12.)),
        (Point3::new(4., 0., 12.), Point3::new(20., 0., 12.)),
        (Point3::new(-20., 0., 0.), Point3::new(20., 0., 0.)),
        (Point3::new(-1e12, 0., 12.), Point3::new(1e12, 0., 12.)),
        (Point3::new(5., 0., 12.), Point3::new(5. + 1e-8, 0., 12.)),
        (Point3::new(f64::NAN, 0., 12.), Point3::new(20., 0., 12.)),
        (
            Point3::new(-20., 0., 12.),
            Point3::new(f64::INFINITY, 0., 12.),
        ),
    ] {
        assert!(
            shape.intersect_segment(a, b, policy(1.)).is_err(),
            "{a:?} {b:?}"
        );
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(shape
        .intersect_segment(
            Point3::new(-20., 4. + 0.01, 12.),
            Point3::new(20., 4. + 0.01, 12.),
            relative
        )
        .is_err());
    assert_eq!(
        shape
            .intersect_segment(
                Point3::new(-20., 0., 12.),
                Point3::new(20., 0., 12.),
                policy(1.)
            )
            .unwrap()
            .material_intervals
            .len(),
        2
    );
    assert_eq!(before, format!("{:?}", shape.solid()));
}
