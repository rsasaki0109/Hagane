use hagane::*;
use std::f64::consts::PI;
fn tol() -> Tolerance {
    Tolerance::default()
}
fn rounded() -> Solid {
    extrude_arc_line(
        &rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, tol()).unwrap(),
        2.0,
        tol(),
    )
    .unwrap()
}
fn split(s: &Solid, face: usize, a: Point3, d: Vec3) -> PlanarFaceSplit {
    split_planar_face(s, face, a, d, GeometryTolerance::default()).unwrap()
}
fn closed(s: &Solid) {
    let m = s.tessellate(0.01, tol()).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for (ti, tri) in m.triangles.iter().enumerate() {
        let p = tri.map(|i| m.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(m.normals[tri[0]]) > 0.0);
        for i in 0..3 {
            if let Surface::FramedCylinder { frame, radius, .. } =
                s.shell.faces[m.face_ids[ti]].surface
            {
                let q = frame.local_point((p[i] + p[(i + 1) % 3]) * 0.5);
                assert!(radius - q.x.hypot(q.y) <= 0.01 + 1e-10);
            }

            let a = key(p[i]);
            let b = key(p[(i + 1) % 3]);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(edge).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(
        uses.values().all(|&(n, s)| n == 2 && s == 0),
        "bad {:?}",
        uses.iter()
            .filter(|(_, u)| u.0 != 2 || u.1 != 0)
            .collect::<Vec<_>>()
    );
}
#[test]
fn curved_cap_split_refines_both_cylinder_walls_and_opposite_rims() {
    let s = rounded();
    let r = split(&s, 0, Point3::new(0.0, 4.5, 0.0), Vec3::new(1.0, 0.0, 0.0));
    r.solid.validate(tol()).unwrap();
    assert_eq!(r.solid.vertices.len(), s.vertices.len() + 4);
    assert_eq!(r.solid.edges.len(), s.edges.len() + 7);
    assert_eq!(r.solid.shell.faces.len(), s.shell.faces.len() + 3);
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
    assert!((r.solid.volume().unwrap() - (120.0 - (4.0 - PI)) * 2.0).abs() < 1e-10);
    assert!((r.solid.bounds().min - s.bounds().min).norm() < 1e-12);
    assert!((r.solid.bounds().max - s.bounds().max).norm() < 1e-12);
    assert_eq!(s.shell.faces.len(), 10);
    closed(&r.solid);
    for e in &r.solid.edges {
        if let Curve::Arc {
            frame,
            radius,
            sweep,
        } = e.curve
        {
            assert!(sweep > 0.0 && sweep <= PI);
            assert!(
                (frame.local_point(e.curve.evaluate(sweep / 2.0)).norm() - radius).abs() < 1e-10
            );
        }
    }
}
#[test]
fn top_bottom_reversed_and_repeated_arc_cuts_preserve_mesh() {
    for (face, z, d) in [(0, 0.0, 1.0), (1, 2.0, -2.0)] {
        let r = split(
            &rounded(),
            face,
            Point3::new(0.0, 4.5, z),
            Vec3::new(d, 0.0, 0.0),
        );
        closed(&r.solid);
        let r = split(
            &r.solid,
            1 - face,
            Point3::new(0.0, -4.5, 2.0 - z),
            Vec3::new(1.0, 0.0, 0.0),
        );
        r.solid.validate(tol()).unwrap();
        closed(&r.solid);
    }
}
#[test]
fn disk_bounded_arcs_and_analytic_hole_ownership() {
    let arcs = [0.0, PI]
        .map(|start_angle| PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 3.0,
            start_angle,
            sweep: PI,
        })
        .to_vec();
    let p = ArcLineRegion {
        origin: Point3::new(0.0, 0.0, 0.0),
        outer: arcs,
        holes: vec![[0.0, PI]
            .map(|start_angle| PlanarSegment::Arc {
                center: [-1.0, 0.0],
                radius: 0.5,
                start_angle,
                sweep: PI,
            })
            .to_vec()],
    };
    let s = extrude_arc_line_region(&p, 2.0, tol()).unwrap();
    let r = split(&s, 0, Point3::new(0.5, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(
        r.faces
            .iter()
            .map(|&i| r.solid.shell.faces[i].wires.len())
            .sum::<usize>(),
        3
    );
    assert!((r.solid.volume().unwrap() - 17.5 * PI).abs() < 1e-10);
    closed(&r.solid);
}
#[test]
fn inward_notch_wall_and_rigid_placement_remain_consistent() {
    let line = |a, b| PlanarSegment::Line { a, b };
    let p = ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: vec![
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
        ],
    };
    let s = extrude_arc_line(&p, 2.0, tol()).unwrap();
    let tr = Transform::translation(Vec3::new(12.0, -7.0, 3.0))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.7).unwrap())
        .unwrap();
    let s = s.transformed(tr, tol()).unwrap();
    let r = split(
        &s,
        0,
        tr.point(Point3::new(0.0, 0.0, 0.0)),
        tr.vector(Vec3::new(0.0, 1.0, 0.0)),
    );
    assert_eq!(
        r.solid
            .shell
            .faces
            .iter()
            .filter(|f| f.orientation == -1 && matches!(f.surface, Surface::FramedCylinder { .. }))
            .count(),
        2
    );
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
    closed(&r.solid);
}
#[test]
fn unsupported_same_edge_vertex_tangent_and_hole_cuts_are_rejected() {
    let s = rounded();
    for y in [4.0, 5.0, 5.0 - 1e-9] {
        assert!(split_planar_face(
            &s,
            0,
            Point3::new(0.0, y, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            GeometryTolerance::default()
        )
        .is_err());
    }
    let p = ArcLineProfile {
        origin: Point3::new(0.0, 0.0, 0.0),
        segments: [0.0, PI]
            .map(|start_angle| PlanarSegment::Arc {
                center: [0.0, 0.0],
                radius: 3.0,
                start_angle,
                sweep: PI,
            })
            .to_vec(),
    };
    let s = extrude_arc_line(&p, 2.0, tol()).unwrap();
    assert!(split_planar_face(
        &s,
        0,
        Point3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        GeometryTolerance::default()
    )
    .is_err());
}
#[test]
fn microscopic_arc_splits_require_a_resolving_tolerance() {
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.0).unwrap();
    let p = rounded_rectangle_profile(
        Point3::new(0.0, 0.0, 0.0),
        12e-6,
        10e-6,
        1e-6,
        tol.absolute(),
    )
    .unwrap();
    let s = extrude_arc_line(&p, 2e-6, tol.absolute()).unwrap();
    let r = split_planar_face(
        &s,
        0,
        Point3::new(0.0, 4.5e-6, 0.0),
        Vec3::new(1e300, 0.0, 0.0),
        tol,
    )
    .unwrap();
    r.solid.validate(tol.absolute()).unwrap();
    assert!((r.solid.volume().unwrap() - (120.0 - (4.0 - PI)) * 2e-18).abs() < 1e-29);
    let m = r.solid.tessellate(1e-8, tol.absolute()).unwrap();
    assert!(m.signed_volume() > 0.0);
}

#[test]
fn tolerance_valid_but_mismatched_rim_domains_are_explicitly_rejected() {
    let mut s = rounded();
    let edge = s.shell.faces[5].wires[0].coedges[0].edge;
    let Curve::Arc { sweep, .. } = &mut s.edges[edge].curve else {
        panic!()
    };
    *sweep += 1e-12;
    s.validate(tol()).unwrap();
    assert!(split_planar_face(
        &s,
        0,
        Point3::new(0.0, 4.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        GeometryTolerance::default()
    )
    .is_err());
}
