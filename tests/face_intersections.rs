use hagane::*;
use std::f64::consts::PI;
fn t() -> GeometryTolerance {
    GeometryTolerance::default()
}
fn block() -> Solid {
    subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        CylinderSpec {
            base: Point3::new(0.0, 0.0, -1.0),
            radius: 1.0,
            height: 4.0,
        },
        t().absolute(),
    )
    .unwrap()
}
fn clip(s: &Solid, a: Point3, d: Vec3) -> PlanarLineClip {
    clip_line_to_planar_face(s, 0, a, d, t()).unwrap()
}
fn check_ranges(c: &PlanarLineClip, expected: &[[f64; 2]]) {
    assert_eq!(c.intervals.len(), expected.len());
    for (i, r) in c.intervals.iter().zip(expected) {
        for k in 0..2 {
            assert!(
                (i.parameter_range[k] - r[k]).abs() < 1e-10,
                "{:?} != {r:?}",
                i.parameter_range
            );
        }
    }
}
fn circle(r: f64) -> Vec<PlanarSegment> {
    [0.0, PI]
        .map(|start_angle| PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: r,
            start_angle,
            sweep: PI,
        })
        .to_vec()
}
#[test]
fn box_cap_with_exact_circle_hole_returns_provenanced_intervals() {
    let s = block();
    let c = clip(&s, Point3::new(-10.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0));
    check_ranges(&c, &[[3.0, 4.5], [5.5, 7.0]]);
    assert_eq!(c.events.len(), 4);
    for e in &c.events {
        assert!((s.edges[e.edge].curve.evaluate(e.edge_parameter) - e.point).norm() < 1e-12);
        assert_eq!(
            s.shell.faces[0].wires[e.wire].coedges[e.coedge].edge,
            e.edge
        );
        assert!(
            (Point3::new(-10.0, 0.0, 0.0) + Vec3::new(2.0, 0.0, 0.0) * e.parameter - e.point)
                .norm()
                < 1e-12
        );
    }
    let c = clip(&s, Point3::new(10.0, 0.0, 0.0), Vec3::new(-2.0, 0.0, 0.0));
    check_ranges(&c, &[[3.0, 4.5], [5.5, 7.0]]);
    assert!(
        clip(&s, Point3::new(0.0, 4.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
            .intervals
            .is_empty()
    );
}
#[test]
fn full_circle_and_annular_caps_use_analytic_roots() {
    let s = make_tube(
        TubeSpec {
            base: Point3::new(0.0, 0.0, 0.0),
            outer_radius: 3.0,
            inner_radius: 1.0,
            height: 2.0,
        },
        t().absolute(),
    )
    .unwrap();
    let c = clip(&s, Point3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let a = 8.75_f64.sqrt();
    let b = 0.75_f64.sqrt();
    check_ranges(&c, &[[-a, -b], [b, a]]);
}
#[test]
fn clockwise_notch_and_mixed_holes_keep_exact_arc_parameters() {
    let line = |a, b| PlanarSegment::Line { a, b };
    let ring = vec![
        line([-4.0, -3.0], [4.0, -3.0]),
        line([4.0, -3.0], [4.0, 3.0]),
        line([4.0, 3.0], [2.0, 3.0]),
        PlanarSegment::Arc {
            center: [0.0, 3.0],
            radius: 2.0,
            start_angle: 0.0,
            sweep: -PI,
        },
        line([-2.0, 3.0], [-4.0, 3.0]),
        line([-4.0, 3.0], [-4.0, -3.0]),
    ];
    let s = extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0.0, 0.0, 0.0),
            outer: ring,
            holes: vec![circle(0.5)],
        },
        2.0,
        t().absolute(),
    )
    .unwrap();
    let c = clip(&s, Point3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let a = 3.0_f64.sqrt();
    check_ranges(&c, &[[-4.0, -a], [a, 4.0]]);
    for e in c.events {
        assert!((e.point - s.edges[e.edge].curve.evaluate(e.edge_parameter)).norm() < 1e-12);
    }
    let c = clip(&s, Point3::new(0.0, 0.25, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let h = 0.1875_f64.sqrt();
    check_ranges(&c, &[[-4.0, -h], [h, 4.0]]);
}
#[test]
fn rounded_outer_trim_and_multiple_holes_produce_multiple_intervals() {
    let p = rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, t().absolute())
        .unwrap();
    let moved = |x: f64| {
        circle(1.0)
            .into_iter()
            .map(|s| match s {
                PlanarSegment::Arc {
                    mut center,
                    radius,
                    start_angle,
                    sweep,
                } => {
                    center[0] = x;
                    PlanarSegment::Arc {
                        center,
                        radius,
                        start_angle,
                        sweep,
                    }
                }
                _ => unreachable!(),
            })
            .collect()
    };
    let s = extrude_arc_line_region(
        &ArcLineRegion {
            origin: p.origin,
            outer: p.segments,
            holes: vec![moved(-2.0), moved(2.0)],
        },
        2.0,
        t().absolute(),
    )
    .unwrap();
    let c = clip(&s, Point3::new(0.0, 4.5, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let a = 5.0 + 0.75_f64.sqrt();
    check_ranges(&c, &[[-a, a]]);
    let c = clip(&s, Point3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let h = 0.75_f64.sqrt();
    check_ranges(&c, &[[-6.0, -2.0 - h], [-2.0 + h, 2.0 - h], [2.0 + h, 6.0]]);
}
#[test]
fn concave_polygon_and_rotated_caps_preserve_intervals() {
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0.0, 0.0, 0.0),
            outer: vec![
                [-4.0, -3.0],
                [4.0, -3.0],
                [4.0, 3.0],
                [2.0, 3.0],
                [2.0, -1.0],
                [-2.0, -1.0],
                [-2.0, 3.0],
                [-4.0, 3.0],
            ],
            holes: vec![],
        },
        Vec3::new(0.5, 0.3, 2.0),
        t().absolute(),
    )
    .unwrap();
    check_ranges(
        &clip(&s, Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        &[[-4.0, -2.0], [2.0, 4.0]],
    );
    let tr = Transform::translation(Vec3::new(12.0, -7.0, 3.0))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.8).unwrap())
        .unwrap();
    check_ranges(
        &clip(
            &s.transformed(tr, t().absolute()).unwrap(),
            tr.point(Point3::new(0.0, 0.0, 0.0)),
            tr.vector(Vec3::new(1.0, 0.0, 0.0)),
        ),
        &[[-4.0, -2.0], [2.0, 4.0]],
    );
}
#[test]
fn two_trimmed_faces_share_finite_edge_and_normalized_pcurves() {
    let a = block();
    let b = make_box(
        BoxSpec {
            min: Point3::new(0.0, -2.0, -1.0),
            size: Vec3::new(1.0, 4.0, 2.0),
        },
        t().absolute(),
    )
    .unwrap();
    let PlanarFacesIntersection::Segments(segments) =
        intersect_planar_faces(&a, 0, &b, 5, t()).unwrap()
    else {
        panic!()
    };
    assert_eq!(segments.len(), 2);
    for seg in &segments {
        let Curve::Line { a: p, b: q } = seg.curve else {
            panic!()
        };
        assert!(((q - p).norm() - 1.0).abs() < 1e-12);
        for t in [0.0, 0.5, 1.0] {
            let p = seg.curve.evaluate(t);
            for (face, pc) in [
                (&a.shell.faces[0], &seg.first),
                (&b.shell.faces[5], &seg.second),
            ] {
                let uv = pc.evaluate(t);
                assert!((face.surface.evaluate(uv[0], uv[1]) - p).norm() < 1e-12);
            }
        }
    }
    let PlanarFacesIntersection::Segments(reverse) =
        intersect_planar_faces(&b, 5, &a, 0, t()).unwrap()
    else {
        panic!()
    };
    assert_eq!(reverse.len(), 2);
    let length: f64 = reverse
        .iter()
        .map(|s| (s.curve.evaluate(1.0) - s.curve.evaluate(0.0)).norm())
        .sum();
    assert!((length - 2.0).abs() < 1e-12);
}
#[test]
fn parallel_disjoint_coplanar_and_endpoint_only_faces_are_distinguished() {
    let a = block();
    assert!(matches!(
        intersect_planar_faces(&a, 0, &a, 1, t()).unwrap(),
        PlanarFacesIntersection::Parallel
    ));
    assert!(intersect_planar_faces(&a, 0, &a, 0, t()).is_err());
    let b = make_box(
        BoxSpec {
            min: Point3::new(0.0, 4.0, -1.0),
            size: Vec3::new(1.0, 2.0, 2.0),
        },
        t().absolute(),
    )
    .unwrap();
    assert!(
        matches!(intersect_planar_faces(&a,0,&b,5,t()).unwrap(),PlanarFacesIntersection::Segments(v) if v.is_empty())
    );
    let b = make_box(
        BoxSpec {
            min: Point3::new(0.0, 3.0, -1.0),
            size: Vec3::new(1.0, 2.0, 2.0),
        },
        t().absolute(),
    )
    .unwrap();
    assert!(intersect_planar_faces(&a, 0, &b, 5, t()).is_err());
}
#[test]
fn tangent_vertex_overlap_invalid_and_off_plane_cuts_are_rejected() {
    let s = block();
    for (a, d) in [
        (Point3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        (Point3::new(0.0, 1.0 + 1e-9, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        (Point3::new(0.0, 3.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        (Point3::new(-4.0, -3.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
        (Point3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0)),
        (Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 1e-12)),
        (Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)),
        (Point3::new(f64::NAN, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
    ] {
        assert!(
            clip_line_to_planar_face(&s, 0, a, d, t()).is_err(),
            "{a:?} {d:?}"
        );
    }
    assert!(clip_line_to_planar_face(
        &s,
        999,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
    assert!(clip_line_to_planar_face(
        &s,
        6,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
    let mut broken = s.clone();
    broken.edges[0].vertices[0] = usize::MAX;
    assert!(clip_line_to_planar_face(
        &broken,
        0,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
}
#[test]
fn small_features_and_original_parameter_scale_are_checked() {
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.0).unwrap();
    let s = make_box(
        BoxSpec {
            min: Point3::new(-4e-6, -3e-6, 0.0),
            size: Vec3::new(8e-6, 6e-6, 2e-6),
        },
        tol.absolute(),
    )
    .unwrap();
    for speed in [1e-300, 1e300] {
        let c = clip_line_to_planar_face(
            &s,
            0,
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(speed, 0.0, 0.0),
            tol,
        )
        .unwrap();
        assert_eq!(c.intervals.len(), 1);
        assert!((c.events[0].parameter * speed + 4e-6).abs() < 1e-19);
        assert!((c.events[1].parameter * speed - 4e-6).abs() < 1e-19);
    }
    assert!(clip_line_to_planar_face(
        &block(),
        0,
        Point3::new(1e30, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
}

#[test]
fn near_plane_offsets_and_unrepresentable_original_parameters_are_rejected() {
    let s = block();
    let d = Vec3::new(1.0, 0.0, 0.0);
    assert!(clip_line_to_planar_face(&s, 0, Point3::new(0.0, 0.0, 1e-9), d, t()).is_err());
    let s = make_box(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        t().absolute(),
    )
    .unwrap();
    assert!(clip_line_to_planar_face(
        &s,
        0,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(f64::from_bits(1), 0.0, 0.0),
        t()
    )
    .is_err());
}
