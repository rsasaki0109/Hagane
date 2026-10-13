use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn physical(actual: f64, expected: f64, direction: Vec3, scale: f64) {
    assert!(
        (actual - expected).abs() * direction.norm() < 1e-6 * scale / 8.,
        "parameter{actual} vs{expected}, original direction{direction:?}"
    );
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let f =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn witness(
    shape: &NurbsTorusHorizontalSlab,
    o: Point3,
    d: Vec3,
    result: &NurbsTorusLineIntersection,
    s: f64,
) {
    let mut previous = f64::NEG_INFINITY;
    for h in &result.hits {
        assert!(h.parameter > previous);
        previous = h.parameter;
        assert!((o + d * h.parameter - h.point).norm() < 1e-7 * s);
        let w = &h.boundary;
        let actual = shape.solid().shell.faces[w.face]
            .surface
            .try_evaluate(w.uv[0], w.uv[1])
            .unwrap();
        assert!((actual - h.point).norm() < 1e-7 * s);
        for interval in &result.material_intervals {
            if (h.parameter - interval[0]).abs() < 1e-8 {
                assert!(w.normal.dot(d) < 0.);
            }
            if (h.parameter - interval[1]).abs() < 1e-8 {
                assert!(w.normal.dot(d) > 0.);
            }
        }
    }
    for interval in &result.material_intervals {
        assert!(interval[0] < interval[1]);
        assert_eq!(
            shape
                .classify_point(o + d * ((interval[0] + interval[1]) / 2.), policy(s))
                .unwrap(),
            PointLocation::Inside
        );
    }
    for pair in result.material_intervals.windows(2) {
        assert_eq!(
            shape
                .classify_point(o + d * ((pair[0][1] + pair[1][0]) / 2.), policy(s))
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn original_nonunit_line_parameters_match_walls_caps_and_mixed_sloped_roots() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let p = policy(s);
            let f = source.frame();
            let original = format!("{:?}", source.solid());
            let source_step = source.export_step_mm(p).unwrap();
            for heights in [[-1.25, 1.25], [0.1, 0.3], [-0.3, -0.1]] {
                let slab = source
                    .slab_between_local_z(heights.map(|x| x * s), p)
                    .unwrap();
                let before = format!("{:?}", slab.solid());
                let step = slab.export_step_mm(p).unwrap();
                let z = (heights[0] + heights[1]) / 2.;
                for magnitude in [0.5, 2., -2., 10.] {
                    let a = (16_f64 - z * z).sqrt();
                    let xs = [-16. - a, -16. + a, 16. - a, 16. + a];
                    let origin = f.point(Point3::new(24., 0., z) * s);
                    let d = f.vector(Vec3::new(magnitude, 0., 0.) * s);
                    let mut expected: Vec<_> = xs.iter().map(|x| (x - 24.) / magnitude).collect();
                    expected.sort_by(f64::total_cmp);
                    let r = slab.intersect_line(origin, d, p).unwrap();
                    assert_eq!(r.hits.len(), 4);
                    assert_eq!(r.material_intervals.len(), 2);
                    for (h, t) in r.hits.iter().zip(&expected) {
                        physical(h.parameter, *t, d, s);
                    }
                    for (i, v) in r.material_intervals.iter().enumerate() {
                        physical(v[0], expected[2 * i], d, s);
                        physical(v[1], expected[2 * i + 1], d, s);
                    }
                    witness(&slab, origin, d, &r, s);
                    let origin = f.point(Point3::new(16., 0., 0.) * s);
                    let d = f.vector(Vec3::new(0., 0., magnitude) * s);
                    let mut expected = heights.map(|z| z / magnitude);
                    expected.sort_by(f64::total_cmp);
                    let r = slab.intersect_line(origin, d, p).unwrap();
                    assert_eq!(r.hits.len(), 2);
                    assert_eq!(r.material_intervals.len(), 1);
                    for (h, t) in r.hits.iter().zip(expected) {
                        physical(h.parameter, t, d, s);
                        assert!(matches!(
                            slab.solid().shell.faces[h.boundary.face].surface,
                            Surface::Plane { .. }
                        ));
                    }
                    witness(&slab, origin, d, &r, s);
                }
                assert_eq!(before, format!("{:?}", slab.solid()));
                assert_eq!(step, slab.export_step_mm(p).unwrap());
            }
            let slab = source.slab_between_local_z([0.5 * s, 2. * s], p).unwrap();
            let origin = f.point(Point3::new(16., 0., 0.) * s);
            for multiplier in [0.5, 2., -2.] {
                let d = f.vector(Vec3::new(4., 0., 2.) * (s * multiplier));
                let mut expected = [0.25 / multiplier, 0.8_f64.sqrt() / multiplier];
                expected.sort_by(f64::total_cmp);
                let r = slab.intersect_line(origin, d, p).unwrap();
                assert_eq!(r.hits.len(), 2);
                assert_eq!(r.material_intervals.len(), 1);
                for (h, t) in r.hits.iter().zip(expected) {
                    physical(h.parameter, t, d, s);
                }
                witness(&slab, origin, d, &r, s);
            }
            assert_eq!(original, format!("{:?}", source.solid()));
            assert_eq!(source_step, source.export_step_mm(p).unwrap());
        }
    }
}
#[test]
fn rays_clip_every_ring_interval_and_finite_cap_void_extensions_are_misses() {
    let source = body(1., true);
    let p = policy(1.);
    let f = source.frame();
    let slab = source.slab_between_local_z([0.1, 0.3], p).unwrap();
    let z = 0.2;
    let a = (16_f64 - z * z).sqrt();
    let inner = 16. - a;
    let outer = 16. + a;
    for (x, magnitude, expected, hits) in [
        (0., 2., vec![[inner / 2., outer / 2.]], 2),
        (16., 2., vec![[0., (outer - 16.) / 2.]], 1),
        (
            -16.,
            2.,
            vec![
                [0., (16. - inner) / 2.],
                [(16. + inner) / 2., (16. + outer) / 2.],
            ],
            3,
        ),
        (24., 2., vec![], 0),
        (
            24.,
            -2.,
            vec![
                [(24. - outer) / 2., (24. - inner) / 2.],
                [(24. + inner) / 2., (24. + outer) / 2.],
            ],
            4,
        ),
    ] {
        let o = f.point(Point3::new(x, 0., z));
        let d = f.vector(Vec3::new(magnitude, 0., 0.));
        let r = slab.intersect_ray(o, d, p).unwrap();
        assert_eq!(r.hits.len(), hits);
        assert_eq!(r.material_intervals.len(), expected.len());
        for (v, t) in r.material_intervals.iter().zip(expected) {
            physical(v[0], t[0], d, 1.);
            physical(v[1], t[1], d, 1.);
        }
        witness(&slab, o, d, &r, 1.);
    }
    let o = f.point(Point3::new(16., 0., z));
    let d = f.vector(Vec3::new(0., 0., 2.));
    let r = slab.intersect_ray(o, d, p).unwrap();
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.material_intervals.len(), 1);
    physical(r.material_intervals[0][0], 0., d, 1.);
    physical(r.hits[0].parameter, 0.05, d, 1.);
    witness(&slab, o, d, &r, 1.);
    for x in [0., 30.] {
        let o = f.point(Point3::new(x, 0., 0.));
        let d = f.vector(Vec3::new(0., 0., 2.));
        for r in [
            slab.intersect_line(o, d, p).unwrap(),
            slab.intersect_ray(o, d, p).unwrap(),
        ] {
            assert!(r.hits.is_empty() && r.material_intervals.is_empty());
        }
    }
    for z in [-2., 2.] {
        let r = slab
            .intersect_line(
                f.point(Point3::new(0., 0., z)),
                f.vector(Vec3::new(2., 0., 0.)),
                p,
            )
            .unwrap();
        assert!(r.hits.is_empty() && r.material_intervals.is_empty());
    }
}
#[test]
fn boundary_contacts_and_bad_original_queries_reject_before_empty_shortcuts_then_recover() {
    let source = body(1., false);
    let p = policy(1.);
    let f = source.frame();
    let slab = source.slab_between_local_z([0.1, 0.3], p).unwrap();
    let before = format!("{:?}", slab.solid());
    let step = slab.export_step_mm(p).unwrap();
    let outer = 16. + (16_f64 - 0.2 * 0.2).sqrt();
    let rim = 16. + (16_f64 - 0.1 * 0.1).sqrt();
    for (o, d) in [
        (Point3::new(16., 0., 0.1), Vec3::new(0., 0., 2.)),
        (Point3::new(outer, 0., 0.2), Vec3::new(2., 0., 0.)),
        (Point3::new(rim, 0., 0.1), Vec3::new(0., 0., 2.)),
        (Point3::new(24., outer, 0.2), Vec3::new(2., 0., 0.)),
        (Point3::new(16., 0., 0.1), Vec3::new(2., 0., 0.)),
        (Point3::new(0., 0., 0.2), Vec3::new(0., 0., 0.)),
        (Point3::new(1e12, 0., 0.2), Vec3::new(2., 0., 0.)),
    ] {
        assert!(
            slab.intersect_ray(f.point(o), f.vector(d), p).is_err(),
            "accepted{o:?},{d:?}"
        );
    }
    for (o, d) in [
        (Point3::new(f64::NAN, 0., 0.), Vec3::new(2., 0., 0.)),
        (Point3::new(0., 0., 0.), Vec3::new(f64::INFINITY, 0., 0.)),
        (Point3::new(0., 0., 0.), Vec3::new(f64::NAN, 0., 0.)),
    ] {
        assert!(slab.intersect_line(o, d, p).is_err());
    }
    assert!(
        slab.intersect_line(
            f.point(Point3::new(0., 0., 0.2)),
            f.vector(Vec3::new(1e-320, 0., 0.)),
            p
        )
        .is_err(),
        "physical wall intersections need unrepresentable original parameters"
    );
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(slab
        .intersect_line(
            f.point(Point3::new(0., 0., 0.)),
            f.vector(Vec3::new(0., 0., 2.)),
            relative
        )
        .is_err());
    let r = slab
        .intersect_line(
            f.point(Point3::new(0., 0., 0.2)),
            f.vector(Vec3::new(2., 0., 0.)),
            p,
        )
        .unwrap();
    assert_eq!(r.hits.len(), 4);
    assert_eq!(before, format!("{:?}", slab.solid()));
    assert_eq!(step, slab.export_step_mm(p).unwrap());
}
