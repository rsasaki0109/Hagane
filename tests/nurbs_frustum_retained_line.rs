use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witnesses(solid: &Solid, o: Point3, d: Vec3, hits: &[NurbsFrustumLineHit], scale: f64) {
    for hit in hits {
        assert!((o + d * hit.parameter - hit.point).norm() < 1e-7 * scale);
        assert!(!hit.faces.is_empty());
        for f in &hit.faces {
            let actual = solid.shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((actual - hit.point).norm() < 1e-7 * scale);
        }
    }
}
fn bore() -> NurbsBoredFrustumSolid {
    NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy(1.))
        .unwrap()
        .bore_coaxial(4., policy(1.))
        .unwrap()
}
#[test]
fn bored_line_and_ray_keep_original_nonunit_parameters_and_material_intervals() {
    let body = bore();
    let before = format!("{:?}", body.solid());
    let step = body.export_step_mm(policy(1.)).unwrap();
    for (o, d, expected) in [
        (
            Point3::new(-20., 0., 12.),
            Vec3::new(2., 0., 0.),
            [4., 8., 12., 16.],
        ),
        (
            Point3::new(20., 0., 12.),
            Vec3::new(-2., 0., 0.),
            [4., 8., 12., 16.],
        ),
        (
            Point3::new(20., 0., 12.),
            Vec3::new(2., 0., 0.),
            [-16., -12., -8., -4.],
        ),
        (
            Point3::new(-20., 0., 12.),
            Vec3::new(4., 0., 0.),
            [2., 4., 6., 8.],
        ),
    ] {
        let r = body.intersect_line(o, d, policy(1.)).unwrap();
        assert_eq!(r.hits.len(), 4);
        assert_eq!(r.material_intervals.len(), 2);
        for (h, t) in r.hits.iter().zip(expected) {
            close(h.parameter, t);
        }
        for (i, interval) in r.material_intervals.iter().enumerate() {
            close(interval[0], expected[2 * i]);
            close(interval[1], expected[2 * i + 1]);
        }
        witnesses(body.solid(), o, d, &r.hits, 1.);
    }
    for (o, d, interval, hit_count) in [
        (
            Point3::new(0., 0., 12.),
            Vec3::new(2., 0., 0.),
            Some([2., 6.]),
            2,
        ),
        (
            Point3::new(6., 0., 12.),
            Vec3::new(2., 0., 0.),
            Some([0., 3.]),
            1,
        ),
        (Point3::new(20., 0., 12.), Vec3::new(2., 0., 0.), None, 0),
    ] {
        let r = body.intersect_ray(o, d, policy(1.)).unwrap();
        assert_eq!(r.hits.len(), hit_count);
        assert_eq!(r.material_intervals.len(), usize::from(interval.is_some()));
        if let Some(interval) = interval {
            close(r.material_intervals[0][0], interval[0]);
            close(r.material_intervals[0][1], interval[1]);
        }
        witnesses(body.solid(), o, d, &r.hits, 1.);
    }
    assert_eq!(before, format!("{:?}", body.solid()));
    assert_eq!(step, body.export_step_mm(policy(1.)).unwrap());
}
#[test]
fn oblique_retained_cap_and_side_witnesses_survive_pose_and_scale() {
    for scale in [1e-4, 1., 10.] {
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let frame = Frame3::new_with_tolerance(
            Point3::new(13., -7., 5.) * scale,
            rotation.axes(),
            policy(scale),
        )
        .unwrap();
        let source =
            NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
                .unwrap();
        let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
        let u = Vec3::new(-n.y, n.x, 0.).normalized().unwrap();
        let vector =
            |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
        let plane = Surface::Plane {
            origin: frame.point(Point3::new(0., 0., 12. * scale)),
            u: vector(u),
            v: vector(n.cross(u)),
        };
        let split = source.split_by_plane(&plane, policy(scale)).unwrap();
        for child in [&split.lower, &split.upper] {
            let before = format!("{:?}", child.solid());
            let step = child.export_step_mm(policy(scale)).unwrap();
            let o = frame.point(Point3::new(2., 3., -5.) * scale);
            let d = vector(Vec3::new(0., 0., 2. * scale));
            let r = child.intersect_line(o, d, policy(scale)).unwrap();
            assert_eq!(r.hits.len(), 2);
            assert_eq!(r.material_intervals.len(), 1);
            witnesses(child.solid(), o, d, &r.hits, scale);
            let expected = if child.is_lower() {
                [2.5, 8.325]
            } else {
                [8.325, 14.5]
            };
            close(r.material_intervals[0][0], expected[0]);
            close(r.material_intervals[0][1], expected[1]);
            let z = if child.is_lower() { 8. } else { 16. };
            let o = frame.point(Point3::new(0., 0., z) * scale);
            let d = vector(Vec3::new(0.37_f64.cos(), 0.37_f64.sin(), 0.) * scale);
            let lateral = child.intersect_line(o, d, policy(scale)).unwrap();
            assert_eq!(lateral.hits.len(), 2);
            witnesses(child.solid(), o, d, &lateral.hits, scale);
            for h in lateral.hits {
                for f in h.faces {
                    assert!((2..6).contains(&f.face_id));
                    assert!((f.uv[1] - z / 24.).abs() > 0.05);
                }
            }
            let ray = child.intersect_ray(o, d, policy(scale)).unwrap();
            close(ray.material_intervals[0][0], 0.);
            assert_eq!(ray.hits.len(), 1);
            witnesses(child.solid(), o, d, &ray.hits, scale);
            assert_eq!(before, format!("{:?}", child.solid()));
            assert_eq!(step, child.export_step_mm(policy(scale)).unwrap());
        }
    }
}
#[test]
fn original_query_precision_contacts_and_extreme_direction_mapping_are_checked() {
    let body = bore();
    for (o, d) in [
        (Point3::new(1e12, 1e12, 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(4., 0., 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(-20., 4., 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 12.), Vec3::new(0., 0., 0.)),
        (Point3::new(f64::NAN, 0., 12.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 12.), Vec3::new(f64::INFINITY, 0., 0.)),
    ] {
        assert!(body.intersect_ray(o, d, policy(1.)).is_err());
    }
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy(1.)).unwrap();
    let normal = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u,
        v: normal.cross(u),
    };
    let split = source.split_by_plane(&plane, policy(1.)).unwrap();
    let x = 12. / (1. - 0.1 / 3.);
    for (origin, direction) in [
        (Point3::new(x, 0., 12. - 0.1 * x), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 12. + 1e-9), Vec3::new(1., 0., -0.1)),
        (Point3::new(1e12, 1e12, 12.), Vec3::new(1., 0., 0.)),
    ] {
        assert!(split
            .upper
            .intersect_ray(origin, direction, policy(1.))
            .is_err());
    }
    for magnitude in [1e-250, 1e250] {
        let o = Point3::new(-20., 0., 12.);
        let d = Vec3::new(magnitude, 0., 0.);
        match body.intersect_line(o, d, policy(1.)) {
            Ok(r) => {
                assert_eq!(r.hits.len(), 4);
                witnesses(body.solid(), o, d, &r.hits, 1.);
                for (h, x) in r.hits.iter().zip([-12., -4., 4., 12.]) {
                    close(h.parameter * magnitude, x + 20.);
                }
            }
            Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
    let r = body
        .intersect_ray(Point3::new(0., 0., 12.), Vec3::new(2., 0., 0.), policy(1.))
        .unwrap();
    assert_eq!(r.material_intervals.len(), 1);
}
