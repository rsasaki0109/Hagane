use hagane::*;
use std::f64::consts::PI;

#[test]
fn unequal_axis_holes_form_closed_solids_with_independent_volume_and_membership() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for tilt in [-0.9, -0.5, 0., 0.5, 0.9] {
            let s = tilted_bore_demo_solid(3. * scale, 0.6 * scale, 2. * scale, tilt, t).unwrap();
            s.validate(t.absolute()).unwrap();
            assert_eq!(
                (s.shell.faces.len(), s.edges.len(), s.vertices.len()),
                (6, 12, 8)
            );
            let volume = PI * (9. - 0.36 / tilt.cos()) * 2. * scale.powi(3);
            assert!((s.volume().unwrap() - volume).abs() < 1e-10 * scale.powi(3));
            let bounds = s.bounds();
            for (actual, expected) in [
                (bounds.min.x, -3. * scale),
                (bounds.max.y, 3. * scale),
                (bounds.min.z, -scale),
                (bounds.max.z, scale),
            ] {
                assert!((actual - expected).abs() < 1e-12 * scale);
            }
            for angle in [0., 0.37] {
                let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
                let placed = s.transformed(transform, t.absolute()).unwrap();
                for (p, location) in [
                    (Point3::new(0., 0., 0.), PointLocation::Outside),
                    (Point3::new(0., 1.5, 0.), PointLocation::Inside),
                    (Point3::new(0., 0.6, 0.), PointLocation::Boundary),
                    (Point3::new(0., 1.5, 1.), PointLocation::Boundary),
                    (Point3::new(tilt.tan(), 0., 1.), PointLocation::Outside),
                    (Point3::new(tilt.tan(), 0.6, 1.), PointLocation::Boundary),
                    (Point3::new(0., 1.5, 1.1), PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&placed, transform.point(p * scale), t).unwrap(),
                        location
                    );
                }
            }
            for (distance, location) in [
                (-2. * epsilon, PointLocation::Outside),
                (2. * epsilon, PointLocation::Inside),
            ] {
                assert_eq!(
                    classify_point_in_solid(&s, Point3::new(0., 0.6 * scale + distance, 0.), t)
                        .unwrap(),
                    location
                );
            }
            let mesh = s.tessellate(0.002 * scale, t.absolute()).unwrap();
            assert!((mesh.signed_volume() - volume).abs() < 0.15 * scale.powi(3));
            let key = |p: Point3| {
                [
                    (p.x / scale * 1e9).round() as i64,
                    (p.y / scale * 1e9).round() as i64,
                    (p.z / scale * 1e9).round() as i64,
                ]
            };
            let mut uses = std::collections::BTreeMap::new();
            let mut sections = 0;
            for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                let points = tri.map(|i| mesh.positions[i]);
                if *face >= 4 {
                    let p = (points[0] + points[1] + points[2]) * (1. / 3.);
                    let center = p.z * tilt.tan();
                    assert!(
                        ((p.x - center) * tilt.cos()).hypot(p.y) / (0.6 * scale)
                            >= 1. - 0.002 / 0.6
                    );
                }
                for i in 0..3 {
                    let a = key(points[i]);
                    let b = key(points[(i + 1) % 3]);
                    let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                    let entry = uses.entry(edge).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += sign;
                    if *face >= 4
                        || (points[i].z - points[(i + 1) % 3].z).abs() > scale * 1e-10
                        || (points[i].z.abs() - scale).abs() > scale * 1e-10
                    {
                        continue;
                    }
                    let radius = if *face < 2 { 3. * scale } else { 0.6 * scale };
                    let cos = if *face < 2 { 1. } else { tilt.cos() };
                    let center = if *face < 2 {
                        0.
                    } else {
                        points[i].z * tilt.tan()
                    };
                    let a = points[i];
                    let b = points[(i + 1) % 3];
                    let start = a.y.atan2((a.x - center) * cos);
                    let mut sweep = b.y.atan2((b.x - center) * cos) - start;
                    if sweep > PI {
                        sweep -= 2. * PI;
                    } else if sweep < -PI {
                        sweep += 2. * PI;
                    }
                    for fraction in [0.25, 0.5, 0.75] {
                        let angle = start + sweep * fraction;
                        let exact = Point3::new(
                            center + radius / cos * angle.cos(),
                            radius * angle.sin(),
                            a.z,
                        );
                        assert!(
                            (exact - (a + (b - a) * fraction)).norm()
                                <= 0.002 * scale * (1. + 1e-10)
                        );
                    }
                    sections += 1;
                }
            }
            assert!(uses.values().all(|v| *v == (2, 0)));
            assert!(sections >= 8);
        }
    }
}

#[test]
fn tilted_bore_cap_clips_return_original_parameters_and_unequal_axis_roots() {
    let t = GeometryTolerance::default();
    for tilt in [-0.5, 0., 0.5] {
        let source = tilted_bore_demo_solid(3., 0.6, 2., tilt, t).unwrap();
        let Surface::Plane { origin, u, v } = source.shell.faces[5].surface else {
            panic!("cap")
        };
        let y = 0.3f64;
        let outer = (9. - y * y).sqrt();
        let inner = (0.36 - y * y).sqrt() / tilt.cos();
        let center = tilt.tan();
        for placement in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), placement).unwrap();
            let s = source.transformed(transform, t.absolute()).unwrap();
            for sign in [-1., 1.] {
                let clip = clip_line_to_planar_face(
                    &s,
                    5,
                    transform.point(origin - u * 4. + v * y),
                    transform.vector(u * (2. * sign)),
                    t,
                )
                .unwrap();
                assert_eq!(clip.intervals.len(), 2);
                let mut expected =
                    [-outer, center - inner, center + inner, outer].map(|x| (4. + x) / (2. * sign));
                expected.sort_by(f64::total_cmp);
                assert_eq!(
                    clip.events.iter().map(|e| e.wire).collect::<Vec<_>>(),
                    [0, 1, 1, 0]
                );
                for (event, parameter) in clip.events.iter().zip(expected) {
                    assert!((event.parameter - parameter).abs() < 1e-12);
                    assert!(
                        (s.edges[event.edge].curve.evaluate(event.edge_parameter) - event.point)
                            .norm()
                            < 1e-10
                    );
                }
            }
        }
    }
}

#[test]
fn touching_outside_degenerate_and_unresolved_bore_inputs_are_rejected() {
    let t = GeometryTolerance::default();
    for (radius, bore, height, tilt) in [
        (0., 0.6, 2., 0.5),
        (3., 0., 2., 0.5),
        (3., 0.6, 0., 0.5),
        (3., 0.6, 2., PI / 3. + 0.01),
        (3., 0.6, 2., f64::NAN),
        (1., 0.6, 2., 0.5),
    ] {
        assert!(tilted_bore_demo_solid(radius, bore, height, tilt, t).is_err());
    }
    let angle = 0.5f64;
    let contact = angle.tan() + 0.6 / angle.cos();
    assert!(tilted_bore_demo_solid(contact, 0.6, 2., angle, t).is_err());
    assert!(tilted_bore_demo_solid(contact + 1e-9, 0.6, 2., angle, t).is_err());
    let s = tilted_bore_demo_solid(3., 0.6, 2., 0.5, t).unwrap();
    let Surface::Plane { origin, u, v } = s.shell.faces[5].surface else {
        panic!("cap")
    };
    for y in [0., 0.6, 0.6 + 1e-9, 3.] {
        assert!(clip_line_to_planar_face(&s, 5, origin - u * 4. + v * y, u, t).is_err());
    }
    assert!(tilted_bore_demo_json(0.5, f64::NAN, 0.).is_err());
    assert!(tilted_bore_demo_json(0.5, 3., f64::INFINITY).is_err());
}
