use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let f =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn witnesses(
    shape: &NurbsTorusSolid,
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
fn independent_circle_section_roots_and_axial_tube_intersections_match_actual_boundaries() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let shape = body(s, posed);
            let f = shape.frame();
            let world = |p: Point3| f.point(p * s);
            let before = format!("{:?}", shape.solid());
            let step = shape.export_step_mm(policy(s)).unwrap();
            for (y, z) in [(0., 0.), (0., 2.), (5., 1.), (16., 0.)] {
                let delta = (16_f64 - z * z).sqrt();
                let outer = ((16. + delta).powi(2) - y * y).sqrt();
                let mut roots = vec![-outer];
                if y < 16. - delta {
                    let inner = ((16. - delta).powi(2) - y * y).sqrt();
                    roots.extend([-inner, inner]);
                }
                roots.push(outer);
                for reverse in [false, true] {
                    let mut a = world(Point3::new(-24., y, z));
                    let mut b = world(Point3::new(24., y, z));
                    if reverse {
                        std::mem::swap(&mut a, &mut b);
                    }
                    let result = shape.intersect_segment(a, b, policy(s)).unwrap();
                    assert_eq!(result.hits.len(), roots.len());
                    assert_eq!(result.material_intervals.len(), roots.len() / 2);
                    let params: Vec<_> = if reverse {
                        roots.iter().rev().map(|x| (24. - x) / 48.).collect()
                    } else {
                        roots.iter().map(|x| (x + 24.) / 48.).collect()
                    };
                    for (hit, t) in result.hits.iter().zip(&params) {
                        close(hit.parameter, *t);
                    }
                    for (i, interval) in result.material_intervals.iter().enumerate() {
                        close(interval[0], params[2 * i]);
                        close(interval[1], params[2 * i + 1]);
                    }
                    witnesses(&shape, a, b, &result, s);
                }
            }
            let a = world(Point3::new(16., 0., -10.));
            let b = world(Point3::new(16., 0., 10.));
            let r = shape.intersect_segment(a, b, policy(s)).unwrap();
            assert_eq!(r.hits.len(), 2);
            close(r.material_intervals[0][0], 0.3);
            close(r.material_intervals[0][1], 0.7);
            witnesses(&shape, a, b, &r, s);
            let r = shape
                .intersect_segment(
                    world(Point3::new(0., 0., -10.)),
                    world(Point3::new(0., 0., 10.)),
                    policy(s),
                )
                .unwrap();
            assert!(r.hits.is_empty());
            assert!(r.material_intervals.is_empty());
            assert_eq!(before, format!("{:?}", shape.solid()));
            assert_eq!(step, shape.export_step_mm(policy(s)).unwrap());
        }
    }
}
#[test]
fn strict_material_endpoint_clipping_keeps_both_components_and_zero_one_limits() {
    let shape = body(1., false);
    let f = shape.frame();
    for (a, b, expected) in [
        (
            Point3::new(15., 0., 0.),
            Point3::new(17., 0., 0.),
            vec![[0., 1.]],
        ),
        (
            Point3::new(-16., 0., 0.),
            Point3::new(16., 0., 0.),
            vec![[0., 0.125], [0.875, 1.]],
        ),
        (
            Point3::new(0., 0., 0.),
            Point3::new(24., 0., 0.),
            vec![[0.5, 5. / 6.]],
        ),
    ] {
        let a = f.point(a);
        let b = f.point(b);
        let result = shape.intersect_segment(a, b, policy(1.)).unwrap();
        assert_eq!(result.material_intervals.len(), expected.len());
        for (actual, expected) in result.material_intervals.iter().zip(expected) {
            close(actual[0], expected[0]);
            close(actual[1], expected[1]);
        }
        witnesses(&shape, a, b, &result, 1.);
    }
}
#[test]
fn contacts_nonfinite_and_original_precision_refuse_without_false_miss() {
    let shape = body(1., false);
    let f = shape.frame();
    let before = format!("{:?}", shape.solid());
    for (a, b) in [
        (Point3::new(-24., 20., 0.), Point3::new(24., 20., 0.)),
        (Point3::new(-24., 0., 4.), Point3::new(24., 0., 4.)),
        (Point3::new(-24., 12., 0.), Point3::new(24., 12., 0.)),
        (Point3::new(20., 0., 0.), Point3::new(24., 0., 0.)),
        (Point3::new(16., 0., 0.), Point3::new(16. + 1e-8, 0., 0.)),
        (Point3::new(-1e12, 0., 0.), Point3::new(1e12, 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), Point3::new(24., 0., 0.)),
        (
            Point3::new(-24., 0., 0.),
            Point3::new(f64::INFINITY, 0., 0.),
        ),
        (
            Point3::new(-24., 20. - 1e-7, 0.),
            Point3::new(24., 20. - 1e-7, 0.),
        ),
    ] {
        assert!(
            shape
                .intersect_segment(f.point(a), f.point(b), policy(1.))
                .is_err(),
            "{a:?} {b:?}"
        );
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(shape
        .intersect_segment(
            f.point(Point3::new(-24., 20. - 0.01, 0.)),
            f.point(Point3::new(24., 20. - 0.01, 0.)),
            relative
        )
        .is_err());
    assert_eq!(
        shape
            .intersect_segment(
                f.point(Point3::new(-24., 0., 0.)),
                f.point(Point3::new(24., 0., 0.)),
                policy(1.)
            )
            .unwrap()
            .hits
            .len(),
        4
    );
    assert_eq!(before, format!("{:?}", shape.solid()));
}
