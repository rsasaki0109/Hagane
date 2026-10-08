use hagane::*;
fn t() -> GeometryTolerance {
    GeometryTolerance::default()
}
fn block() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        t().absolute(),
    )
    .unwrap()
}
fn hole() -> Solid {
    subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        CylinderSpec {
            base: Point3::new(0.0, 0.0, -1.0),
            radius: 0.5,
            height: 4.0,
        },
        t().absolute(),
    )
    .unwrap()
}
fn split(s: &Solid, face: usize, y: f64) -> PlanarFaceSplit {
    split_planar_face(
        s,
        face,
        Point3::new(0.0, y, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t(),
    )
    .unwrap()
}
fn closed_mesh(s: &Solid) {
    let m = s.tessellate(0.01, t().absolute()).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for tri in &m.triangles {
        for i in 0..3 {
            let a = key(m.positions[tri[i]]);
            let b = key(m.positions[tri[(i + 1) % 3]]);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(edge).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(
        uses.values().all(|&(n, s)| n == 2 && s == 0),
        "bad edges {:?}",
        uses.iter()
            .filter(|(_, v)| v.0 != 2 || v.1 != 0)
            .collect::<Vec<_>>()
    );
}
#[test]
fn box_split_preserves_closed_topology_volume_and_bounds() {
    let s = block();
    let r = split(&s, 0, 0.0);
    assert_eq!(r.solid.vertices.len(), 10);
    assert_eq!(r.solid.edges.len(), 15);
    assert_eq!(r.solid.shell.faces.len(), 7);
    assert_eq!(r.faces, [0, 6]);
    assert_eq!(r.solid.bounds(), s.bounds());
    assert!((r.solid.volume().unwrap() - 96.0).abs() < 1e-10);
    r.solid.validate(t().absolute()).unwrap();
    closed_mesh(&r.solid);
    assert_eq!(s.shell.faces.len(), 6);
}
#[test]
fn circle_hole_moves_to_one_child_without_being_polygonized() {
    let s = hole();
    let r = split(&s, 0, 1.0);
    let counts = r.faces.map(|i| r.solid.shell.faces[i].wires.len());
    assert!(counts == [1, 2] || counts == [2, 1]);
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
    closed_mesh(&r.solid);
}
#[test]
fn repeated_and_reversed_cuts_keep_shared_coedges() {
    let s = split(&block(), 0, 0.0).solid;
    let face = s
        .shell
        .faces
        .iter()
        .position(|f| {
            matches!(f.surface,Surface::Plane{origin,..} if origin.z==0.0)
                && f.wires[0]
                    .coedges
                    .iter()
                    .any(|c| c.pcurve.evaluate(0.0)[1] > 3.5)
        })
        .unwrap();
    let r = split_planar_face(
        &s,
        face,
        Point3::new(0.0, 1.0, 0.0),
        Vec3::new(-2.0, 0.0, 0.0),
        t(),
    )
    .unwrap();
    r.solid.validate(t().absolute()).unwrap();
    closed_mesh(&r.solid);
    assert_eq!(r.solid.shell.faces.len(), 8);
}
#[test]
fn contacts_hole_crossings_curved_outer_and_disjoint_cuts_are_rejected() {
    for y in [0.0, 0.5, 3.0, 4.0] {
        assert!(split_planar_face(
            &hole(),
            0,
            Point3::new(0.0, y, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            t()
        )
        .is_err());
    }
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0.0, 0.0, 0.0),
            radius: 2.0,
            height: 2.0,
        },
        t().absolute(),
    )
    .unwrap();
    assert!(split_planar_face(
        &s,
        0,
        Point3::new(0.0, 0.2, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
}
#[test]
fn placement_and_small_scale_preserve_split_geometry() {
    let tr = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.7).unwrap();
    let s = block().transformed(tr, t().absolute()).unwrap();
    let r = split_planar_face(
        &s,
        0,
        tr.point(Point3::new(0.0, 0.0, 0.0)),
        tr.vector(Vec3::new(1.0, 0.0, 0.0)),
        t(),
    )
    .unwrap();
    closed_mesh(&r.solid);
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.0).unwrap();
    let s = make_box(
        BoxSpec {
            min: Point3::new(-4e-6, -3e-6, 0.0),
            size: Vec3::new(8e-6, 6e-6, 2e-6),
        },
        tol.absolute(),
    )
    .unwrap();
    let r = split_planar_face(
        &s,
        0,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1e300, 0.0, 0.0),
        tol,
    )
    .unwrap();
    r.solid.validate(tol.absolute()).unwrap();
    assert!((r.solid.volume().unwrap() - 96e-18).abs() < 1e-30);
}
#[test]
fn splitting_a_side_face_keeps_circular_cap_mesh_boundaries_conforming() {
    let s = hole();
    let r = split_planar_face(
        &s,
        3,
        Point3::new(4.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        t(),
    )
    .unwrap();
    closed_mesh(&r.solid);
}
#[test]
fn concave_single_interval_splits_and_multiple_intervals_are_rejected() {
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
        Vec3::new(0.0, 0.0, 2.0),
        t().absolute(),
    )
    .unwrap();
    let r = split(&s, 0, -2.0);
    closed_mesh(&r.solid);
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
    assert!(split_planar_face(
        &s,
        0,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        t()
    )
    .is_err());
}
#[test]
fn polygon_holes_and_unsupported_arc_neighbors_are_checked() {
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0.0, 0.0, 0.0),
            outer: vec![[-4.0, -3.0], [4.0, -3.0], [4.0, 3.0], [-4.0, 3.0]],
            holes: vec![vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]],
        },
        Vec3::new(0.0, 0.0, 2.0),
        t().absolute(),
    )
    .unwrap();
    let r = split(&s, 0, 2.0);
    closed_mesh(&r.solid);
    assert_eq!(
        r.faces
            .iter()
            .map(|&i| r.solid.shell.faces[i].wires.len())
            .sum::<usize>(),
        3
    );
    let p = rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, t().absolute())
        .unwrap();
    let s = extrude_arc_line(&p, 2.0, t().absolute()).unwrap();
    assert!(split_planar_face(
        &s,
        2,
        Point3::new(0.0, -5.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        t()
    )
    .is_err());
}
