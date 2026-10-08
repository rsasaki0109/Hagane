use hagane::*;
use std::f64::consts::PI;

#[test]
fn minor_segment_solids_have_independent_volume_bounds_and_membership() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for sweep in [0.6, PI / 2., 2.5, PI] {
            for slope in [-0.25, 0., 0.25] {
                let radius = 2. * scale;
                let height = 4. * scale;
                let s = ellipse_segment_planar_demo_solid(radius, height, slope, sweep, t).unwrap();
                s.validate(t.absolute()).unwrap();
                let volume = radius.powi(2) * height * (sweep - sweep.sin()) / 4.;
                assert!((s.volume().unwrap() - volume).abs() < scale.powi(3) * 1e-11);
                let chord_y = if sweep == PI {
                    0.
                } else {
                    radius * (sweep / 2.).cos()
                };
                let half_x = radius * (sweep / 2.).sin();
                let bounds = s.bounds();
                for (actual, expected) in [
                    (bounds.min.x, -half_x),
                    (bounds.max.x, half_x),
                    (bounds.min.y, chord_y),
                    (bounds.max.y, radius),
                    (bounds.min.z, -height / 2.),
                    (bounds.max.z, slope.abs() * half_x),
                ] {
                    assert!((actual - expected).abs() < scale * 1e-12);
                }
                for (y, z, expected) in [
                    ((chord_y + radius) / 2., -scale, PointLocation::Inside),
                    ((chord_y + radius) / 2., 0., PointLocation::Boundary),
                    (chord_y, -scale, PointLocation::Boundary),
                    (radius, 0., PointLocation::Boundary),
                    (chord_y - 0.1 * scale, -scale, PointLocation::Outside),
                    ((chord_y + radius) / 2., scale, PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&s, Point3::new(0., y, z), t).unwrap(),
                        expected
                    );
                }
                let mesh = s.tessellate(0.002 * scale, t.absolute()).unwrap();
                assert!((mesh.signed_volume() - volume).abs() < 0.02 * scale.powi(3));
                // Independently sample actual ellipse sections against the display chords.
                // The cap boundary shared with the circular wall lies on z=-slope*x.
                let start = (PI - sweep) / 2.;
                let mut checked = 0;
                for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                    if *face != 1 {
                        continue;
                    }
                    for i in 0..3 {
                        let a = mesh.positions[tri[i]];
                        let b = mesh.positions[tri[(i + 1) % 3]];
                        if (a.z + slope * a.x).abs() > scale * 1e-10
                            || (b.z + slope * b.x).abs() > scale * 1e-10
                        {
                            continue;
                        }
                        let ta = a.y.atan2(a.x);
                        let tb = b.y.atan2(b.x);
                        assert!(ta.min(tb) >= start - 1e-12 && ta.max(tb) <= start + sweep + 1e-12);
                        for fraction in [0.25, 0.5, 0.75] {
                            let angle = ta + (tb - ta) * fraction;
                            let exact = Point3::new(
                                radius * angle.cos(),
                                radius * angle.sin(),
                                -slope * radius * angle.cos(),
                            );
                            let chord = a + (b - a) * fraction;
                            assert!((exact - chord).norm() <= 0.002 * scale * (1. + 1e-10));
                        }
                        checked += 1;
                    }
                }
                assert!(checked > 0);
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
                        let (e, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                        let entry = uses.entry(e).or_insert((0, 0));
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
fn chord_and_arc_clips_preserve_parameters_after_placement_and_reversal() {
    let t = GeometryTolerance::default();
    for sweep in [0.6, PI / 2., 2.5, PI] {
        let source = ellipse_segment_planar_demo_solid(2., 4., 0.25, sweep, t).unwrap();
        let chord_y = if sweep == PI {
            0.
        } else {
            2. * (sweep / 2.).cos()
        };
        for angle in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
            let s = source.transformed(transform, t.absolute()).unwrap();
            for sign in [-1., 1.] {
                let clip = clip_line_to_planar_face(
                    &s,
                    3,
                    transform.point(Point3::new(0., -3., 0.)),
                    transform.vector(Vec3::new(0., 2. * sign, 0.)),
                    t,
                )
                .unwrap();
                let expected = if sign > 0. {
                    [(3. + chord_y) / 2., 2.5]
                } else {
                    [-2.5, -(3. + chord_y) / 2.]
                };
                assert_eq!(clip.intervals.len(), 1);
                for (event, parameter) in clip.events.iter().zip(expected) {
                    assert!((event.parameter - parameter).abs() < 1e-12);
                    assert!(
                        (s.edges[event.edge].curve.evaluate(event.edge_parameter) - event.point)
                            .norm()
                            < 1e-10
                    );
                }
                let arc = clip.events.iter().find(|e| e.coedge == 0).unwrap();
                let chord = clip.events.iter().find(|e| e.coedge == 1).unwrap();
                assert!((arc.edge_parameter - sweep / 2.).abs() < 1e-12);
                assert!((chord.edge_parameter - 0.5).abs() < 1e-12);
            }
            let x = 0.3 * 2. * (sweep / 2.).sin();
            let clip = clip_line_to_planar_face(
                &s,
                3,
                transform.point(Point3::new(x, -3., -0.25 * x)),
                transform.vector(Vec3::new(0., 2., 0.)),
                t,
            )
            .unwrap();
            let expected = [(3. + chord_y) / 2., (3. + (4. - x * x).sqrt()) / 2.];
            for (event, parameter) in clip.events.iter().zip(expected) {
                assert!((event.parameter - parameter).abs() < 1e-12);
                assert!(
                    (s.edges[event.edge].curve.evaluate(event.edge_parameter) - event.point).norm()
                        < 1e-10
                );
            }
            let chord = clip.events.iter().find(|e| e.coedge == 1).unwrap();
            let Curve::Line { a, b } = source.edges[chord.edge].curve else {
                panic!("chord")
            };
            let expected = if b.x > a.x { 0.65 } else { 0.35 };
            assert!((chord.edge_parameter - expected).abs() < 1e-12);
            let y = (2. + chord_y) / 2.;
            let u = Vec3::new(1., 0., -0.25).normalized().unwrap();
            let clip = clip_line_to_planar_face(
                &s,
                3,
                transform.point(u * (-3.) + Vec3::new(0., y, 0.)),
                transform.vector(u * 2.),
                t,
            )
            .unwrap();
            let half = (4. - y * y).sqrt() * 1.0625f64.sqrt();
            for (event, parameter) in clip.events.iter().zip([(3. - half) / 2., (3. + half) / 2.]) {
                assert!((event.parameter - parameter).abs() < 1e-12);
                assert_eq!(event.coedge, 0);
            }
        }
    }
}

#[test]
fn contacts_tiny_segments_and_invalid_inputs_return_errors() {
    let t = GeometryTolerance::default();
    let sweep = PI / 2.;
    let s = ellipse_segment_planar_demo_solid(2., 4., 0.25, sweep, t).unwrap();
    let u = Vec3::new(1., 0., -0.25).normalized().unwrap();
    let chord_y = 2. * (sweep / 2.).cos();
    for y in [chord_y, chord_y + 1e-9, 2., 2. + 1e-9] {
        assert!(clip_line_to_planar_face(&s, 3, u * (-3.) + Vec3::new(0., y, 0.), u, t).is_err());
    }
    assert!(
        clip_line_to_planar_face(&s, 3, u * (-3.) + Vec3::new(0., chord_y - 0.1, 0.), u, t)
            .unwrap()
            .events
            .is_empty()
    );
    let endpoint = Point3::new(
        2. * (sweep / 2.).sin(),
        chord_y,
        -0.25 * 2. * (sweep / 2.).sin(),
    );
    assert!(clip_line_to_planar_face(&s, 3, endpoint, Vec3::new(0., 1., 0.), t).is_err());
    for sweep in [0., -0.1, PI + 0.1, 2. * PI, 1e-6, f64::NAN, f64::INFINITY] {
        assert!(ellipse_segment_planar_demo_solid(2., 4., 0.25, sweep, t).is_err());
    }
    assert!(ellipse_segment_planar_demo_json(PI / 2., 3, 20., 0.).is_err());
    assert!(ellipse_segment_planar_demo_json(PI / 2., 0, f64::NAN, 0.).is_err());
    assert!(ellipse_segment_planar_demo_json(PI / 2., 0, 20., f64::INFINITY).is_err());
}
