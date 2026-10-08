use hagane::*;
use std::f64::consts::PI;
fn fixture() -> (Solid, [usize; 2]) {
    let t = GeometryTolerance::default();
    let source = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            holes: vec![],
            outer: [0., PI]
                .map(|start_angle| PlanarSegment::Arc {
                    center: [0., 0.],
                    radius: 2.,
                    start_angle,
                    sweep: PI,
                })
                .to_vec(),
        },
        Vec3::new(0., 0., 4.),
        t.absolute(),
    )
    .unwrap();
    let cut = subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(0., 0., 2.),
        Vec3::new(0.25, 0., 1.),
        t,
    )
    .unwrap();
    let children = *cut.split_faces.iter().find(|pair| pair[0] == 2).unwrap();
    (cut.solid, children)
}
fn hits(s: &Solid, f: usize, a: Point3, d: Vec3) -> Vec<CircularFacePoint> {
    let CircularFaceLineIntersection::Points { points, .. } =
        intersect_line_circular_face(s, f, a, d, GeometryTolerance::default()).unwrap()
    else {
        panic!("expected point intersection")
    };
    for p in &points {
        assert!((s.shell.faces[f].surface.evaluate(p.uv[0], p.uv[1]) - p.point).norm() < 1e-10);
        for b in &p.boundaries {
            assert!((s.edges[b.edge].curve.evaluate(b.edge_parameter) - p.point).norm() < 1e-10);
        }
    }
    points
}
#[test]
fn transverse_roots_are_clipped_by_analytic_height() {
    let (s, fs) = fixture();
    for (f, x) in fs.into_iter().zip([-3f64.sqrt(), 3f64.sqrt()]) {
        let p = hits(&s, f, Point3::new(-3., 1., 2.), Vec3::new(2., 0., 0.));
        assert_eq!(p.len(), 1);
        assert!((p[0].parameter - (3. + x) / 2.).abs() < 1e-12);
        assert!(p[0].boundaries.is_empty());
        assert_eq!(
            intersect_line_circular_face(
                &s,
                f,
                Point3::new(-3., -1., 2.),
                Vec3::new(2., 0., 0.),
                GeometryTolerance::default()
            )
            .unwrap(),
            CircularFaceLineIntersection::Empty
        );
    }
}
#[test]
fn shared_ellipse_incidence_preserves_provenance_and_rejects_near_lines() {
    let (s, fs) = fixture();
    let mut shared = None;
    for (f, coedge) in fs.into_iter().zip([2, 0]) {
        let p = hits(&s, f, Point3::new(0., 0., 2.), Vec3::new(0., 2., 0.));
        assert_eq!(p.len(), 1);
        assert!((p[0].parameter - 1.).abs() < 1e-12);
        let b = &p[0].boundaries[0];
        assert_eq!(b.coedge, coedge);
        assert!((b.edge_parameter - PI / 2.).abs() < 1e-12);
        if let Some(edge) = shared {
            assert_eq!(b.edge, edge);
        } else {
            shared = Some(b.edge);
        }
        for epsilon in [-1e-9, 1e-9] {
            assert!(matches!(
                intersect_line_circular_face(
                    &s,
                    f,
                    Point3::new(0., 0., 2. + epsilon),
                    Vec3::new(0., 2., 0.),
                    GeometryTolerance::default()
                ),
                Err(Error::Unsupported(_))
            ));
        }
    }
}
#[test]
fn generator_overlaps_stop_at_ellipse_and_reverse_original_parameters() {
    let (s, fs) = fixture();
    for (f, expected) in fs.into_iter().zip([[0.5, 1.5], [1.5, 2.5]]) {
        for sign in [-1., 1.] {
            let CircularFaceLineIntersection::Coincident { start, end } =
                intersect_line_circular_face(
                    &s,
                    f,
                    Point3::new(0., 2., -1.),
                    Vec3::new(0., 0., 2. * sign),
                    GeometryTolerance::default(),
                )
                .unwrap()
            else {
                panic!("expected bounded generator")
            };
            let target = if sign > 0. {
                expected
            } else {
                [-expected[1], -expected[0]]
            };
            assert!((start.parameter - target[0]).abs() < 1e-12);
            assert!((end.parameter - target[1]).abs() < 1e-12);
            assert!(!start.boundaries.is_empty() && !end.boundaries.is_empty());
        }
    }
}
#[test]
fn coplanar_tangency_and_generator_corners_keep_trim_metadata() {
    let (s, fs) = fixture();
    for f in fs {
        let result = intersect_line_circular_face(
            &s,
            f,
            Point3::new(0., 2., 2.),
            Vec3::new(2., 0., -0.5),
            GeometryTolerance::default(),
        )
        .unwrap();
        let CircularFaceLineIntersection::Points { points, contact } = result else {
            panic!("expected tangent")
        };
        assert_eq!(contact, IntersectionContact::Tangent);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].boundaries.len(), 1);
        let p = hits(&s, f, Point3::new(0., 0., 2.), Vec3::new(2., 0., -0.5));
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|p| p.boundaries.len() == 2));
    }
}
#[test]
fn placed_and_tiny_ellipse_incidence_uses_stored_axes() {
    let (s, fs) = fixture();
    for angle in [0., 0.37] {
        let placed = s
            .transformed(
                Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap(),
                Tolerance::default(),
            )
            .unwrap();
        for f in fs {
            let index = if f == fs[0] { 2 } else { 0 };
            let edge = placed.shell.faces[f].wires[0].coedges[index].edge;
            let Curve::EllipseArc { center, sine, .. } = placed.edges[edge].curve else {
                panic!("ellipse required")
            };
            assert_eq!(hits(&placed, f, center, sine).len(), 1);
        }
    }
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let source = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            holes: vec![],
            outer: [0., PI]
                .map(|start_angle| PlanarSegment::Arc {
                    center: [0., 0.],
                    radius: 2e-6,
                    start_angle,
                    sweep: PI,
                })
                .to_vec(),
        },
        Vec3::new(2e-6, -1e-6, 4e-6),
        t.absolute(),
    )
    .unwrap();
    let cut = subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(1e-6, -0.5e-6, 2e-6),
        Vec3::new(0.25, 0., 1.),
        t,
    )
    .unwrap();
    let pair = cut.split_faces.iter().find(|p| p[0] == 2).unwrap();
    for (f, index) in pair.iter().zip([2, 0]) {
        let edge = cut.solid.shell.faces[*f].wires[0].coedges[index].edge;
        let Curve::EllipseArc { center, sine, .. } = cut.solid.edges[edge].curve else {
            panic!("ellipse required")
        };
        let CircularFaceLineIntersection::Points { points, .. } =
            intersect_line_circular_face(&cut.solid, *f, center, sine, t).unwrap()
        else {
            panic!("expected tiny crossing")
        };
        assert_eq!(points.len(), 1);
        assert!((points[0].parameter - 1.).abs() < 1e-10);
        assert_eq!(points[0].boundaries[0].edge, edge);
    }
}
#[test]
fn malformed_trims_and_invalid_queries_are_not_hidden_by_empty_space() {
    let (mut s, fs) = fixture();
    let t = GeometryTolerance::default();
    assert!(intersect_line_circular_face(
        &s,
        usize::MAX,
        Point3::new(0., 0., 0.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
    assert!(intersect_line_circular_face(
        &s,
        fs[0],
        Point3::new(f64::NAN, 0., 0.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
    assert!(intersect_line_circular_face(
        &s,
        fs[0],
        Point3::new(0., 0., 0.),
        Vec3::new(0., 0., 0.),
        t
    )
    .is_err());
    let PCurve::HeightGraph { ref mut offset, .. } =
        s.shell.faces[fs[0]].wires[0].coedges[2].pcurve
    else {
        panic!("height graph required")
    };
    *offset += 1.;
    assert!(intersect_line_circular_face(
        &s,
        fs[0],
        Point3::new(100., 100., 100.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
}
