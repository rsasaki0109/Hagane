use hagane::*;
use std::f64::consts::{FRAC_PI_2, PI};
fn tol() -> Tolerance {
    Tolerance::default()
}
fn rounded() -> ArcLineProfile {
    rounded_rectangle_profile(Point3::new(0.0, 0.0, -1.0), 8.0, 6.0, 1.0, tol()).unwrap()
}
fn capsule(length: f64, r: f64) -> ArcLineProfile {
    ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: vec![
            PlanarSegment::Line {
                a: [-length * 0.5, -r],
                b: [length * 0.5, -r],
            },
            PlanarSegment::Arc {
                center: [length * 0.5, 0.0],
                radius: r,
                start_angle: -FRAC_PI_2,
                sweep: PI,
            },
            PlanarSegment::Line {
                a: [length * 0.5, r],
                b: [-length * 0.5, r],
            },
            PlanarSegment::Arc {
                center: [-length * 0.5, 0.0],
                radius: r,
                start_angle: FRAC_PI_2,
                sweep: PI,
            },
        ],
    }
}
#[test]
fn rounded_rectangle_has_exact_analytic_volume_bounds_and_shared_arcs() {
    for r in [0.1, 1.0, 2.5] {
        let profile =
            rounded_rectangle_profile(Point3::new(2.0, -3.0, 4.0), 8.0, 6.0, r, tol()).unwrap();
        let s = extrude_arc_line(&profile, 2.0, tol()).unwrap();
        s.validate(tol()).unwrap();
        assert!((s.volume().unwrap() - (48.0 - (4.0 - PI) * r * r) * 2.0).abs() < 1e-11);
        assert_eq!(
            (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
            (16, 24, 10)
        );
        assert_eq!(
            s.edges
                .iter()
                .filter(|e| matches!(e.curve, Curve::Arc { .. }))
                .count(),
            8
        );
        assert_eq!(
            s.shell
                .faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::FramedCylinder { .. }))
                .count(),
            4
        );
        assert!((s.bounds().min - Point3::new(-2.0, -6.0, 4.0)).norm() < 1e-12);
        assert!((s.bounds().max - Point3::new(6.0, 0.0, 6.0)).norm() < 1e-12);
    }
}
#[test]
fn capsule_and_arc_only_disk_use_partial_cylinders() {
    let s = extrude_arc_line(&capsule(6.0, 2.0), 3.0, tol()).unwrap();
    assert!((s.volume().unwrap() - (24.0 + 4.0 * PI) * 3.0).abs() < 1e-11);
    s.validate(tol()).unwrap();
    let disk = ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: (0..4)
            .map(|i| PlanarSegment::Arc {
                center: [0.0, 0.0],
                radius: 2.0,
                start_angle: i as f64 * FRAC_PI_2,
                sweep: FRAC_PI_2,
            })
            .collect(),
    };
    let s = extrude_arc_line(&disk, 3.0, tol()).unwrap();
    assert!((s.volume().unwrap() - PI * 12.0).abs() < 1e-11);
    s.validate(tol()).unwrap();
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (8, 12, 6)
    );
}
#[test]
fn partial_arc_bounds_include_only_its_angular_extrema() {
    let frame = Frame3::new(
        Vec3::new(5.0, 7.0, 2.0),
        [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        tol(),
    )
    .unwrap();
    let curve = Curve::Arc {
        frame,
        radius: 3.0,
        sweep: FRAC_PI_2,
    };
    assert_eq!(curve.range(), [0.0, FRAC_PI_2]);
    assert!((curve.evaluate(0.0) - Point3::new(5.0, 10.0, 2.0)).norm() < 1e-12);
    let s = Solid {
        vertices: vec![
            Vertex {
                point: curve.evaluate(0.0),
            },
            Vertex {
                point: curve.evaluate(FRAC_PI_2),
            },
        ],
        edges: vec![Edge {
            vertices: [0, 1],
            curve,
        }],
        shell: Shell { faces: vec![] },
    };
    assert!((s.bounds().min - Point3::new(2.0, 7.0, 2.0)).norm() < 1e-12);
    assert!((s.bounds().max - Point3::new(5.0, 10.0, 2.0)).norm() < 1e-12);
}
#[test]
fn partial_face_volume_is_invariant_under_rigid_placement() {
    for profile in [rounded(), capsule(6.0, 2.0)] {
        let s = extrude_arc_line(&profile, 3.0, tol()).unwrap();
        for angle in [0.3, FRAC_PI_2, PI, -2.0] {
            let t = Transform::translation(Vec3::new(1000.0, -30.0, 7.0))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle).unwrap())
                .unwrap();
            let moved = s.transformed(t, tol()).unwrap();
            moved.validate(tol()).unwrap();
            let bounds = moved.bounds();
            for edge in &moved.edges {
                let [lo, hi] = edge.curve.range();
                for k in 0..=128 {
                    let p = edge.curve.evaluate(lo + (hi - lo) * k as f64 / 128.0);
                    assert!(
                        p.x >= bounds.min.x - 1e-10
                            && p.x <= bounds.max.x + 1e-10
                            && p.y >= bounds.min.y - 1e-10
                            && p.y <= bounds.max.y + 1e-10
                            && p.z >= bounds.min.z - 1e-10
                            && p.z <= bounds.max.z + 1e-10
                    );
                }
            }
            assert!((s.volume().unwrap() - moved.volume().unwrap()).abs() < 1e-9);
            for (a, b) in s.edges.iter().zip(&moved.edges) {
                for k in 0..=16 {
                    let [lo, hi] = a.curve.range();
                    let u = lo + (hi - lo) * k as f64 / 16.0;
                    assert!((t.point(a.curve.evaluate(u)) - b.curve.evaluate(u)).norm() < 1e-10);
                }
            }
        }
    }
}
fn verify_mesh(s: &Solid, error: f64) {
    let mesh = s.tessellate(error, tol()).unwrap();
    let key = |p: Point3| {
        (
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        )
    };
    let mut uses = std::collections::BTreeMap::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        assert!(n.norm() > 0.0);
        assert!(n.dot(mesh.normals[tri[0]]) > 0.0);
        for j in 0..3 {
            let a = key(p[j]);
            let b = key(p[(j + 1) % 3]);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(edge).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
            if let Surface::FramedCylinder { frame, radius, .. } = s.shell.faces[fi].surface {
                let mid = frame.local_point((p[j] + p[(j + 1) % 3]) * 0.5);
                assert!(radius - mid.x.hypot(mid.y) <= error + 1e-10);
            }
        }
    }
    assert!(uses.values().all(|&(count, sign)| count == 2 && sign == 0));
    assert!(mesh.signed_volume() > 0.0);
}
#[test]
fn tessellation_is_closed_oriented_and_respects_arc_sagitta() {
    let s = extrude_arc_line(&rounded(), 2.0, tol()).unwrap();
    let mut previous = f64::INFINITY;
    for error in [0.1, 0.01, 0.001] {
        verify_mesh(&s, error);
        let deviation = s.volume().unwrap() - s.tessellate(error, tol()).unwrap().signed_volume();
        assert!(deviation > 0.0 && deviation < previous);
        previous = deviation;
    }
    let moved = s
        .transformed(
            Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.8).unwrap(),
            tol(),
        )
        .unwrap();
    verify_mesh(&moved, 0.01);
    verify_mesh(
        &extrude_arc_line(&capsule(6.0, 2.0), 3.0, tol()).unwrap(),
        0.01,
    );
}
#[test]
fn unsupported_or_invalid_arc_profiles_do_not_succeed() {
    for height in [0.0, -1.0, 1e-9, f64::NAN, f64::INFINITY] {
        assert!(extrude_arc_line(&rounded(), height, tol()).is_err());
    }
    for radius in [0.0, -1.0, 3.0, 4.0, 1e-9, f64::NAN, f64::INFINITY] {
        assert!(
            rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 8.0, 6.0, radius, tol()).is_err()
        );
    }
    for (start, sweep, radius) in [
        (0.0, -1.0, 1.0),
        (0.0, PI + 1e-6, 1.0),
        (0.0, 0.0, 1.0),
        (f64::NAN, FRAC_PI_2, 1.0),
        (100.0, FRAC_PI_2, 1.0),
        (0.0, FRAC_PI_2, -1.0),
    ] {
        let mut profile = rounded();
        profile.segments[1] = PlanarSegment::Arc {
            center: [3.0, -2.0],
            radius,
            start_angle: start,
            sweep,
        };
        assert!(extrude_arc_line(&profile, 2.0, tol()).is_err());
    }
    let mut open = rounded();
    open.segments.pop();
    assert!(extrude_arc_line(&open, 2.0, tol()).is_err());
    let mut non_tangent = rounded();
    non_tangent.segments[0] = PlanarSegment::Line {
        a: [-3.0, -2.0],
        b: [3.0, -3.0],
    };
    assert!(extrude_arc_line(&non_tangent, 2.0, tol()).is_err());
    let mut twice = rounded();
    twice.segments.extend(twice.segments.clone());
    assert!(extrude_arc_line(&twice, 2.0, tol()).is_err());
    let triangle = ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: vec![
            PlanarSegment::Line {
                a: [0.0, 0.0],
                b: [1.0, 0.0],
            },
            PlanarSegment::Line {
                a: [1.0, 0.0],
                b: [0.0, 1.0],
            },
            PlanarSegment::Line {
                a: [0.0, 1.0],
                b: [0.0, 0.0],
            },
        ],
    };
    assert!(matches!(
        extrude_arc_line(&triangle, 2.0, tol()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn malformed_arc_pcurves_and_partial_trims_are_rejected() {
    let s = extrude_arc_line(&rounded(), 2.0, tol()).unwrap();
    let mut bad = s.clone();
    if let PCurve::Arc { ref mut sweep, .. } = bad.shell.faces[0].wires[0].coedges[1].pcurve {
        *sweep *= 0.5;
    }
    assert!(bad.validate(tol()).is_err());
    let mut bad = s.clone();
    if let PCurve::Arc { ref mut radius, .. } = bad.shell.faces[0].wires[0].coedges[1].pcurve {
        *radius *= 0.5;
    }
    assert!(bad.validate(tol()).is_err());
    let mut bad = s.clone();
    let extra = bad.shell.faces[0].wires[0].clone();
    bad.shell.faces[0].wires.push(extra);
    assert!(bad.validate(tol()).is_err());
    let mut bad = s.clone();
    bad.shell.faces[3].wires[0].coedges[1].pcurve = PCurve::Affine {
        origin: [7.0, 0.0],
        direction: [0.0, 2.0],
    };
    assert!(bad.validate(tol()).is_err());
    let mut bad = s.clone();
    if let Curve::Arc { ref mut sweep, .. } = bad.edges[3].curve {
        *sweep = f64::NAN;
    }
    assert!(bad.validate(tol()).is_err());
}
#[test]
fn microscopic_profiles_require_explicit_tolerance_and_far_origins_fail() {
    let t = Tolerance::new(1e-14).unwrap();
    let profile = rounded_rectangle_profile(Vec3::new(0.0, 0.0, 0.0), 8e-6, 6e-6, 1e-6, t).unwrap();
    let s = extrude_arc_line(&profile, 2e-6, t).unwrap();
    assert!((s.volume().unwrap() - (48.0 - (4.0 - PI)) * 2e-18).abs() < 1e-30);
    s.tessellate(1e-9, t).unwrap();
    let profile =
        rounded_rectangle_profile(Vec3::new(1e30, 1e30, 1e30), 8.0, 6.0, 1.0, tol()).unwrap();
    assert!(extrude_arc_line(&profile, 2.0, tol()).is_err());
}
