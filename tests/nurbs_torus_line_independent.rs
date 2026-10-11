use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1.3e-7, "{a} != {b}");
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let f =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn witness(
    shape: &NurbsTorusSolid,
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
fn original_nonunit_line_parameters_match_independent_finite_circle_sections() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let shape = body(s, posed);
            let f = shape.frame();
            let world = |p: Point3| f.point(p * s);
            let vector = |p: Vec3| {
                f.axes()[0] * (p.x * s) + f.axes()[1] * (p.y * s) + f.axes()[2] * (p.z * s)
            };
            let before = format!("{:?}", shape.solid());
            let step = shape.export_step_mm(policy(s)).unwrap();
            for (y, z) in [(0., 0.), (5., 1.), (16., 0.)] {
                let delta = (16_f64 - z * z).sqrt();
                let outer = ((16. + delta).powi(2) - y * y).sqrt();
                let mut roots = vec![-outer];
                if y < 16. - delta {
                    let inner = ((16. - delta).powi(2) - y * y).sqrt();
                    roots.extend([-inner, inner]);
                }
                roots.push(outer);
                for magnitude in [0.5, 2., -2., 10.] {
                    let o = world(Point3::new(24., y, z));
                    let d = vector(Vec3::new(magnitude, 0., 0.));
                    let r = shape.intersect_line(o, d, policy(s)).unwrap();
                    assert_eq!(r.hits.len(), roots.len());
                    let mut params: Vec<_> = roots.iter().map(|x| (x - 24.) / magnitude).collect();
                    params.sort_by(f64::total_cmp);
                    for (h, t) in r.hits.iter().zip(params) {
                        close(h.parameter, t);
                        assert!((h.parameter - t).abs() * magnitude.abs() < 1e-6 / 8.);
                    }
                    witness(&shape, o, d, &r, s);
                }
            }
            let o = world(Point3::new(16., 0., 0.));
            let d = vector(Vec3::new(0., 0., 2.));
            let r = shape.intersect_line(o, d, policy(s)).unwrap();
            assert_eq!(r.hits.len(), 2);
            close(r.hits[0].parameter, -2.);
            close(r.hits[1].parameter, 2.);
            witness(&shape, o, d, &r, s);
            let r = shape
                .intersect_line(world(Point3::new(0., 0., 0.)), d, policy(s))
                .unwrap();
            assert!(r.hits.is_empty());
            assert!(r.material_intervals.is_empty());
            assert_eq!(before, format!("{:?}", shape.solid()));
            assert_eq!(step, shape.export_step_mm(policy(s)).unwrap());
        }
    }
}
#[test]
fn rays_clip_all_material_intervals_from_void_material_and_behind_origin() {
    let shape = body(1., false);
    let f = shape.frame();
    for (origin, direction, expected, hits) in [
        (
            Point3::new(0., 0., 0.),
            Vec3::new(2., 0., 0.),
            vec![[6., 10.]],
            2,
        ),
        (
            Point3::new(16., 0., 0.),
            Vec3::new(2., 0., 0.),
            vec![[0., 2.]],
            1,
        ),
        (
            Point3::new(-16., 0., 0.),
            Vec3::new(2., 0., 0.),
            vec![[0., 2.], [14., 18.]],
            3,
        ),
        (Point3::new(24., 0., 0.), Vec3::new(2., 0., 0.), vec![], 0),
        (
            Point3::new(24., 0., 0.),
            Vec3::new(-2., 0., 0.),
            vec![[2., 6.], [18., 22.]],
            4,
        ),
    ] {
        let o = f.point(origin);
        let r = shape.intersect_ray(o, direction, policy(1.)).unwrap();
        assert_eq!(r.hits.len(), hits);
        assert_eq!(r.material_intervals.len(), expected.len());
        for (actual, expected) in r.material_intervals.iter().zip(expected) {
            close(actual[0], expected[0]);
            close(actual[1], expected[1]);
        }
        witness(&shape, o, direction, &r, 1.);
    }
}
#[test]
fn original_anchor_precision_contacts_and_bad_directions_reject_then_recover() {
    let shape = body(1., false);
    let f = shape.frame();
    let before = format!("{:?}", shape.solid());
    let step = shape.export_step_mm(policy(1.)).unwrap();
    for (o, d) in [
        (Point3::new(20., 0., 0.), Vec3::new(1., 0., 0.)),
        (Point3::new(-24., 20., 0.), Vec3::new(1., 0., 0.)),
        (Point3::new(-24., 12., 0.), Vec3::new(1., 0., 0.)),
        (Point3::new(-24., 0., 4.), Vec3::new(1., 0., 0.)),
        (Point3::new(1e12, 1e12, 1e12), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 0.), Vec3::new(0., 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), Vec3::new(1., 0., 0.)),
        (Point3::new(0., 0., 0.), Vec3::new(f64::INFINITY, 0., 0.)),
    ] {
        assert!(
            shape.intersect_ray(f.point(o), d, policy(1.)).is_err(),
            "{o:?} {d:?}"
        );
    }
    let r = shape
        .intersect_line(
            f.point(Point3::new(0., 0., 0.)),
            Vec3::new(2., 0., 0.),
            policy(1.),
        )
        .unwrap();
    assert_eq!(r.hits.len(), 4);
    assert_eq!(before, format!("{:?}", shape.solid()));
    assert_eq!(step, shape.export_step_mm(policy(1.)).unwrap());
}
