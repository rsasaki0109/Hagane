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
    shape: &NurbsTorusHorizontalSolid,
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
fn retained_horizontal_circle_sections_and_cap_clipped_axial_segments_match_independent_roots() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let p = policy(s);
            let split = source.split_by_local_z(1.25 * s, p).unwrap();
            let world = |q: Point3| source.frame().point(q * s);
            for child in [&split.lower, &split.upper] {
                let before = format!("{:?}", child.solid());
                let step = child.export_step_mm(p).unwrap();
                let z = if child.is_lower() { -1. } else { 2. };
                let a = (16_f64 - z * z).sqrt();
                let rho = [16. - a, 16. + a];
                for y in [0., 5.] {
                    let xs = [
                        -(rho[1] * rho[1] - y * y).sqrt(),
                        -(rho[0] * rho[0] - y * y).sqrt(),
                        (rho[0] * rho[0] - y * y).sqrt(),
                        (rho[1] * rho[1] - y * y).sqrt(),
                    ];
                    for reversed in [false, true] {
                        let (start, end) = if reversed {
                            (Point3::new(24., y, z), Point3::new(-24., y, z))
                        } else {
                            (Point3::new(-24., y, z), Point3::new(24., y, z))
                        };
                        let mut expected: Vec<_> = xs
                            .iter()
                            .map(|x| (x - start.x) / (end.x - start.x))
                            .collect();
                        expected.sort_by(f64::total_cmp);
                        let result = child
                            .intersect_segment(world(start), world(end), p)
                            .unwrap();
                        assert_eq!(result.hits.len(), 4);
                        assert_eq!(result.material_intervals.len(), 2);
                        for (hit, t) in result.hits.iter().zip(&expected) {
                            close(hit.parameter, *t);
                        }
                        for (index, interval) in result.material_intervals.iter().enumerate() {
                            close(interval[0], expected[2 * index]);
                            close(interval[1], expected[2 * index + 1]);
                        }
                        witnesses(child, world(start), world(end), &result, s);
                    }
                }
                let a = world(Point3::new(16., 0., -10.));
                let b = world(Point3::new(16., 0., 10.));
                let result = child.intersect_segment(a, b, p).unwrap();
                assert_eq!(result.hits.len(), 2);
                assert_eq!(result.material_intervals.len(), 1);
                let expected = if child.is_lower() {
                    [0.3, 0.5625]
                } else {
                    [0.5625, 0.7]
                };
                for (hit, t) in result.hits.iter().zip(expected) {
                    close(hit.parameter, t);
                }
                witnesses(child, a, b, &result, s);
                assert!(result.hits.iter().any(|h| matches!(
                    child.solid().shell.faces[h.boundary.face].surface,
                    Surface::Plane { .. }
                )));
                let start = world(Point3::new(16., 0., z));
                let end = world(Point3::new(24., 0., z));
                let result = child.intersect_segment(start, end, p).unwrap();
                assert_eq!(result.hits.len(), 1);
                assert_eq!(result.material_intervals.len(), 1);
                close(result.material_intervals[0][0], 0.);
                witnesses(child, start, end, &result, s);
                assert_eq!(before, format!("{:?}", child.solid()));
                assert_eq!(step, child.export_step_mm(p).unwrap());
            }
        }
    }
}
#[test]
fn finite_annular_void_and_removed_halfspace_have_no_phantom_plane_hits() {
    let source = body(1., true);
    let p = policy(1.);
    let split = source.split_by_local_z(1.25, p).unwrap();
    let w = |v| source.frame().point(v);
    for child in [&split.lower, &split.upper] {
        for (a, b) in [
            (Point3::new(0., 0., -10.), Point3::new(0., 0., 10.)),
            (Point3::new(30., 0., -10.), Point3::new(30., 0., 10.)),
            (
                Point3::new(-24., 0., if child.is_lower() { 2. } else { -2. }),
                Point3::new(24., 0., if child.is_lower() { 2. } else { -2. }),
            ),
        ] {
            let result = child.intersect_segment(w(a), w(b), p).unwrap();
            assert!(result.hits.is_empty());
            assert!(result.material_intervals.is_empty());
        }
        let z = if child.is_lower() { -1. } else { 2. };
        let a = w(Point3::new(15.5, 0., z));
        let b = w(Point3::new(16.5, 0., z));
        let result = child.intersect_segment(a, b, p).unwrap();
        assert!(result.hits.is_empty());
        assert_eq!(result.material_intervals, vec![[0., 1.]]);
        witnesses(child, a, b, &result, 1.);
    }
}
#[test]
fn contact_tangent_unresolved_and_invalid_queries_fail_atomically_then_recover() {
    let source = body(1., false);
    let p = policy(1.);
    let split = source.split_by_local_z(1.25, p).unwrap();
    let w = |v| source.frame().point(v);
    let radial = (16_f64 - 1.25 * 1.25).sqrt();
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(p).unwrap();
        let z = if child.is_lower() { -1. } else { 2. };
        let outer = 16. + (16_f64 - z * z).sqrt();
        for (a, b) in [
            (Point3::new(-24., 0., 1.25), Point3::new(24., 0., 1.25)),
            (
                Point3::new(16. + radial, 0., -10.),
                Point3::new(16. + radial, 0., 10.),
            ),
            (Point3::new(-24., 20., 0.), Point3::new(24., 20., 0.)),
            (Point3::new(outer, 0., z), Point3::new(24., 0., z)),
            (Point3::new(16., 0., z), Point3::new(16. + 1e-12, 0., z)),
            (Point3::new(-1e12, 0., z), Point3::new(1e12, 0., z)),
        ] {
            assert!(
                child.intersect_segment(w(a), w(b), p).is_err(),
                "unexpected contact success {a:?}->{b:?}"
            );
        }
        let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
        let near_z = 1.25 + if child.is_lower() { -0.001 } else { 0.001 };
        let near_a = w(Point3::new(-24., 0., near_z));
        let near_b = w(Point3::new(24., 0., near_z));
        assert_eq!(
            child
                .intersect_segment(near_a, near_b, p)
                .unwrap()
                .hits
                .len(),
            4
        );
        assert!(child.intersect_segment(near_a, near_b, relative).is_err());
        for a in [
            Point3::new(f64::NAN, 0., 0.),
            Point3::new(f64::INFINITY, 0., 0.),
        ] {
            assert!(child
                .intersect_segment(a, w(Point3::new(16., 0., z)), p)
                .is_err());
        }
        let result = child
            .intersect_segment(w(Point3::new(-24., 0., z)), w(Point3::new(24., 0., z)), p)
            .unwrap();
        assert_eq!(result.hits.len(), 4);
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(p).unwrap());
    }
}
