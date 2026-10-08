use hagane::*;
use std::f64::consts::PI;
fn tol() -> Tolerance {
    Tolerance::default()
}
fn circle(center: [f64; 2], radius: f64) -> Vec<PlanarSegment> {
    vec![
        PlanarSegment::Arc {
            center,
            radius,
            start_angle: 0.0,
            sweep: PI,
        },
        PlanarSegment::Arc {
            center,
            radius,
            start_angle: PI,
            sweep: PI,
        },
    ]
}
fn reverse(ring: &[PlanarSegment]) -> Vec<PlanarSegment> {
    ring.iter().rev().map(|s| s.reversed()).collect()
}
fn rectangle(x: f64, y: f64) -> Vec<PlanarSegment> {
    let points = [[-x, -y], [x, -y], [x, y], [-x, y]];
    (0..4)
        .map(|i| PlanarSegment::Line {
            a: points[i],
            b: points[(i + 1) % 4],
        })
        .collect()
}
fn region(outer: Vec<PlanarSegment>, holes: Vec<Vec<PlanarSegment>>) -> ArcLineRegion {
    ArcLineRegion {
        origin: Point3::new(0.0, 0.0, 0.0),
        outer,
        holes,
    }
}
fn notched(radius: f64) -> Vec<PlanarSegment> {
    let line = |a, b| PlanarSegment::Line { a, b };
    vec![
        line([-4.0, -3.0], [4.0, -3.0]),
        line([4.0, -3.0], [4.0, 3.0]),
        line([4.0, 3.0], [radius, 3.0]),
        PlanarSegment::Arc {
            center: [0.0, 3.0],
            radius,
            start_angle: 0.0,
            sweep: -PI,
        },
        line([-radius, 3.0], [-4.0, 3.0]),
        line([-4.0, 3.0], [-4.0, -3.0]),
    ]
}
fn mesh_closed(s: &Solid, error: f64) {
    let m = s.tessellate(error, tol()).unwrap();
    let key = |p: Point3| {
        (
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        )
    };
    let mut uses = std::collections::BTreeMap::new();
    for (tri, &face) in m.triangles.iter().zip(&m.face_ids) {
        let p = tri.map(|i| m.positions[i]);
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        assert!(cross.norm() > 0.0 && cross.dot(m.normals[tri[0]]) > 0.0);
        for i in 0..3 {
            let a = key(p[i]);
            let b = key(p[(i + 1) % 3]);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(edge).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
            if let Surface::FramedCylinder { frame, radius, .. } = s.shell.faces[face].surface {
                let q = frame.local_point((p[i] + p[(i + 1) % 3]) * 0.5);
                assert!(radius - q.x.hypot(q.y) <= error + 1e-10);
            }
        }
    }
    assert!(
        uses.values().all(|&(count, sign)| count == 2 && sign == 0),
        "bad edges: {:?}",
        uses.iter()
            .filter(|(_, u)| u.0 != 2 || u.1 != 0)
            .collect::<Vec<_>>()
    );
    assert!(m.signed_volume() > 0.0);
}
#[test]
fn clockwise_notch_and_sharp_line_arc_joins_have_exact_volume() {
    let r = region(notched(2.0), vec![]);
    let s = extrude_arc_line_region(&r, 2.0, tol()).unwrap();
    s.validate(tol()).unwrap();
    assert!((s.volume().unwrap() - (48.0 - 2.0 * PI) * 2.0).abs() < 1e-11);
    assert!(s
        .shell
        .faces
        .iter()
        .any(|f| matches!(f.surface, Surface::FramedCylinder { .. }) && f.orientation == -1));
    mesh_closed(&s, 0.01);
    let d = ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: vec![
            PlanarSegment::Line {
                a: [-1.0, 0.0],
                b: [1.0, 0.0],
            },
            PlanarSegment::Arc {
                center: [0.0, 0.0],
                radius: 1.0,
                start_angle: 0.0,
                sweep: PI,
            },
        ],
    };
    assert!((extrude_arc_line(&d, 2.0, tol()).unwrap().volume().unwrap() - PI).abs() < 1e-12);
}
#[test]
fn curved_and_polygon_holes_normalize_both_windings() {
    let outer = rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, tol())
        .unwrap()
        .segments;
    let holes = [
        circle([-2.0, 0.0], 1.0),
        rectangle(0.5, 1.0)
            .into_iter()
            .map(|s| match s {
                PlanarSegment::Line { mut a, mut b } => {
                    a[0] += 2.0;
                    b[0] += 2.0;
                    PlanarSegment::Line { a, b }
                }
                _ => unreachable!(),
            })
            .collect(),
    ];
    let expected = (120.0 - (4.0 - PI) - PI - 2.0) * 3.0;
    for reverse_outer in [false, true] {
        for reverse_holes in [false, true] {
            let r = region(
                if reverse_outer {
                    reverse(&outer)
                } else {
                    outer.clone()
                },
                holes
                    .iter()
                    .map(|h| if reverse_holes { reverse(h) } else { h.clone() })
                    .collect(),
            );
            let s = extrude_arc_line_region(&r, 3.0, tol()).unwrap();
            s.validate(tol()).unwrap();
            assert!((s.volume().unwrap() - expected).abs() < 1e-10);
            assert_eq!(s.shell.faces[0].wires.len(), 3);
            mesh_closed(&s, 0.01);
            let moved = s
                .transformed(
                    Transform::translation(Vec3::new(100.0, -20.0, 10.0))
                        .unwrap()
                        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.7).unwrap())
                        .unwrap(),
                    tol(),
                )
                .unwrap();
            assert!((moved.volume().unwrap() - expected).abs() < 1e-9);
            mesh_closed(&moved, 0.01);
        }
    }
}
#[test]
fn annular_arc_region_has_inward_normals_and_volume_convergence() {
    let s = extrude_arc_line_region(
        &region(circle([0.0, 0.0], 5.0), vec![circle([0.0, 0.0], 2.0)]),
        2.0,
        tol(),
    )
    .unwrap();
    assert!((s.volume().unwrap() - 42.0 * PI).abs() < 1e-11);
    assert_eq!(
        s.shell
            .faces
            .iter()
            .filter(|f| f.orientation == -1 && matches!(f.surface, Surface::FramedCylinder { .. }))
            .count(),
        2
    );
    let mut prev = f64::INFINITY;
    for error in [0.1, 0.01, 0.001] {
        mesh_closed(&s, error);
        let delta =
            (s.volume().unwrap() - s.tessellate(error, tol()).unwrap().signed_volume()).abs();
        assert!(delta < prev);
        prev = delta;
    }
}
#[test]
fn analytic_ray_classification_handles_extrema_endpoints_and_winding() {
    for ring in [circle([0.0, 0.0], 2.0), reverse(&circle([0.0, 0.0], 2.0))] {
        for i in -5..=5 {
            for j in -5..=5 {
                let p = [i as f64 * 0.5, j as f64 * 0.5];
                let d = p[0].hypot(p[1]);
                let expected = if (d - 2.0).abs() <= tol().linear {
                    PointLocation::Boundary
                } else if d < 2.0 {
                    PointLocation::Inside
                } else {
                    PointLocation::Outside
                };
                assert_eq!(
                    classify_arc_line_point(p, &ring, tol()).unwrap(),
                    expected,
                    "point {p:?}, ring {ring:?}"
                );
            }
        }
    }
    let ring = notched(2.0);
    assert_eq!(
        classify_arc_line_point([0.0, 0.0], &ring, tol()).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        classify_arc_line_point([0.0, 2.0], &ring, tol()).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_arc_line_point([0.0, 1.0], &ring, tol()).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        classify_arc_line_point([3.0, 3.0], &ring, tol()).unwrap(),
        PointLocation::Boundary
    );
}
#[test]
fn holes_crossing_or_outside_a_concave_boundary_are_rejected() {
    for hole in [
        circle([0.0, 2.0], 0.2),
        circle([0.0, 0.0], 1.5),
        circle([8.0, 0.0], 1.0),
    ] {
        assert!(extrude_arc_line_region(&region(notched(2.0), vec![hole]), 1.0, tol()).is_err());
    }
    let s = extrude_arc_line_region(
        &region(notched(2.0), vec![circle([0.0, -1.5], 0.5)]),
        2.0,
        tol(),
    )
    .unwrap();
    assert!((s.volume().unwrap() - (48.0 - 2.0 * PI - PI * 0.25) * 2.0).abs() < 1e-11);
    mesh_closed(&s, 0.01);
}
#[test]
fn hole_contact_near_contact_overlap_and_nesting_are_rejected() {
    let t = Tolerance::new(1e-6).unwrap();
    for gap in [0.0, 5e-7, 1e-6] {
        assert!(
            extrude_arc_line_region(
                &region(circle([0.0, 0.0], 5.0), vec![circle([4.0 - gap, 0.0], 1.0)]),
                2.0,
                t
            )
            .is_err(),
            "gap {gap}"
        );
    }
    assert!(extrude_arc_line_region(
        &region(
            circle([0.0, 0.0], 5.0),
            vec![circle([4.0 - 2e-6, 0.0], 1.0)]
        ),
        2.0,
        t
    )
    .is_ok());
    for holes in [
        vec![circle([0.0, 0.0], 2.0), circle([0.0, 0.0], 1.0)],
        vec![circle([-1.0, 0.0], 1.0), circle([1.0, 0.0], 1.0)],
        vec![circle([-0.5, 0.0], 1.0), circle([0.5, 0.0], 1.0)],
    ] {
        assert!(extrude_arc_line_region(&region(rectangle(5.0, 5.0), holes), 1.0, tol()).is_err());
    }
}
#[test]
fn adjacent_secondary_intersections_and_overlaps_are_not_ignored() {
    let invalid = vec![
        PlanarSegment::Line {
            a: [-2.0, 0.0],
            b: [1.0, 0.0],
        },
        PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 1.0,
            start_angle: 0.0,
            sweep: PI,
        },
        PlanarSegment::Line {
            a: [-1.0, 0.0],
            b: [-2.0, 0.0],
        },
    ];
    assert!(extrude_arc_line_region(&region(invalid, vec![]), 1.0, tol()).is_err());
    let duplicate = vec![
        PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 1.0,
            start_angle: 0.0,
            sweep: PI,
        },
        PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 1.0,
            start_angle: PI,
            sweep: -PI,
        },
    ];
    assert!(extrude_arc_line_region(&region(duplicate, vec![]), 1.0, tol()).is_err());
    let r = 5.0f64.sqrt();
    let start = (-1.0f64).atan2(-2.0);
    let end = (-1.0f64).atan2(2.0);
    let lens = region(
        vec![
            PlanarSegment::Arc {
                center: [0.0, 0.0],
                radius: 2.0,
                start_angle: 0.0,
                sweep: PI,
            },
            PlanarSegment::Arc {
                center: [0.0, 1.0],
                radius: r,
                start_angle: start,
                sweep: end - start,
            },
        ],
        vec![],
    );
    let s = extrude_arc_line_region(&lens, 1.0, tol()).unwrap();
    s.validate(tol()).unwrap();
    mesh_closed(&s, 0.01);
}
#[test]
fn coarse_display_cannot_move_a_hole_outside_its_exact_boundary() {
    let angle = PI / 12.0;
    let profile = region(
        circle([0.0, 0.0], 5.0),
        vec![circle([3.999 * angle.cos(), 3.999 * angle.sin()], 1.0)],
    );
    let s = extrude_arc_line_region(&profile, 1.0, tol()).unwrap();
    s.validate(tol()).unwrap();
    assert!(matches!(
        s.tessellate(0.5, tol()),
        Err(Error::Tessellation(_))
    ));
    mesh_closed(&s, 0.0001);
}
#[test]
fn microscopic_holes_and_invalid_inputs_have_checked_outcomes() {
    let t = Tolerance::new(1e-14).unwrap();
    let s = extrude_arc_line_region(
        &region(circle([0.0, 0.0], 5e-6), vec![circle([0.0, 0.0], 2e-6)]),
        2e-6,
        t,
    )
    .unwrap();
    assert!((s.volume().unwrap() - 42.0 * PI * 1e-18).abs() < 1e-29);
    s.tessellate(1e-9, t).unwrap();
    let mut invalid = region(circle([0.0, 0.0], 5.0), vec![]);
    invalid.origin.x = f64::NAN;
    assert!(extrude_arc_line_region(&invalid, 1.0, tol()).is_err());
    assert!(extrude_arc_line_region(&region(Vec::new(), vec![]), 1.0, tol()).is_err());
    assert!(
        extrude_arc_line_region(&region(circle([0.0, 0.0], 5.0), vec![vec![]]), 1.0, tol())
            .is_err()
    );
    assert!(classify_arc_line_point([f64::NAN, 0.0], &circle([0.0, 0.0], 1.0), tol()).is_err());
}

#[test]
fn collinear_bridges_between_polygon_holes_keep_all_boundary_vertices() {
    let shifted = |x: f64| {
        rectangle(0.5, 1.0)
            .into_iter()
            .map(|s| match s {
                PlanarSegment::Line { mut a, mut b } => {
                    a[0] += x;
                    b[0] += x;
                    PlanarSegment::Line { a, b }
                }
                _ => unreachable!(),
            })
            .collect()
    };
    let s = extrude_arc_line_region(
        &region(rectangle(6.0, 5.0), vec![shifted(-2.0), shifted(2.0)]),
        3.0,
        tol(),
    )
    .unwrap();
    assert!((s.volume().unwrap() - (120.0 - 4.0) * 3.0).abs() < 1e-10);
    mesh_closed(&s, 0.01);
}
#[test]
fn excessive_mixed_trim_sampling_returns_an_explicit_error() {
    let s = extrude_arc_line_region(&region(circle([0.0, 0.0], 5.0), vec![]), 2.0, tol()).unwrap();
    assert!(matches!(
        s.tessellate(1e-8, tol()),
        Err(Error::Tessellation(_))
    ));
    assert!(s.tessellate(0.01, tol()).is_ok());
}

#[test]
fn analytic_rays_avoid_gaps_allowed_by_endpoint_tolerance() {
    let ring = vec![
        PlanarSegment::Line {
            a: [-2.0, 0.0],
            b: [2.0, 0.0],
        },
        PlanarSegment::Line {
            a: [2.0, 0.8 * tol().linear],
            b: [0.0, 2.0],
        },
        PlanarSegment::Line {
            a: [0.0, 2.0],
            b: [-2.0, 0.0],
        },
    ];
    for ring in [ring.clone(), reverse(&ring)] {
        assert_eq!(
            classify_arc_line_point([-3.0, 0.4 * tol().linear], &ring, tol()).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            classify_arc_line_point([0.0, 0.5], &ring, tol()).unwrap(),
            PointLocation::Inside
        );
    }
}
