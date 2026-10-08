use hagane::*;
use std::f64::consts::PI;

#[test]
fn eccentric_holes_preserve_first_moment_volume_material_and_mesh() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for center in [[0.7, 0.2], [-0.6, -0.4], [0., 0.]] {
            for slope in [-0.2, 0., 0.2] {
                let s = ellipse_eccentric_planar_demo_solid(
                    3. * scale,
                    scale,
                    6. * scale,
                    slope,
                    center.map(|x| x * scale),
                    t,
                )
                .unwrap();
                s.validate(t.absolute()).unwrap();
                let volume = (24. * PI + slope * PI * center[0]) * scale.powi(3);
                assert!((s.volume().unwrap() - volume).abs() < 1e-10 * scale.powi(3));
                assert_eq!((s.shell.faces.len(), s.edges.len()), (6, 12));
                let bounds = s.bounds();
                assert!((bounds.min.x + 3. * scale).abs() < 1e-12 * scale);
                assert!((bounds.max.z - slope.abs() * 3. * scale).abs() < 1e-12 * scale);
                for angle in [0., 0.37] {
                    let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
                    let placed = s.transformed(transform, t.absolute()).unwrap();
                    for (p, location) in [
                        (
                            Point3::new(center[0], center[1], -1.),
                            PointLocation::Outside,
                        ),
                        (
                            Point3::new(center[0], center[1], -slope * center[0]),
                            PointLocation::Outside,
                        ),
                        (
                            Point3::new(center[0], center[1] + 1., -1.),
                            PointLocation::Boundary,
                        ),
                        (
                            Point3::new(center[0], center[1] + 1., -slope * center[0]),
                            PointLocation::Boundary,
                        ),
                        (Point3::new(0., 2., -1.), PointLocation::Inside),
                        (Point3::new(0., 2., 0.), PointLocation::Boundary),
                        (Point3::new(0., 2., 1.), PointLocation::Outside),
                    ] {
                        assert_eq!(
                            classify_point_in_solid(&placed, transform.point(p * scale), t)
                                .unwrap(),
                            location
                        );
                    }
                }
                let mesh = s.tessellate(0.002 * scale, t.absolute()).unwrap();
                assert!((mesh.signed_volume() - volume).abs() < 0.08 * scale.powi(3));
                let key = |p: Point3| {
                    [
                        (p.x / scale * 1e9).round() as i64,
                        (p.y / scale * 1e9).round() as i64,
                        (p.z / scale * 1e9).round() as i64,
                    ]
                };
                let mut uses = std::collections::BTreeMap::new();
                for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                    if *face == 5 {
                        let centroid = (mesh.positions[tri[0]]
                            + mesh.positions[tri[1]]
                            + mesh.positions[tri[2]])
                            * (1. / 3.);
                        assert!(
                            (centroid.x - center[0] * scale).hypot(centroid.y - center[1] * scale)
                                >= 0.998 * scale
                        );
                    }
                    for i in 0..3 {
                        let a = key(mesh.positions[tri[i]]);
                        let b = key(mesh.positions[tri[(i + 1) % 3]]);
                        let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                        let entry = uses.entry(edge).or_insert((0, 0));
                        entry.0 += 1;
                        entry.1 += sign;
                    }
                }
                assert!(uses.values().all(|v| *v == (2, 0)));
            }
        }
    }
}

#[test]
fn eccentric_crossings_have_independent_roots_and_original_parameters() {
    let t = GeometryTolerance::default();
    for center in [[0.7, 0.2], [-0.6, -0.4]] {
        let source = ellipse_eccentric_planar_demo_solid(3., 1., 6., 0.2, center, t).unwrap();
        let Surface::Plane { origin, u, v } = source.shell.faces[5].surface else {
            panic!("cap")
        };
        let y = center[1] + 0.3;
        let stretch = 1.04f64.sqrt();
        let outer = (9. - y * y).sqrt() * stretch;
        let inner = (1. - 0.3f64.powi(2)).sqrt() * stretch;
        let shift = center[0] * stretch;
        for angle in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
            let s = source.transformed(transform, t.absolute()).unwrap();
            for sign in [-1., 1.] {
                let anchor = transform.point(origin - u * 5. + v * y);
                let direction = transform.vector(u * (2. * sign));
                let clip = clip_line_to_planar_face(&s, 5, anchor, direction, t).unwrap();
                let mut expected =
                    [-outer, shift - inner, shift + inner, outer].map(|x| (5. + x) / (2. * sign));
                expected.sort_by(f64::total_cmp);
                assert_eq!(clip.intervals.len(), 2);
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
                for interval in clip.intervals {
                    let p = anchor
                        + direction
                            * ((interval.parameter_range[0] + interval.parameter_range[1]) / 2.);
                    assert_eq!(
                        classify_point_in_solid(&s, p, t).unwrap(),
                        PointLocation::Boundary
                    );
                }
            }
        }
    }
}

#[test]
fn containment_contacts_precision_and_invalid_centers_are_rejected() {
    let t = GeometryTolerance::default();
    for center in [
        [2., 0.],
        [2. - 1e-9, 0.],
        [3., 0.],
        [0., 2.],
        [f64::NAN, 0.],
        [0., f64::INFINITY],
    ] {
        assert!(ellipse_eccentric_planar_demo_solid(3., 1., 6., 0.2, center, t).is_err());
    }
    let s = ellipse_eccentric_planar_demo_solid(3., 1., 6., 0.2, [0.7, 0.2], t).unwrap();
    let Surface::Plane { origin, u, v } = s.shell.faces[5].surface else {
        panic!("cap")
    };
    for y in [1.2, 1.2 + 1e-9, 0.2] {
        assert!(clip_line_to_planar_face(&s, 5, origin - u * 5. + v * y, u, t).is_err());
    }
    let mut malformed = s.clone();
    for c in &mut malformed.shell.faces[5].wires[1].coedges {
        let PCurve::EllipseArc { center, .. } = &mut c.pcurve else {
            panic!("ellipse")
        };
        center[0] += 1e-9;
    }
    assert!(malformed.validate(t.absolute()).is_err());
    assert!(clip_line_to_planar_face(&malformed, 5, origin + v * 100., u, t).is_err());
    assert!(ellipse_eccentric_planar_demo_json(16., 4., 0.).is_err());
    assert!(ellipse_eccentric_planar_demo_json(6., f64::NAN, 0.).is_err());
}
