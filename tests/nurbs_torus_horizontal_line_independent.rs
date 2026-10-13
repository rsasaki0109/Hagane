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
    shape: &NurbsTorusHorizontalSolid,
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
fn original_nonunit_parameters_match_independent_circle_sections_and_vertical_cap_wall() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let p = policy(s);
            let split = source.split_by_local_z(1.25 * s, p).unwrap();
            let f = source.frame();
            for child in [&split.lower, &split.upper] {
                let before = format!("{:?}", child.solid());
                let step = child.export_step_mm(p).unwrap();
                let z = if child.is_lower() { -1. } else { 2. };
                for y in [0., 5.] {
                    let a = (16_f64 - z * z).sqrt();
                    let inner = 16. - a;
                    let outer = 16. + a;
                    let roots = [
                        -(outer * outer - y * y).sqrt(),
                        -(inner * inner - y * y).sqrt(),
                        (inner * inner - y * y).sqrt(),
                        (outer * outer - y * y).sqrt(),
                    ];
                    for magnitude in [0.5, 2., -2., 10.] {
                        let origin = f.point(Point3::new(24., y, z) * s);
                        let direction = f.vector(Vec3::new(magnitude, 0., 0.) * s);
                        let result = child.intersect_line(origin, direction, p).unwrap();
                        assert_eq!(result.hits.len(), 4);
                        assert_eq!(result.material_intervals.len(), 2);
                        let mut expected: Vec<_> =
                            roots.iter().map(|x| (x - 24.) / magnitude).collect();
                        expected.sort_by(f64::total_cmp);
                        for (hit, want) in result.hits.iter().zip(&expected) {
                            assert!((hit.parameter - want).abs() * magnitude.abs() < 1e-6 / 8.);
                        }
                        for (i, interval) in result.material_intervals.iter().enumerate() {
                            close(interval[0], expected[2 * i]);
                            close(interval[1], expected[2 * i + 1]);
                        }
                        witness(child, origin, direction, &result, s);
                    }
                }
                let o = f.point(Point3::new(16., 0., 0.) * s);
                let d = f.vector(Vec3::new(0., 0., 2.) * s);
                let result = child.intersect_line(o, d, p).unwrap();
                let expected = if child.is_lower() {
                    [-2., 0.625]
                } else {
                    [0.625, 2.]
                };
                assert_eq!(result.hits.len(), 2);
                assert_eq!(result.material_intervals.len(), 1);
                for (hit, want) in result.hits.iter().zip(expected) {
                    close(hit.parameter, want);
                }
                assert!(result.hits.iter().any(|h| matches!(
                    child.solid().shell.faces[h.boundary.face].surface,
                    Surface::Plane { .. }
                )));
                witness(child, o, d, &result, s);
                assert_eq!(before, format!("{:?}", child.solid()));
                assert_eq!(step, child.export_step_mm(p).unwrap());
            }
        }
    }
}
#[test]
fn rays_clip_all_intervals_and_finite_cap_extensions_have_no_phantom_hit() {
    let source = body(1., true);
    let p = policy(1.);
    let f = source.frame();
    let split = source.split_by_local_z(1.25, p).unwrap();
    for child in [&split.lower, &split.upper] {
        let z = if child.is_lower() { -1. } else { 2. };
        let a = (16_f64 - z * z).sqrt();
        let inner = 16. - a;
        let outer = 16. + a;
        for (x, direction, intervals, hits) in [
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
            let d = f.vector(Vec3::new(direction, 0., 0.));
            let r = child.intersect_ray(o, d, p).unwrap();
            assert_eq!(r.hits.len(), hits);
            assert_eq!(r.material_intervals.len(), intervals.len());
            for (got, want) in r.material_intervals.iter().zip(intervals) {
                close(got[0], want[0]);
                close(got[1], want[1]);
            }
            witness(child, o, d, &r, 1.);
        }
        for o in [Point3::new(0., 0., 0.), Point3::new(30., 0., 0.)] {
            let r = child
                .intersect_line(f.point(o), f.vector(Vec3::new(0., 0., 2.)), p)
                .unwrap();
            assert!(r.hits.is_empty() && r.material_intervals.is_empty());
        }
        let removed_z = if child.is_lower() { 2. } else { -2. };
        let r = child
            .intersect_line(
                f.point(Point3::new(0., 0., removed_z)),
                f.vector(Vec3::new(2., 0., 0.)),
                p,
            )
            .unwrap();
        assert!(r.hits.is_empty() && r.material_intervals.is_empty());
    }
}
#[test]
fn original_query_contacts_bad_directions_and_precision_refuse_then_recover() {
    let source = body(1., false);
    let p = policy(1.);
    let f = source.frame();
    let split = source.split_by_local_z(1.25, p).unwrap();
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(p).unwrap();
        let z = if child.is_lower() { -1. } else { 2. };
        let outer = 16. + (16_f64 - z * z).sqrt();
        for (origin, direction) in [
            (Point3::new(16., 0., 1.25), Vec3::new(0., 0., 2.)),
            (Point3::new(outer, 0., z), Vec3::new(2., 0., 0.)),
            (
                Point3::new(16. + (16_f64 - 1.25 * 1.25).sqrt(), 0., 1.25),
                Vec3::new(0., 0., 2.),
            ),
            (Point3::new(24., 20., 0.), Vec3::new(2., 0., 0.)),
            (Point3::new(0., 0., z), Vec3::new(0., 0., 0.)),
            (Point3::new(1e12, 0., z), Vec3::new(2., 0., 0.)),
        ] {
            assert!(child
                .intersect_ray(f.point(origin), f.vector(direction), p)
                .is_err());
        }
        for (o, d) in [
            (Point3::new(f64::NAN, 0., 0.), Vec3::new(2., 0., 0.)),
            (Point3::new(0., 0., 0.), Vec3::new(f64::INFINITY, 0., 0.)),
        ] {
            assert!(child.intersect_line(o, d, p).is_err());
        }
        let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
        let near_z = 1.25 + if child.is_lower() { -0.001 } else { 0.001 };
        let o = f.point(Point3::new(0., 0., near_z));
        let d = f.vector(Vec3::new(2., 0., 0.));
        assert_eq!(child.intersect_line(o, d, p).unwrap().hits.len(), 4);
        assert!(child.intersect_line(o, d, relative).is_err());
        assert_eq!(
            child
                .intersect_line(f.point(Point3::new(0., 0., z)), d, p)
                .unwrap()
                .hits
                .len(),
            4
        );
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(p).unwrap());
    }
}
