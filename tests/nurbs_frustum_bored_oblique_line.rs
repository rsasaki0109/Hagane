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
    o: Point3,
    d: Vec3,
    r: &NurbsFrustumMaterialLineIntersection,
    s: f64,
) {
    for hit in &r.hits {
        assert!((hit.point - (o + d * hit.parameter)).norm() < 1e-7 * s);
        for w in &hit.faces {
            let p = child.solid().shell.faces[w.face_id]
                .surface
                .try_evaluate(w.uv[0], w.uv[1])
                .unwrap();
            assert!((p - hit.point).norm() < 1e-7 * s);
        }
    }
}
#[test]
fn actual_nonunit_lines_rays_cut_caps_and_void_roots_survive_pose_scale() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let (split, f) = fixture(s, posed);
            let world = |p: Point3| f.point(p * s);
            let vector = |d: Vec3| {
                f.axes()[0] * (d.x * s) + f.axes()[1] * (d.y * s) + f.axes()[2] * (d.z * s)
            };
            for child in [&split.lower, &split.upper] {
                let before = format!("{:?}", child.solid());
                let step = child.export_step_mm(policy(s)).unwrap();
                let z = if child.is_lower() { 6. } else { 18. };
                for (origin, direction, expected) in [
                    (
                        Point3::new(-20., 0., z),
                        Vec3::new(2., 0., 0.),
                        [[4., 8.], [12., 16.]],
                    ),
                    (
                        Point3::new(20., 0., z),
                        Vec3::new(-2., 0., 0.),
                        [[4., 8.], [12., 16.]],
                    ),
                    (
                        Point3::new(20., 0., z),
                        Vec3::new(2., 0., 0.),
                        [[-16., -12.], [-8., -4.]],
                    ),
                ] {
                    let o = world(origin);
                    let d = vector(direction);
                    let r = child.intersect_line(o, d, policy(s)).unwrap();
                    assert_eq!(r.material_intervals.len(), 2);
                    assert_eq!(r.hits.len(), 4);
                    for (a, b) in r.material_intervals.iter().zip(expected) {
                        close(a[0], b[0]);
                        close(a[1], b[1]);
                    }
                    witnesses(child, o, d, &r, s);
                }
                for (origin, expected, hits) in [
                    (Point3::new(0., 0., z), Some([2., 6.]), 2),
                    (Point3::new(6., 0., z), Some([0., 3.]), 1),
                    (Point3::new(20., 0., z), None, 0),
                ] {
                    let o = world(origin);
                    let d = vector(Vec3::new(2., 0., 0.));
                    let r = child.intersect_ray(o, d, policy(s)).unwrap();
                    assert_eq!(r.hits.len(), hits);
                    assert_eq!(r.material_intervals.len(), usize::from(expected.is_some()));
                    if let Some(e) = expected {
                        close(r.material_intervals[0][0], e[0]);
                        close(r.material_intervals[0][1], e[1]);
                    }
                    witnesses(child, o, d, &r, s);
                }
                let o = world(Point3::new(-20., 0., 12.));
                let d = vector(Vec3::new(2., 0., 0.));
                let r = child.intersect_line(o, d, policy(s)).unwrap();
                assert_eq!(r.material_intervals.len(), 1);
                let expected = if child.is_lower() {
                    [4., 8.]
                } else {
                    [12., 16.]
                };
                close(r.material_intervals[0][0], expected[0]);
                close(r.material_intervals[0][1], expected[1]);
                assert_eq!(r.hits.len(), 2);
                assert!(r.hits.iter().flat_map(|h| &h.faces).all(|w| w.face_id >= 2));
                witnesses(child, o, d, &r, s);
                // The slanted query's finite cut root is in annular material.
                let o = world(Point3::new(5., 0., -5.));
                let d = vector(Vec3::new(2., 0., 35.));
                let r = child.intersect_line(o, d, policy(s)).unwrap();
                assert_eq!(r.material_intervals.len(), 1);
                let expected = if child.is_lower() {
                    [5. / 35., 16. / 35.4]
                } else {
                    [16. / 35.4, 29. / 35.]
                };
                close(r.material_intervals[0][0], expected[0]);
                close(r.material_intervals[0][1], expected[1]);
                assert!(r.hits.iter().flat_map(|h| &h.faces).any(|w| w.face_id < 2));
                witnesses(child, o, d, &r, s);
                assert_eq!(before, format!("{:?}", child.solid()));
                assert_eq!(step, child.export_step_mm(policy(s)).unwrap());
            }
        }
    }
}
#[test]
fn contact_and_original_anchor_precision_failures_keep_geometry_and_recover() {
    let (split, f) = fixture(1., false);
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let z = if child.is_lower() { 6. } else { 18. };
        for (o, d) in [
            (Point3::new(4., 0., z), Vec3::new(1., 0., 0.)),
            (Point3::new(-20., 4., z), Vec3::new(1., 0., 0.)),
            (Point3::new(1e12, 1e12, z), Vec3::new(1., 0., 0.)),
            (Point3::new(0., 0., z), Vec3::new(0., 0., 0.)),
            (Point3::new(f64::NAN, 0., z), Vec3::new(1., 0., 0.)),
            (Point3::new(0., 0., z), Vec3::new(f64::INFINITY, 0., 0.)),
        ] {
            assert!(child.intersect_ray(f.point(o), d, policy(1.)).is_err());
        }
        assert_eq!(before, format!("{:?}", child.solid()));
        let r = child
            .intersect_ray(
                f.point(Point3::new(0., 0., z)),
                Vec3::new(2., 0., 0.),
                policy(1.),
            )
            .unwrap();
        assert_eq!(r.material_intervals.len(), 1);
    }
}
