use hagane::*;
use std::f64::consts::PI;

#[test]
fn oblique_tube_cap_preserves_hole_volume_material_and_closed_mesh() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for slope in [-0.25, 0., 0.25] {
            let s =
                ellipse_annulus_planar_demo_solid(2. * scale, scale, 4. * scale, slope, t).unwrap();
            s.validate(t.absolute()).unwrap();
            assert_eq!(
                (s.shell.faces.len(), s.edges.len(), s.vertices.len()),
                (6, 12, 8)
            );
            assert_eq!(s.shell.faces[5].wires.len(), 2);
            let expected = PI * 6. * scale.powi(3);
            assert!((s.volume().unwrap() - expected).abs() < 1e-11 * scale.powi(3));
            let bounds = s.bounds();
            assert!((bounds.min.x + 2. * scale).abs() < 1e-12 * scale);
            assert!((bounds.max.y - 2. * scale).abs() < 1e-12 * scale);
            assert!((bounds.min.z + 2. * scale).abs() < 1e-12 * scale);
            for angle in [0., 0.37] {
                let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
                let placed = s.transformed(transform, t.absolute()).unwrap();
                for (p, location) in [
                    (Point3::new(0., 0., -1.), PointLocation::Outside),
                    (Point3::new(0., 0., 0.), PointLocation::Outside),
                    (Point3::new(0., 0.5, -1.), PointLocation::Outside),
                    (Point3::new(0., 1.5, -1.), PointLocation::Inside),
                    (Point3::new(0., 1.5, 0.), PointLocation::Boundary),
                    (Point3::new(0., 1., -1.), PointLocation::Boundary),
                    (Point3::new(0., 1., 0.), PointLocation::Boundary),
                    (Point3::new(0., 2., 0.), PointLocation::Boundary),
                    (Point3::new(0., 2.1, -1.), PointLocation::Outside),
                    (Point3::new(0., 1.5, 1.), PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&placed, transform.point(p * scale), t).unwrap(),
                        location
                    );
                }
            }
            let mesh = s.tessellate(0.002 * scale, t.absolute()).unwrap();
            assert!((mesh.signed_volume() - expected).abs() < 0.05 * scale.powi(3));
            let mut checked_chords = 0;
            for (triangle, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                let points = triangle.map(|i| mesh.positions[i]);
                if *face == 5 {
                    let centroid = (points[0] + points[1] + points[2]) * (1. / 3.);
                    assert!(centroid.x.hypot(centroid.y) >= scale * (1. - 0.002));
                }
                if !(1..=4).contains(face) {
                    continue;
                }
                let radius = if *face <= 2 { 2. * scale } else { scale };
                for i in 0..3 {
                    let a = points[i];
                    let b = points[(i + 1) % 3];
                    if (a.z + slope * a.x).abs() > 1e-10 * scale
                        || (b.z + slope * b.x).abs() > 1e-10 * scale
                    {
                        continue;
                    }
                    let start = a.y.atan2(a.x);
                    let mut sweep = b.y.atan2(b.x) - start;
                    if sweep > PI {
                        sweep -= 2. * PI;
                    } else if sweep < -PI {
                        sweep += 2. * PI;
                    }
                    for fraction in [0.25, 0.5, 0.75] {
                        let angle = start + sweep * fraction;
                        let exact = Point3::new(
                            radius * angle.cos(),
                            radius * angle.sin(),
                            -slope * radius * angle.cos(),
                        );
                        assert!(
                            (exact - (a + (b - a) * fraction)).norm()
                                <= 0.002 * scale * (1. + 1e-10)
                        );
                    }
                    checked_chords += 1;
                }
            }
            assert!(checked_chords >= 4);
            let key = |p: Point3| {
                [
                    (p.x / scale * 1e9).round() as i64,
                    (p.y / scale * 1e9).round() as i64,
                    (p.z / scale * 1e9).round() as i64,
                ]
            };
            let mut uses = std::collections::BTreeMap::new();
            for tri in &mesh.triangles {
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

#[test]
fn annular_line_clipping_returns_two_material_intervals_and_hole_provenance() {
    let t = GeometryTolerance::default();
    let source = ellipse_annulus_planar_demo_solid(2., 1., 4., 0.25, t).unwrap();
    let Surface::Plane { origin, u, v } = source.shell.faces[5].surface else {
        panic!("cap")
    };
    for angle in [0., 0.37] {
        let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
        let s = source.transformed(transform, t.absolute()).unwrap();
        for offset in [-0.5, 0.5, 1.5, 3.] {
            for sign in [-1., 1.] {
                let clip = clip_line_to_planar_face(
                    &s,
                    5,
                    transform.point(origin - u * 3. + v * offset),
                    transform.vector(u * (2. * sign)),
                    t,
                )
                .unwrap();
                if offset == 3. {
                    assert!(clip.events.is_empty());
                    continue;
                }
                let outer = (4. - offset * offset).sqrt() * 1.0625f64.sqrt();
                let mut expected = vec![(3. - outer) / 2., (3. + outer) / 2.];
                if offset.abs() < 1. {
                    let inner = (1. - offset * offset).sqrt() * 1.0625f64.sqrt();
                    expected = vec![
                        (3. - outer) / 2.,
                        (3. - inner) / 2.,
                        (3. + inner) / 2.,
                        (3. + outer) / 2.,
                    ];
                }
                expected = expected.into_iter().map(|x| x / sign).collect();
                expected.sort_by(f64::total_cmp);
                assert_eq!(clip.events.len(), expected.len());
                assert_eq!(clip.intervals.len(), expected.len() / 2);
                for (event, parameter) in clip.events.iter().zip(expected) {
                    assert!((event.parameter - parameter).abs() < 1e-12);
                    assert!(
                        (s.edges[event.edge].curve.evaluate(event.edge_parameter) - event.point)
                            .norm()
                            < 1e-10
                    );
                    let PCurve::EllipseArc {
                        cosine,
                        sine,
                        center,
                        ..
                    } = s.shell.faces[5].wires[event.wire].coedges[event.coedge].pcurve
                    else {
                        panic!("ellipse")
                    };
                    let uv = s.shell.faces[5].surface.parameters(event.point);
                    let angle = ((uv[1] - center[1]) / sine[1])
                        .atan2((uv[0] - center[0]) / cosine[0])
                        .rem_euclid(2. * PI);
                    assert!((event.edge_parameter - angle).abs() < 1e-12);
                }
                if offset.abs() < 1. {
                    assert_eq!(
                        clip.events.iter().map(|e| e.wire).collect::<Vec<_>>(),
                        [0, 1, 1, 0]
                    );
                }
                for interval in clip.intervals {
                    let midpoint = transform.point(origin - u * 3. + v * offset)
                        + transform.vector(u * (2. * sign))
                            * ((interval.parameter_range[0] + interval.parameter_range[1]) / 2.);
                    assert_eq!(
                        classify_point_in_solid(&s, midpoint, t).unwrap(),
                        PointLocation::Boundary
                    );
                }
            }
        }
    }
}

#[test]
fn unsupported_holes_and_contacts_fail_before_empty_shortcuts() {
    let t = GeometryTolerance::default();
    let source = ellipse_annulus_planar_demo_solid(2., 1., 4., 0.25, t).unwrap();
    let Surface::Plane { origin, u, v } = source.shell.faces[5].surface else {
        panic!("cap")
    };
    for y in [0., 1., 1. + 1e-9, 2., 2. + 1e-9] {
        assert!(clip_line_to_planar_face(&source, 5, origin - u * 3. + v * y, u, t).is_err());
    }
    for variant in 0..4 {
        let mut s = source.clone();
        let cap = &mut s.shell.faces[5];
        match variant {
            0 => cap.wires.push(cap.wires[1].clone()),
            1 => cap.wires[1] = cap.wires[0].clone(),
            _ => {
                for c in &mut cap.wires[1].coedges {
                    let PCurve::EllipseArc { center, cosine, .. } = &mut c.pcurve else {
                        panic!("ellipse")
                    };
                    if variant == 2 {
                        center[0] += 1e-9;
                    } else {
                        cosine[0] *= 1. + 1e-9;
                    }
                }
            }
        }
        assert!(s.validate(t.absolute()).is_err());
        assert!(clip_line_to_planar_face(&s, 5, origin + v * 100., u, t).is_err());
    }
    for inner in [0., -1., 2., 2. - 1e-9, 3., f64::NAN, f64::INFINITY] {
        assert!(ellipse_annulus_planar_demo_solid(2., inner, 4., 0.25, t).is_err());
    }
    assert!(ellipse_annulus_planar_demo_solid(2., 1., 4., 1., t).is_err());
}

#[test]
fn polygon_and_ellipse_annulus_faces_share_two_exact_intersection_segments() {
    let t = GeometryTolerance::default();
    let annulus = ellipse_annulus_planar_demo_solid(2., 1., 4., 0.25, t).unwrap();
    let polygon = make_box(
        BoxSpec {
            min: Point3::new(0., -3., -3.),
            size: Vec3::new(1., 6., 6.),
        },
        t.absolute(),
    )
    .unwrap();
    let face = polygon
        .shell
        .faces
        .iter()
        .position(|f| {
            let Surface::Plane { origin, u, v } = f.surface else {
                return false;
            };
            origin.x == 0. && u.cross(v).x.abs() > 0.9
        })
        .unwrap();
    let PlanarFacesIntersection::Segments(segments) =
        intersect_planar_faces(&annulus, 5, &polygon, face, t).unwrap()
    else {
        panic!("segments")
    };
    assert_eq!(segments.len(), 2);
    for segment in segments {
        for parameter in [0., 0.5, 1.] {
            let point = segment.curve.evaluate(parameter);
            assert!(point.x.abs() < 1e-12 && point.z.abs() < 1e-12);
            assert!((1. - 1e-12..=2. + 1e-12).contains(&point.y.abs()));
            let uv = segment.first.evaluate(parameter);
            assert!((annulus.shell.faces[5].surface.evaluate(uv[0], uv[1]) - point).norm() < 1e-12);
        }
    }
}
