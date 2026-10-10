use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn plane(f: Frame3, s: f64, d: f64, tilt: bool) -> Surface {
    let n = Vec3::new(
        if tilt { 0.1 } else { 0. },
        if tilt { 0.05 } else { 0. },
        1.,
    )
    .normalized()
    .unwrap();
    let u = (Vec3::new(0., 1., 0.) - n * n.y).normalized().unwrap();
    let vector = |p: Vec3| f.axes()[0] * p.x + f.axes()[1] * p.y + f.axes()[2] * p.z;
    Surface::Plane {
        origin: f.point(Point3::new(0., 0., d * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn next(p: &Surface, f: Frame3, d: f64) -> Surface {
    let Surface::Plane { origin, u, v } = p else {
        panic!("plane")
    };
    Surface::Plane {
        origin: *origin + f.axes()[2] * d,
        u: *u,
        v: *v,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn witness(
    solid: &Solid,
    a: Point3,
    b: Point3,
    hits: &[NurbsFrustumSegmentHit],
    intervals: &[[f64; 2]],
    scale: f64,
) {
    for hit in hits {
        assert!((hit.point - (a + (b - a) * hit.parameter)).norm() < 1e-7 * scale);
        for w in &hit.faces {
            let face = &solid.shell.faces[w.face_id];
            let point = face.surface.try_evaluate(w.uv[0], w.uv[1]).unwrap();
            assert!((point - hit.point).norm() < 1e-7 * scale);
            let n = face.surface.normal_at(w.uv[0], w.uv[1]).unwrap() * face.orientation as f64;
            for interval in intervals {
                if (hit.parameter - interval[0]).abs() < 1e-9 {
                    assert!(n.dot(b - a) < 0.);
                }
                if (hit.parameter - interval[1]).abs() < 1e-9 {
                    assert!(n.dot(b - a) > 0.);
                }
            }
        }
    }
}
#[test]
fn both_caps_clip_only_final_material_and_actual_walls_keep_parameter_witnesses() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let p = policy(scale);
            let r =
                Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
            let f =
                Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * scale, r.axes(), p).unwrap();
            let source =
                NurbsFrustumSolid::new(f, [12. * scale, 12. * scale], 24. * scale, p).unwrap();
            let lower = plane(f, scale, 8., true);
            let first = source.split_by_plane(&lower, p).unwrap();
            let upper = next(&lower, f, 4. * scale);
            let slab = first
                .upper
                .split_by_parallel_plane(&upper, p)
                .unwrap()
                .lower;
            let bore = source.bore_coaxial(4. * scale, p).unwrap();
            let first = bore.split_by_plane(&lower, p).unwrap();
            let bored = first
                .upper
                .split_by_parallel_plane(&upper, p)
                .unwrap()
                .lower;
            let world = |v: Point3| f.point(v * scale);
            let a = world(Point3::new(5., 0., -5.));
            let b = world(Point3::new(7., 0., 30.));
            let lo = 12.5 / 35.2;
            let hi = 16.5 / 35.2;
            let actual = slab.intersect_segment(a, b, p).unwrap();
            assert_eq!(actual.hits.len(), 2);
            close(actual.material_interval.unwrap()[0], lo);
            close(actual.material_interval.unwrap()[1], hi);
            assert!(actual
                .hits
                .iter()
                .flat_map(|h| &h.faces)
                .all(|w| w.face_id < 2));
            witness(
                slab.solid(),
                a,
                b,
                &actual.hits,
                &[actual.material_interval.unwrap()],
                scale,
            );
            let actual = bored.intersect_segment(a, b, p).unwrap();
            assert_eq!(actual.hits.len(), 2);
            assert_eq!(actual.material_intervals.len(), 1);
            close(actual.material_intervals[0][0], lo);
            close(actual.material_intervals[0][1], hi);
            assert!(actual
                .hits
                .iter()
                .flat_map(|h| &h.faces)
                .all(|w| w.face_id < 2));
            witness(
                bored.solid(),
                a,
                b,
                &actual.hits,
                &actual.material_intervals,
                scale,
            );
            for reverse in [false, true] {
                let mut a = world(Point3::new(-20., 0., 10.));
                let mut b = world(Point3::new(20., 0., 10.));
                if reverse {
                    std::mem::swap(&mut a, &mut b);
                }
                let result = bored.intersect_segment(a, b, p).unwrap();
                assert_eq!(result.material_intervals.len(), 2); // Oblique caps trim the otherwise four-wall traversal.
                witness(
                    bored.solid(),
                    a,
                    b,
                    &result.hits,
                    &result.material_intervals,
                    scale,
                );
            }
            let before = format!("{:?}", bored.solid());
            let step = bored.export_step_mm(p).unwrap();
            let result = bored
                .intersect_segment(
                    world(Point3::new(5., 0., 9.)),
                    world(Point3::new(6., 0., 9.)),
                    p,
                )
                .unwrap();
            assert_eq!(result.material_intervals, vec![[0., 1.]]);
            assert!(result.hits.is_empty());
            assert_eq!(before, format!("{:?}", bored.solid()));
            assert_eq!(step, bored.export_step_mm(p).unwrap());
        }
    }
}
#[test]
fn horizontal_annular_slab_has_four_walls_and_no_void_cap_events() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let lower = plane(f, 1., 8., false);
    let first = source.split_by_plane(&lower, p).unwrap();
    let slab = first
        .upper
        .split_by_parallel_plane(&next(&lower, f, 4.), p)
        .unwrap()
        .lower;
    let a = Point3::new(-20., 0., 10.);
    let b = Point3::new(20., 0., 10.);
    let r = slab.intersect_segment(a, b, p).unwrap();
    assert_eq!(r.hits.len(), 4);
    assert_eq!(r.material_intervals.len(), 2);
    for (actual, expected) in r.material_intervals.iter().zip([[0.2, 0.4], [0.6, 0.8]]) {
        close(actual[0], expected[0]);
        close(actual[1], expected[1]);
    }
    witness(slab.solid(), a, b, &r.hits, &r.material_intervals, 1.);
    let r = slab
        .intersect_segment(Point3::new(0., 0., -5.), Point3::new(0., 0., 30.), p)
        .unwrap();
    assert!(r.hits.is_empty());
    assert!(r.material_intervals.is_empty());
}
#[test]
fn contact_precision_and_removed_earlier_boundaries_cannot_create_phantom_hits() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let lower = plane(f, 1., 8., false);
    let first = source.split_by_plane(&lower, p).unwrap();
    let slab = first
        .upper
        .split_by_parallel_plane(&next(&lower, f, 4.), p)
        .unwrap()
        .lower;
    let before = format!("{:?}", slab.solid());
    for (a, b) in [
        (Point3::new(-20., 4., 10.), Point3::new(20., 4., 10.)),
        (Point3::new(4., 0., 10.), Point3::new(20., 0., 10.)),
        (Point3::new(-20., 0., 8.), Point3::new(20., 0., 8.)),
        (
            Point3::new(6., 0., 8. - 1e-10),
            Point3::new(6., 0., 8. + 1e-10),
        ),
        (Point3::new(-1e12, 0., 10.), Point3::new(1e12, 0., 10.)),
        (Point3::new(f64::NAN, 0., 10.), Point3::new(6., 0., 10.)),
        (
            Point3::new(6., 0., 10.),
            Point3::new(f64::INFINITY, 0., 10.),
        ),
    ] {
        assert!(slab.intersect_segment(a, b, p).is_err());
    }
    let removed = slab
        .intersect_segment(Point3::new(5., 0., 1.), Point3::new(6., 0., 1.), p)
        .unwrap();
    assert!(removed.hits.is_empty());
    assert!(removed.material_intervals.is_empty());
    assert_eq!(before, format!("{:?}", slab.solid()));
}
