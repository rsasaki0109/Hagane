use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn body(posed: bool) -> NurbsFrustumSolid {
    let frame = if posed {
        Transform::translation(Vec3::new(12., -3., 5.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap()
    } else {
        Frame3::IDENTITY
    };
    NurbsFrustumSolid::new(frame, [16., 8.], 24., policy()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9 * b.abs().max(1e-300), "{a} vs {b}");
}
fn witnesses(
    body: &NurbsFrustumSolid,
    origin: Point3,
    direction: Vec3,
    result: &NurbsFrustumLineIntersection,
    expected: &[f64],
    interval: Option<[f64; 2]>,
) {
    assert_eq!(result.hits.len(), expected.len());
    assert!(result
        .hits
        .windows(2)
        .all(|pair| pair[0].parameter < pair[1].parameter));
    for (hit, parameter) in result.hits.iter().zip(expected) {
        close(hit.parameter, *parameter);
        assert!((hit.point - (origin + direction * hit.parameter)).norm() < 1e-8);
        assert!(!hit.faces.is_empty());
        for witness in &hit.faces {
            assert!(
                (body.solid().shell.faces[witness.face_id]
                    .surface
                    .try_evaluate(witness.uv[0], witness.uv[1])
                    .unwrap()
                    - hit.point)
                    .norm()
                    < 1e-8
            );
        }
    }
    match (result.material_interval, interval) {
        (Some(a), Some(b)) => {
            close(a[0], b[0]);
            close(a[1], b[1]);
            assert_eq!(
                body.classify_point(origin + direction * ((a[0] + a[1]) * 0.5), policy())
                    .unwrap(),
                PointLocation::Inside
            );
        }
        (None, None) => (),
        _ => panic!("incorrect material interval"),
    }
}

#[test]
fn infinite_axis_lateral_reversed_and_nonunit_queries() {
    for posed in [false, true] {
        let body = body(posed);
        let frame = body.frame();
        let origin = frame.point(Point3::new(0., 0., -5.));
        let direction = frame.vector(Vec3::new(0., 0., 2.));
        witnesses(
            &body,
            origin,
            direction,
            &body.intersect_line(origin, direction, policy()).unwrap(),
            &[2.5, 14.5],
            Some([2.5, 14.5]),
        );
        for magnitude in [4., -4., 1e-200, 1e200] {
            let origin = frame.point(Point3::new(0., 0., 12.));
            let direction = frame.vector(Vec3::new(
                0.37_f64.cos() * magnitude,
                0.37_f64.sin() * magnitude,
                0.,
            ));
            let t = 12. / magnitude.abs();
            witnesses(
                &body,
                origin,
                direction,
                &body.intersect_line(origin, direction, policy()).unwrap(),
                &[-t, t],
                Some([-t, t]),
            );
        }
        for epsilon in [-1e-16, 0., 1e-16] {
            let origin = frame.point(Point3::new(0., 0., 12.));
            let direction = frame.vector(Vec3::new(1., epsilon, epsilon));
            witnesses(
                &body,
                origin,
                direction,
                &body.intersect_line(origin, direction, policy()).unwrap(),
                &[-12., 12.],
                Some([-12., 12.]),
            );
            let origin = frame.point(Point3::new(17., 0., 12.));
            let direction = frame.vector(Vec3::new(epsilon, 1., 0.));
            witnesses(
                &body,
                origin,
                direction,
                &body.intersect_line(origin, direction, policy()).unwrap(),
                &[],
                None,
            );
        }
        let origin = frame.point(Point3::new(-20., 30., 12.));
        let direction = frame.vector(Vec3::new(1., 0., 0.));
        witnesses(
            &body,
            origin,
            direction,
            &body.intersect_line(origin, direction, policy()).unwrap(),
            &[],
            None,
        );
    }
}

#[test]
fn ray_origin_inside_outside_away_and_reversed_axis() {
    for posed in [false, true] {
        let body = body(posed);
        let frame = body.frame();
        for (z, d, parameters, interval) in [
            (-5., 2., vec![2.5, 14.5], Some([2.5, 14.5])),
            (12., -2., vec![6.], Some([0., 6.])),
            (29., 2., vec![], None),
            (29., -2., vec![2.5, 14.5], Some([2.5, 14.5])),
        ] {
            let origin = frame.point(Point3::new(0., 0., z));
            let direction = frame.vector(Vec3::new(0., 0., d));
            witnesses(
                &body,
                origin,
                direction,
                &body.intersect_ray(origin, direction, policy()).unwrap(),
                &parameters,
                interval,
            );
        }
        let origin = frame.point(Point3::new(0., 0., 29.));
        let direction = frame.vector(Vec3::new(0., 0., 1e200));
        witnesses(
            &body,
            origin,
            direction,
            &body.intersect_ray(origin, direction, policy()).unwrap(),
            &[],
            None,
        );
        let origin = frame.point(Point3::new(0., 0., 12.));
        let direction = frame.vector(Vec3::new(4. * 0.37_f64.cos(), 4. * 0.37_f64.sin(), 0.));
        witnesses(
            &body,
            origin,
            direction,
            &body.intersect_ray(origin, direction, policy()).unwrap(),
            &[3.],
            Some([0., 3.]),
        );
    }
}

#[test]
fn contacts_original_far_anchors_and_unrepresentable_directions_reject() {
    let body = body(false);
    for (origin, direction) in [
        (Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 12., 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(1e15, 0., 12.), Vec3::new(-1., 0., 0.)),
        (Point3::new(0., 0., 12.), Vec3::new(0., 0., 0.)),
        (Point3::new(f64::NAN, 0., 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 12.), Vec3::new(f64::INFINITY, 0., 0.)),
        (Point3::new(0., 0., 12.), Vec3::new(1e-310, 0., 0.)),
    ] {
        assert!(body.intersect_line(origin, direction, policy()).is_err());
    }
    assert!(body
        .intersect_ray(Point3::new(12., 0., 12.), Vec3::new(1., 0., 0.), policy())
        .is_err());
    let result = body
        .intersect_line(Point3::new(0., 0., -5.), Vec3::new(0., 0., 2.), policy())
        .unwrap();
    witnesses(
        &body,
        Point3::new(0., 0., -5.),
        Vec3::new(0., 0., 2.),
        &result,
        &[2.5, 14.5],
        Some([2.5, 14.5]),
    );
}
