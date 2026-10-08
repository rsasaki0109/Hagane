use hagane::*;
use std::f64::consts::PI;
fn holes() -> Vec<EllipseCapHole> {
    vec![
        EllipseCapHole {
            radius: 0.5,
            center: [-2., 0.],
        },
        EllipseCapHole {
            radius: 0.4,
            center: [0.1, 0.],
        },
        EllipseCapHole {
            radius: 0.6,
            center: [2., 0.],
        },
    ]
}

#[test]
fn multiple_holes_preserve_volume_material_and_oriented_mesh() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for slope in [-0.2, 0., 0.2] {
            let input = holes()
                .into_iter()
                .map(|h| EllipseCapHole {
                    radius: h.radius * scale,
                    center: h.center.map(|x| x * scale),
                })
                .collect::<Vec<_>>();
            let s = ellipse_multi_hole_planar_demo_solid(4. * scale, 6. * scale, slope, &input, t)
                .unwrap();
            s.validate(t.absolute()).unwrap();
            assert_eq!(
                (s.shell.faces.len(), s.edges.len(), s.vertices.len()),
                (10, 24, 16)
            );
            assert_eq!(s.shell.faces[9].wires.len(), 4);
            let expected = PI
                * (16. * 3.
                    - holes()
                        .iter()
                        .map(|h| h.radius.powi(2) * (3. - slope * h.center[0]))
                        .sum::<f64>())
                * scale.powi(3);
            assert!((s.volume().unwrap() - expected).abs() < 1e-10 * scale.powi(3));
            for angle in [0., 0.37] {
                let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
                let placed = s.transformed(transform, t.absolute()).unwrap();
                for hole in &input {
                    for (p, location) in [
                        (
                            Point3::new(hole.center[0], hole.center[1], -scale),
                            PointLocation::Outside,
                        ),
                        (
                            Point3::new(hole.center[0], hole.center[1] + hole.radius, -scale),
                            PointLocation::Boundary,
                        ),
                        (
                            Point3::new(
                                hole.center[0],
                                hole.center[1] + hole.radius,
                                -slope * hole.center[0],
                            ),
                            PointLocation::Boundary,
                        ),
                    ] {
                        assert_eq!(
                            classify_point_in_solid(&placed, transform.point(p), t).unwrap(),
                            location
                        );
                    }
                }
                for (z, location) in [
                    (-scale, PointLocation::Inside),
                    (0., PointLocation::Boundary),
                    (scale, PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(
                            &placed,
                            transform.point(Point3::new(0., 2. * scale, z)),
                            t
                        )
                        .unwrap(),
                        location
                    );
                }
            }
            let mesh = s.tessellate(0.002 * scale, t.absolute()).unwrap();
            assert!((mesh.signed_volume() - expected).abs() < 0.15 * scale.powi(3));
            let key = |p: Point3| {
                [
                    (p.x / scale * 1e9).round() as i64,
                    (p.y / scale * 1e9).round() as i64,
                    (p.z / scale * 1e9).round() as i64,
                ]
            };
            let mut uses = std::collections::BTreeMap::new();
            let mut checked_chords = 0;
            for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                if *face == 9 {
                    let p =
                        (mesh.positions[tri[0]] + mesh.positions[tri[1]] + mesh.positions[tri[2]])
                            * (1. / 3.);
                    for hole in &input {
                        assert!(
                            (p.x - hole.center[0]).hypot(p.y - hole.center[1])
                                >= hole.radius - 0.002 * scale
                        );
                    }
                }
                if (1..=8).contains(face) {
                    let (radius, center) = if *face <= 2 {
                        (4. * scale, [0., 0.])
                    } else {
                        let h = input[(*face - 3) / 2];
                        (h.radius, h.center)
                    };
                    for i in 0..3 {
                        let a = mesh.positions[tri[i]];
                        let b = mesh.positions[tri[(i + 1) % 3]];
                        if (a.z + slope * a.x).abs() > 1e-10 * scale
                            || (b.z + slope * b.x).abs() > 1e-10 * scale
                        {
                            continue;
                        }
                        checked_chords += 1;
                        let start = (a.y - center[1]).atan2(a.x - center[0]);
                        let mut sweep = (b.y - center[1]).atan2(b.x - center[0]) - start;
                        if sweep > PI {
                            sweep -= 2. * PI;
                        } else if sweep < -PI {
                            sweep += 2. * PI;
                        }
                        for fraction in [0.25, 0.5, 0.75] {
                            let angle = start + sweep * fraction;
                            let x = center[0] + radius * angle.cos();
                            let exact =
                                Point3::new(x, center[1] + radius * angle.sin(), -slope * x);
                            assert!(
                                (exact - (a + (b - a) * fraction)).norm()
                                    <= 0.002 * scale * (1. + 1e-10)
                            );
                        }
                    }
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
            assert!(checked_chords >= 8);
        }
    }
}

#[test]
fn intervals_and_wire_ids_are_independent_of_hole_order_and_line_direction() {
    let t = GeometryTolerance::default();
    for order in [vec![0, 1, 2], vec![2, 0, 1]] {
        let all = holes();
        let input = order.iter().map(|&i| all[i]).collect::<Vec<_>>();
        let source = ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &input, t).unwrap();
        let Surface::Plane { origin, u, v } = source.shell.faces[9].surface else {
            panic!("cap")
        };
        for angle in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
            let s = source.transformed(transform, t.absolute()).unwrap();
            for sign in [-1., 1.] {
                let anchor = transform.point(origin - u * 5. + v * 0.2);
                let direction = transform.vector(u * (2. * sign));
                let clip = clip_line_to_planar_face(&s, 9, anchor, direction, t).unwrap();
                assert_eq!(clip.intervals.len(), 4);
                assert_eq!(clip.events.len(), 8);
                let stretch = 1.04f64.sqrt();
                let outer = (16. - 0.04f64).sqrt() * stretch;
                let mut expected = vec![
                    ((-outer + 5.) / (2. * sign), 0),
                    ((outer + 5.) / (2. * sign), 0),
                ];
                for (i, hole) in input.iter().enumerate() {
                    let half = (hole.radius.powi(2) - 0.04).sqrt() * stretch;
                    for x in [
                        hole.center[0] * stretch - half,
                        hole.center[0] * stretch + half,
                    ] {
                        expected.push(((x + 5.) / (2. * sign), i + 1));
                    }
                }
                expected.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (event, (parameter, wire)) in clip.events.iter().zip(expected) {
                    assert!((event.parameter - parameter).abs() < 1e-12);
                    assert_eq!(event.wire, wire);
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
        for (y, intervals) in [(0.45, 3), (0.55, 2), (1., 1), (5., 0)] {
            assert_eq!(
                clip_line_to_planar_face(&source, 9, origin - u * 5. + v * y, u, t)
                    .unwrap()
                    .intervals
                    .len(),
                intervals
            );
        }
    }
}

#[test]
fn overlapping_touching_nested_and_excessive_holes_are_rejected() {
    let t = GeometryTolerance::default();
    for distance in [0., 0.5, 1., 1. + 1e-9] {
        let input = [
            EllipseCapHole {
                radius: 0.5,
                center: [0., 0.],
            },
            EllipseCapHole {
                radius: 0.5,
                center: [distance, 0.],
            },
        ];
        assert!(ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &input, t).is_err());
    }
    let nested = [
        EllipseCapHole {
            radius: 1.,
            center: [0., 0.],
        },
        EllipseCapHole {
            radius: 0.2,
            center: [0.1, 0.],
        },
    ];
    assert!(ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &nested, t).is_err());
    let s = ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &holes(), t).unwrap();
    let mut malformed = s.clone();
    malformed.shell.faces[9].wires[2] = malformed.shell.faces[9].wires[1].clone();
    assert!(malformed.validate(t.absolute()).is_err());
    assert!(clip_line_to_planar_face(
        &malformed,
        9,
        Point3::new(0., 100., 0.),
        Vec3::new(1., 0., -0.2),
        t
    )
    .is_err());
    let many = (0..16)
        .map(|i| EllipseCapHole {
            radius: 0.5,
            center: [(i % 4) as f64 * 4. - 6., (i / 4) as f64 * 4. - 6.],
        })
        .collect::<Vec<_>>();
    let s = ellipse_multi_hole_planar_demo_solid(12., 6., 0.1, &many, t).unwrap();
    assert_eq!(s.shell.faces.last().unwrap().wires.len(), 17);
    assert!((s.volume().unwrap() - PI * (144. - 4.) * 3.).abs() < 1e-10);
    let mut excessive = many;
    excessive.push(excessive[0]);
    assert!(ellipse_multi_hole_planar_demo_solid(12., 6., 0.1, &excessive, t).is_err());
    assert!(ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &[], t).is_ok());
}

#[test]
fn a_polygon_face_intersects_three_hole_cap_in_four_exact_segments() {
    let t = GeometryTolerance::default();
    let s = ellipse_multi_hole_planar_demo_solid(4., 6., 0.2, &holes(), t).unwrap();
    let polygon = make_box(
        BoxSpec {
            min: Point3::new(-5., 0.2, -3.),
            size: Vec3::new(10., 1., 6.),
        },
        t.absolute(),
    )
    .unwrap();
    let index = polygon
        .shell
        .faces
        .iter()
        .position(|face| {
            let Surface::Plane { origin, u, v } = face.surface else {
                return false;
            };
            origin.y == 0.2 && u.cross(v).y.abs() > 0.9
        })
        .unwrap();
    let PlanarFacesIntersection::Segments(segments) =
        intersect_planar_faces(&s, 9, &polygon, index, t).unwrap()
    else {
        panic!("segments")
    };
    assert_eq!(segments.len(), 4);
    for segment in segments {
        for parameter in [0., 0.5, 1.] {
            let p = segment.curve.evaluate(parameter);
            let uv = segment.first.evaluate(parameter);
            assert!((p.y - 0.2).abs() < 1e-12 && (p.z + 0.2 * p.x).abs() < 1e-12);
            assert!((s.shell.faces[9].surface.evaluate(uv[0], uv[1]) - p).norm() < 1e-12);
            for hole in holes() {
                assert!((p.x - hole.center[0]).hypot(p.y - hole.center[1]) >= hole.radius - 1e-12);
            }
        }
    }
}
