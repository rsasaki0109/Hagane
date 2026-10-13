use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-9, "{a} != {b}");
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let f =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn witnesses(
    shape: &NurbsTorusHorizontalSlab,
    a: Point3,
    b: Point3,
    r: &NurbsTorusSegmentIntersection,
    s: f64,
) {
    let mut previous = -1.;
    for hit in &r.hits {
        assert!(hit.parameter > previous && hit.parameter > 0. && hit.parameter < 1.);
        previous = hit.parameter;
        assert!((hit.point - (a + (b - a) * hit.parameter)).norm() < 1e-7 * s);
        let w = &hit.boundary;
        let actual = shape.solid().shell.faces[w.face]
            .surface
            .try_evaluate(w.uv[0], w.uv[1])
            .unwrap();
        assert!((actual - hit.point).norm() < 1e-7 * s);
        assert!((w.point - hit.point).norm() < 1e-7 * s);
        for interval in &r.material_intervals {
            if (hit.parameter - interval[0]).abs() < 1e-9 {
                assert!(w.normal.dot(b - a) < 0.);
            }
            if (hit.parameter - interval[1]).abs() < 1e-9 {
                assert!(w.normal.dot(b - a) > 0.);
            }
        }
    }
    for interval in &r.material_intervals {
        assert!(interval[0] >= 0. && interval[1] <= 1. && interval[0] < interval[1]);
        assert_eq!(
            shape
                .classify_point(a + (b - a) * ((interval[0] + interval[1]) / 2.), policy(s))
                .unwrap(),
            PointLocation::Inside
        );
    }
    for pair in r.material_intervals.windows(2) {
        assert!(pair[0][1] < pair[1][0]);
        assert_eq!(
            shape
                .classify_point(a + (b - a) * ((pair[0][1] + pair[1][0]) / 2.), policy(s))
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn independent_circle_roots_two_cap_roots_and_sloped_cap_wall_hits_hold_under_pose_scale() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let p = policy(s);
            let f = source.frame();
            let world = |x: Point3| f.point(x * s);
            let original = format!("{:?}", source.solid());
            let original_step = source.export_step_mm(p).unwrap();
            for heights in [[-1.25, 1.25], [0.1, 0.3], [-0.3, -0.1]] {
                let slab = source
                    .slab_between_local_z(heights.map(|x| x * s), p)
                    .unwrap();
                let before = format!("{:?}", slab.solid());
                let step = slab.export_step_mm(p).unwrap();
                let z = (heights[0] + heights[1]) / 2.;
                for y in [0., 5.] {
                    let radius = (16_f64 - z * z).sqrt();
                    let rho = [16. - radius, 16. + radius];
                    let roots = [
                        -(rho[1] * rho[1] - y * y).sqrt(),
                        -(rho[0] * rho[0] - y * y).sqrt(),
                        (rho[0] * rho[0] - y * y).sqrt(),
                        (rho[1] * rho[1] - y * y).sqrt(),
                    ];
                    for reversed in [false, true] {
                        let (a, b) = if reversed {
                            (Point3::new(24., y, z), Point3::new(-24., y, z))
                        } else {
                            (Point3::new(-24., y, z), Point3::new(24., y, z))
                        };
                        let mut expected: Vec<_> =
                            roots.iter().map(|x| (x - a.x) / (b.x - a.x)).collect();
                        expected.sort_by(f64::total_cmp);
                        let r = slab.intersect_segment(world(a), world(b), p).unwrap();
                        assert_eq!(r.hits.len(), 4);
                        assert_eq!(r.material_intervals.len(), 2);
                        for (h, t) in r.hits.iter().zip(&expected) {
                            close(h.parameter, *t);
                        }
                        for (i, interval) in r.material_intervals.iter().enumerate() {
                            close(interval[0], expected[2 * i]);
                            close(interval[1], expected[2 * i + 1]);
                        }
                        witnesses(&slab, world(a), world(b), &r, s);
                    }
                }
                for reverse in [false, true] {
                    let (a, b) = if reverse {
                        (Point3::new(16., 0., 10.), Point3::new(16., 0., -10.))
                    } else {
                        (Point3::new(16., 0., -10.), Point3::new(16., 0., 10.))
                    };
                    let mut expected = heights.map(|h| (h - a.z) / (b.z - a.z));
                    expected.sort_by(f64::total_cmp);
                    let r = slab.intersect_segment(world(a), world(b), p).unwrap();
                    assert_eq!(r.hits.len(), 2);
                    assert_eq!(r.material_intervals.len(), 1);
                    for (h, t) in r.hits.iter().zip(expected) {
                        close(h.parameter, t);
                        assert!(matches!(
                            slab.solid().shell.faces[h.boundary.face].surface,
                            Surface::Plane { .. }
                        ));
                    }
                    witnesses(&slab, world(a), world(b), &r, s);
                }
                let a = world(Point3::new(15.5, 0., z));
                let b = world(Point3::new(16.5, 0., z));
                let r = slab.intersect_segment(a, b, p).unwrap();
                assert!(r.hits.is_empty());
                assert_eq!(r.material_intervals, vec![[0., 1.]]);
                witnesses(&slab, a, b, &r, s);
                assert_eq!(before, format!("{:?}", slab.solid()));
                assert_eq!(step, slab.export_step_mm(p).unwrap());
            }
            // Analytic sloped path rho=16+4t,z=2t. Outer tube equation is20t²=16.
            let slab = source.slab_between_local_z([0.5 * s, 2. * s], p).unwrap();
            let a = world(Point3::new(16., 0., 0.));
            let b = world(Point3::new(20., 0., 2.));
            let r = slab.intersect_segment(a, b, p).unwrap();
            assert_eq!(r.hits.len(), 2);
            close(r.hits[0].parameter, 0.25);
            assert!((r.hits[1].parameter - (0.8_f64).sqrt()).abs() * 20_f64.sqrt() < 1e-6 / 8.);
            assert!(matches!(
                slab.solid().shell.faces[r.hits[0].boundary.face].surface,
                Surface::Plane { .. }
            ));
            assert!(matches!(
                slab.solid().shell.faces[r.hits[1].boundary.face].surface,
                Surface::Nurbs(_)
            ));
            witnesses(&slab, a, b, &r, s);
            assert_eq!(original, format!("{:?}", source.solid()));
            assert_eq!(original_step, source.export_step_mm(p).unwrap());
        }
    }
}
#[test]
fn finite_ring_void_cap_extensions_and_removed_material_miss_without_phantom_cap_hits() {
    let source = body(1., true);
    let p = policy(1.);
    let f = source.frame();
    let slab = source.slab_between_local_z([0.1, 0.3], p).unwrap();
    let w = |x| f.point(x);
    for (a, b) in [
        (Point3::new(0., 0., -10.), Point3::new(0., 0., 10.)),
        (Point3::new(30., 0., -10.), Point3::new(30., 0., 10.)),
        (Point3::new(-24., 0., 2.), Point3::new(24., 0., 2.)),
        (Point3::new(-24., 0., -2.), Point3::new(24., 0., -2.)),
        (Point3::new(16., 0., 2.), Point3::new(16., 0., 3.)),
    ] {
        let r = slab.intersect_segment(w(a), w(b), p).unwrap();
        assert!(r.hits.is_empty() && r.material_intervals.is_empty());
    }
}
#[test]
fn contact_bad_queries_and_original_full_policy_fail_without_mutation_then_recover() {
    let source = body(1., false);
    let p = policy(1.);
    let f = source.frame();
    let slab = source.slab_between_local_z([0.1, 0.3], p).unwrap();
    let before = format!("{:?}", slab.solid());
    let step = slab.export_step_mm(p).unwrap();
    let w = |x| f.point(x);
    let radial = (16_f64 - 0.2 * 0.2).sqrt();
    let rim = (16_f64 - 0.1 * 0.1).sqrt();
    for (a, b) in [
        (Point3::new(16., 0., 0.1), Point3::new(16., 0., 1.)),
        (Point3::new(-24., 0., 0.1), Point3::new(24., 0., 0.1)),
        (
            Point3::new(16. + rim, 0., -1.),
            Point3::new(16. + rim, 0., 1.),
        ),
        (
            Point3::new(-24., 16. + radial, 0.2),
            Point3::new(24., 16. + radial, 0.2),
        ),
        (Point3::new(16., 0., 0.2), Point3::new(16. + 1e-12, 0., 0.2)),
        (Point3::new(-1e12, 0., 0.2), Point3::new(1e12, 0., 0.2)),
    ] {
        assert!(
            slab.intersect_segment(w(a), w(b), p).is_err(),
            "accepted {a:?}->{b:?}"
        );
    }
    for bad in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(f64::INFINITY, 0., 0.),
    ] {
        assert!(slab
            .intersect_segment(bad, w(Point3::new(16., 0., 0.2)), p)
            .is_err());
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(
        slab.intersect_segment(
            w(Point3::new(0., 0., -10.)),
            w(Point3::new(0., 0., 10.)),
            relative
        )
        .is_err(),
        "unresolved slab thickness must reject before void miss"
    );
    let r = slab
        .intersect_segment(
            w(Point3::new(-24., 0., 0.2)),
            w(Point3::new(24., 0., 0.2)),
            p,
        )
        .unwrap();
    assert_eq!(r.hits.len(), 4);
    assert_eq!(before, format!("{:?}", slab.solid()));
    assert_eq!(step, slab.export_step_mm(p).unwrap());
}
