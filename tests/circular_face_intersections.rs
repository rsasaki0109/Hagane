use hagane::*;
use std::f64::consts::PI;
fn solid(skew: bool, scale: f64, t: Tolerance) -> Solid {
    let p = ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: vec![
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 2. * scale,
                start_angle: 0.,
                sweep: PI,
            },
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 2. * scale,
                start_angle: PI,
                sweep: PI,
            },
        ],
        holes: vec![],
    };
    extrude_arc_line_region_along(
        &p,
        Vec3::new(
            if skew { 4. * scale } else { 0. },
            if skew { 2. * scale } else { 0. },
            4. * scale,
        ),
        t,
    )
    .unwrap()
}
fn points(r: CircularFaceLineIntersection) -> Vec<CircularFacePoint> {
    let CircularFaceLineIntersection::Points { points, .. } = r else {
        panic!("expected points: {r:?}")
    };
    points
}
fn verify(s: &Solid, face: usize, p: &CircularFacePoint) {
    assert!((s.shell.faces[face].surface.evaluate(p.uv[0], p.uv[1]) - p.point).norm() < 1e-10);
    for b in &p.boundaries {
        assert_eq!(
            s.shell.faces[face].wires[b.wire].coedges[b.coedge].edge,
            b.edge
        );
        assert!((s.edges[b.edge].curve.evaluate(b.edge_parameter) - p.point).norm() < 1e-10);
    }
}
#[test]
fn partial_walls_keep_only_the_selected_angular_half_for_normal_and_skew_solids() {
    let t = GeometryTolerance::default();
    for skew in [false, true] {
        let s = solid(skew, 1., t.absolute());
        for y in [-1.5, -0.5, 0.5, 1.5] {
            let a = Point3::new(
                -5. + if skew { 2. } else { 0. },
                y + if skew { 1. } else { 0. },
                2.,
            );
            let d = Vec3::new(2., 0., 0.);
            let (inside, outside) = if y > 0. { (2, 3) } else { (3, 2) };
            assert_eq!(
                intersect_line_circular_face(&s, outside, a, d, t).unwrap(),
                CircularFaceLineIntersection::Empty
            );
            let hits = points(intersect_line_circular_face(&s, inside, a, d, t).unwrap());
            assert_eq!(hits.len(), 2);
            let root = (4. - y * y).sqrt();
            for (p, expected) in hits.iter().zip([(5. - root) / 2., (5. + root) / 2.]) {
                assert!((p.parameter - expected).abs() < 1e-12);
                assert!(p.boundaries.is_empty());
                verify(&s, inside, p);
            }
        }
    }
}
#[test]
fn exact_generators_rims_vertices_and_reversal_preserve_edge_parameters() {
    let t = GeometryTolerance::default();
    let s = solid(true, 1., t.absolute());
    for z in [0., 2., 4.] {
        let hits = points(
            intersect_line_circular_face(
                &s,
                2,
                Point3::new(-5. + z, 0.5 * z, z),
                Vec3::new(2., 0., 0.),
                t,
            )
            .unwrap(),
        );
        assert_eq!(hits.len(), 2);
        for (p, coedge) in hits.iter().zip([1, 3]) {
            verify(&s, 2, p);
            let boundary = p.boundaries.iter().find(|b| b.coedge == coedge).unwrap();
            assert_eq!(boundary.edge_parameter, z / 4.);
            assert_eq!(p.boundaries.len(), if z == 2. { 1 } else { 2 });
        }
    }
    let a = Point3::new(2., 0., 0.);
    let d = Vec3::new(4., 2., 4.);
    for (a, d, reversed) in [(a, d, false), (a + d, d * (-1.), true)] {
        let CircularFaceLineIntersection::Coincident { start, end } =
            intersect_line_circular_face(&s, 2, a, d, t).unwrap()
        else {
            panic!("expected trimmed generator")
        };
        assert!((start.parameter).abs() < 1e-12 && (end.parameter - 1.).abs() < 1e-12);
        verify(&s, 2, &start);
        verify(&s, 2, &end);
        assert_eq!(
            start
                .boundaries
                .iter()
                .map(|b| b.coedge)
                .collect::<Vec<_>>(),
            if reversed { vec![2, 3] } else { vec![0, 3] }
        );
        assert_eq!(
            end.boundaries.iter().map(|b| b.coedge).collect::<Vec<_>>(),
            if reversed { vec![0, 3] } else { vec![2, 3] }
        );
    }
    assert_eq!(
        intersect_line_circular_face(&s, 2, Point3::new(0., -2., 0.), d, t).unwrap(),
        CircularFaceLineIntersection::Empty
    );
    let tangent =
        intersect_line_circular_face(&s, 2, Point3::new(-3., 3., 2.), Vec3::new(2., 0., 0.), t)
            .unwrap();
    let CircularFaceLineIntersection::Points { points, contact } = tangent else {
        panic!("expected tangent")
    };
    assert_eq!(contact, IntersectionContact::Tangent);
    assert_eq!(points.len(), 1);
    assert_eq!(
        intersect_line_circular_face(&s, 2, Point3::new(-3., -1., 2.), Vec3::new(2., 0., 0.), t)
            .unwrap(),
        CircularFaceLineIntersection::Empty
    );
}
#[test]
fn full_periodic_seams_retain_both_uses_without_duplicate_hits() {
    let t = GeometryTolerance::default();
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    let hits = points(
        intersect_line_circular_face(&s, 2, Point3::new(-5., 0., 2.), Vec3::new(2., 0., 0.), t)
            .unwrap(),
    );
    assert_eq!(hits.len(), 2);
    assert!(hits[0].boundaries.is_empty());
    assert_eq!(hits[1].boundaries.len(), 2);
    assert_eq!(hits[1].boundaries[0].edge, hits[1].boundaries[1].edge);
    assert_eq!(
        hits[1]
            .boundaries
            .iter()
            .map(|b| b.coedge)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    for p in &hits {
        verify(&s, 2, p);
    }
    assert_eq!(
        points(
            intersect_line_circular_face(
                &s,
                2,
                Point3::new(-5., 1e-9, 2.),
                Vec3::new(2., 0., 0.),
                t
            )
            .unwrap()
        )
        .len(),
        2
    );
}
#[test]
fn inward_normals_placement_tiny_dimensions_and_near_boundaries() {
    let t = GeometryTolerance::default();
    let tube = make_tube(
        TubeSpec {
            base: Point3::new(0., 0., 0.),
            outer_radius: 3.,
            inner_radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    let face = tube
        .shell
        .faces
        .iter()
        .position(|f| f.orientation == -1 && !matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    let hits = points(
        intersect_line_circular_face(
            &tube,
            face,
            Point3::new(-5., 1., 2.),
            Vec3::new(2., 0., 0.),
            t,
        )
        .unwrap(),
    );
    for p in hits {
        assert!(p.normal.dot(Vec3::new(-p.point.x, -p.point.y, 0.)) > 0.);
    }
    let s = solid(true, 1., t.absolute());
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let placed = s.transformed(tr, t.absolute()).unwrap();
    let hits = points(
        intersect_line_circular_face(
            &placed,
            2,
            tr.point(Point3::new(-3., 2., 2.)),
            tr.vector(Vec3::new(2., 0., 0.)),
            t,
        )
        .unwrap(),
    );
    assert_eq!(hits.len(), 2);
    for p in &hits {
        verify(&placed, 2, p);
    }
    let tiny = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = solid(true, 1e-6, tiny.absolute());
    assert_eq!(
        points(
            intersect_line_circular_face(
                &s,
                2,
                Point3::new(-3e-6, 2e-6, 2e-6),
                Vec3::new(2., 0., 0.),
                tiny
            )
            .unwrap()
        )
        .len(),
        2
    );
    let s = solid(true, 1., t.absolute());
    for y in [-1e-9, 1e-9] {
        assert!(intersect_line_circular_face(
            &s,
            2,
            Point3::new(-3., 1. + y, 2.),
            Vec3::new(2., 0., 0.),
            t
        )
        .is_err());
    }
    assert!(
        intersect_line_circular_face(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), t)
            .is_err()
    );
    assert!(intersect_line_circular_face(
        &s,
        999,
        Point3::new(0., 0., 0.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
    let mut invalid = s.clone();
    invalid.shell.faces[2].wires[0].coedges[1].forward = false;
    assert!(intersect_line_circular_face(
        &invalid,
        2,
        Point3::new(100., 100., 100.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
    assert!(intersect_line_circular_face(
        &s,
        2,
        Point3::new(f64::NAN, 0., 0.),
        Vec3::new(1., 0., 0.),
        t
    )
    .is_err());
}
#[test]
fn quarter_arc_retains_one_root_and_its_original_parameter() {
    let t = GeometryTolerance::default();
    let p = rounded_rectangle_profile(Point3::new(0., 0., 0.), 8., 6., 1., t.absolute()).unwrap();
    let s = extrude_arc_line_in_frame(&p, Vec3::new(4., 2., 4.), Frame3::IDENTITY, t.absolute())
        .unwrap();
    let hits = points(
        intersect_line_circular_face(&s, 3, Point3::new(0., -1.5, 2.), Vec3::new(1., 0., 0.), t)
            .unwrap(),
    );
    assert_eq!(hits.len(), 1);
    assert!((hits[0].parameter - (5. + 0.75_f64.sqrt())).abs() < 1e-12);
    assert!((hits[0].uv[0] - PI / 3.).abs() < 1e-12);
    assert!(hits[0].boundaries.is_empty());
    verify(&s, 3, &hits[0]);
}
#[test]
fn shared_edge_generator_certificate_clears_trigonometric_frame_ambiguity() {
    let t = GeometryTolerance::default();
    let s = solid(true, 1., t.absolute());
    // The second arc frame uses sin(pi); its raw surface reduction may be
    // near-generator, while the actual shared straight edge is exactly incident.
    let a = Point3::new(2., 0., 0.);
    let d = Vec3::new(4., 2., 4.);
    let CircularFaceLineIntersection::Coincident { start, end } =
        intersect_line_circular_face(&s, 3, a, d, t).unwrap()
    else {
        panic!("expected exact shared-edge overlap")
    };
    assert_eq!(start.parameter, 0.);
    assert_eq!(end.parameter, 1.);
    verify(&s, 3, &start);
    verify(&s, 3, &end);
    for offset in [-1e-9, 1e-9] {
        assert!(intersect_line_circular_face(&s, 3, a + Vec3::new(offset, 0., 0.), d, t).is_err());
    }
    assert!(intersect_line_circular_face(&s, 3, a, Vec3::new(4. + 1e-11, 2., 4.), t).is_err());
}
#[test]
fn certified_boundary_crossing_preserves_a_hit_without_snapping_nearby_lines() {
    let t = GeometryTolerance::default();
    let s = solid(true, 1., t.absolute());
    let a = Point3::new(3., 0., 2.);
    let d = Vec3::new(1., 1., 0.);
    let hits = points(intersect_line_circular_face(&s, 3, a, d, t).unwrap());
    assert_eq!(hits.len(), 2);
    assert!((hits[0].parameter + 1.).abs() < 1e-12);
    assert!(hits[0].boundaries.is_empty());
    assert!((hits[1].parameter - 1.).abs() < 1e-12);
    assert_eq!(hits[1].boundaries.len(), 1);
    assert_eq!(hits[1].boundaries[0].coedge, 1);
    assert!((hits[1].boundaries[0].edge_parameter - 0.5).abs() < 1e-12);
    verify(&s, 3, &hits[1]);
    for offset in [-1e-9, 1e-9] {
        assert!(intersect_line_circular_face(&s, 3, a + Vec3::new(offset, 0., 0.), d, t).is_err());
    }
}
#[test]
fn generator_certificates_do_not_hide_lost_radius_at_large_world_coordinates() {
    let t = GeometryTolerance::default();
    let centre = Point3::new(1e15, -1e15, 1e15);
    assert!(make_cylinder(
        CylinderSpec {
            base: centre,
            radius: 2.01,
            height: 4.,
        },
        t.absolute(),
    )
    .is_err());
    // A malformed raw boundary must still be rejected by the query API.
    let mut s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.01,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    let placement = Transform::translation(centre).unwrap();
    for v in &mut s.vertices {
        v.point = placement.point(v.point);
    }
    for e in &mut s.edges {
        e.curve = e.curve.transformed(placement).unwrap();
    }
    for f in &mut s.shell.faces {
        f.surface = f.surface.transformed(placement).unwrap();
    }
    let a = s.edges[s.shell.faces[2].wires[0].coedges[1].edge]
        .curve
        .evaluate(0.);
    assert!(intersect_line_circular_face(&s, 2, a, Vec3::new(0., 0., 4.), t).is_err());
}
