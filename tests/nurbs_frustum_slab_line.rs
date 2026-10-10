use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn plane(f: Frame3, s: f64, d: f64) -> Surface {
    let u = f.axes()[0];
    let v = f.axes()[1];
    Surface::Plane {
        origin: f.point(Point3::new(0., 0., d * s)),
        u,
        v,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witness(solid: &Solid, o: Point3, d: Vec3, hits: &[NurbsFrustumLineHit], scale: f64) {
    for hit in hits {
        assert!((o + d * hit.parameter - hit.point).norm() < 1e-7 * scale);
        for w in &hit.faces {
            let face = &solid.shell.faces[w.face_id];
            let actual = face.surface.try_evaluate(w.uv[0], w.uv[1]).unwrap();
            assert!((actual - hit.point).norm() < 1e-7 * scale);
            assert!(
                face.surface
                    .normal_at(w.uv[0], w.uv[1])
                    .unwrap()
                    .dot(d)
                    .abs()
                    > 0.
            );
        }
    }
}
#[test]
fn actual_caps_walls_and_all_annular_ray_intervals_preserve_original_parameters() {
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let rot = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let f = Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), p).unwrap();
        let source = NurbsFrustumSolid::new(f, [12. * s, 12. * s], 24. * s, p).unwrap();
        let lower = plane(f, s, 8.);
        let upper = plane(f, s, 12.);
        let first = source.split_by_plane(&lower, p).unwrap();
        let slab = first
            .upper
            .split_by_parallel_plane(&upper, p)
            .unwrap()
            .lower;
        let bored = source.bore_coaxial(4. * s, p).unwrap();
        let first = bored.split_by_plane(&lower, p).unwrap();
        let bore = first
            .upper
            .split_by_parallel_plane(&upper, p)
            .unwrap()
            .lower;
        let world = |v: Point3| f.point(v * s);
        let vector =
            |v: Vec3| f.axes()[0] * (v.x * s) + f.axes()[1] * (v.y * s) + f.axes()[2] * (v.z * s);
        let before = format!("{:?}", bore.solid());
        let step = bore.export_step_mm(p).unwrap();
        for magnitude in [0.5, 2., 10.] {
            let o = world(Point3::new(5., 0., -5.));
            let d = vector(Vec3::new(2., 0., 35.) * magnitude);
            let r = slab.intersect_line(o, d, p).unwrap();
            assert_eq!(r.hits.len(), 2);
            assert_eq!(r.material_intervals.len(), 1);
            close(r.material_intervals[0][0], 13. / 35. / magnitude);
            close(r.material_intervals[0][1], 17. / 35. / magnitude);
            witness(slab.solid(), o, d, &r.hits, s);
            let r = bore.intersect_line(o, d, p).unwrap();
            assert_eq!(r.hits.len(), 2);
            close(r.material_intervals[0][0], 13. / 35. / magnitude);
            close(r.material_intervals[0][1], 17. / 35. / magnitude);
            witness(bore.solid(), o, d, &r.hits, s);
        }
        for (o, d, expected) in [
            (
                Point3::new(-20., 0., 10.),
                Vec3::new(2., 0., 0.),
                [[4., 8.], [12., 16.]],
            ),
            (
                Point3::new(20., 0., 10.),
                Vec3::new(2., 0., 0.),
                [[-16., -12.], [-8., -4.]],
            ),
            (
                Point3::new(20., 0., 10.),
                Vec3::new(-2., 0., 0.),
                [[4., 8.], [12., 16.]],
            ),
        ] {
            let o = world(o);
            let d = vector(d);
            let r = bore.intersect_line(o, d, p).unwrap();
            assert_eq!(r.hits.len(), 4);
            assert_eq!(r.material_intervals.len(), 2);
            for (a, b) in r.material_intervals.iter().zip(expected) {
                close(a[0], b[0]);
                close(a[1], b[1]);
            }
            witness(bore.solid(), o, d, &r.hits, s);
        }
        for (o, expected, hits) in [
            (Point3::new(0., 0., 10.), vec![[2., 6.]], 2),
            (Point3::new(6., 0., 10.), vec![[0., 3.]], 1),
            (Point3::new(-6., 0., 10.), vec![[0., 1.], [5., 9.]], 3),
            (Point3::new(20., 0., 10.), vec![], 0),
        ] {
            let o = world(o);
            let d = vector(Vec3::new(2., 0., 0.));
            let r = bore.intersect_ray(o, d, p).unwrap();
            assert_eq!(r.hits.len(), hits);
            assert_eq!(r.material_intervals.len(), expected.len());
            for (a, b) in r.material_intervals.iter().zip(expected) {
                close(a[0], b[0]);
                close(a[1], b[1]);
            }
            witness(bore.solid(), o, d, &r.hits, s);
        }
        assert_eq!(before, format!("{:?}", bore.solid()));
        assert_eq!(step, bore.export_step_mm(p).unwrap());
    }
}
#[test]
fn original_anchor_contact_and_nonfinite_guards_fail_and_recover() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let first = source.split_by_plane(&plane(f, 1., 8.), p).unwrap();
    let body = first
        .upper
        .split_by_parallel_plane(&plane(f, 1., 12.), p)
        .unwrap()
        .lower;
    let before = format!("{:?}", body.solid());
    for (o, d) in [
        (Point3::new(4., 0., 10.), Vec3::new(1., 0., 0.)),
        (Point3::new(-20., 4., 10.), Vec3::new(1., 0., 0.)),
        (Point3::new(6., 0., 8.), Vec3::new(0., 0., 1.)),
        (Point3::new(1e12, 1e12, 10.), Vec3::new(1., 0., 0.)),
        (Point3::new(f64::NAN, 0., 10.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 10.), Vec3::new(f64::INFINITY, 0., 0.)),
        (Point3::new(0., 0., 10.), Vec3::new(0., 0., 0.)),
    ] {
        assert!(body.intersect_ray(o, d, p).is_err());
    }
    let r = body
        .intersect_ray(Point3::new(0., 0., 10.), Vec3::new(2., 0., 0.), p)
        .unwrap();
    assert_eq!(r.material_intervals.len(), 1);
    assert_eq!(before, format!("{:?}", body.solid()));
}
