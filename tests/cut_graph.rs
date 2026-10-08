use hagane::*;
fn profile() -> PolygonProfile {
    PolygonProfile {
        origin: Point3::new(0., 0., 0.),
        outer: vec![[-6., -4.], [6., -4.], [6., 4.], [-6., 4.]],
        holes: vec![
            vec![[-3., -1.], [-1., -1.], [-1., 1.], [-3., 1.]],
            vec![[1., -1.], [3., -1.], [3., 1.], [1., 1.]],
        ],
    }
}
fn cut(s: &Solid, y: f64, d: f64) -> Result<PlanarFaceSubdivision> {
    subdivide_planar_face(
        s,
        0,
        Point3::new(0., y, 0.),
        Vec3::new(d, 0., 0.),
        GeometryTolerance::default(),
    )
}
fn check(s: &Solid, r: &PlanarFaceSubdivision, children: usize, cuts: usize) {
    assert_eq!(r.faces.len(), children);
    assert_eq!(r.cut_edges.len(), cuts);
    r.solid.validate(Tolerance::default()).unwrap();
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-9);
    assert_eq!(r.solid.bounds(), s.bounds());
    let m = r.solid.tessellate(0.01, Tolerance::default()).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for tri in m.triangles {
        for i in 0..3 {
            let a = key(m.positions[tri[i]]);
            let b = key(m.positions[tri[(i + 1) % 3]]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(k).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
}
#[test]
fn two_crossed_holes_become_outer_boundaries() {
    let s = extrude_polygon(&profile(), Vec3::new(0., 0., 2.), Tolerance::default()).unwrap();
    for d in [1., -3.] {
        let r = cut(&s, 0., d).unwrap();
        check(&s, &r, 2, 3);
        assert!(r
            .faces
            .iter()
            .all(|&i| r.solid.shell.faces[i].wires.len() == 1));
    }
    assert_eq!(s.shell.faces[0].wires.len(), 3);
}
#[test]
fn uncut_holes_keep_single_owner() {
    let s = extrude_polygon(&profile(), Vec3::new(0., 0., 2.), Tolerance::default()).unwrap();
    let r = cut(&s, 2., 1.).unwrap();
    check(&s, &r, 2, 1);
    assert_eq!(
        r.faces
            .iter()
            .map(|&i| r.solid.shell.faces[i].wires.len())
            .sum::<usize>(),
        4
    );
}
#[test]
fn disconnected_half_plane_produces_three_children() {
    let mut p = profile();
    p.holes.clear();
    p.outer = vec![
        [-4., -3.],
        [4., -3.],
        [4., 3.],
        [2., 3.],
        [2., -1.],
        [-2., -1.],
        [-2., 3.],
        [-4., 3.],
    ];
    let s = extrude_polygon(&p, Vec3::new(0., 0., 2.), Tolerance::default()).unwrap();
    check(&s, &cut(&s, 0., 1.).unwrap(), 3, 2);
}
#[test]
fn contacts_and_invalid_requests_are_errors() {
    let s = extrude_polygon(&profile(), Vec3::new(0., 0., 2.), Tolerance::default()).unwrap();
    for y in [1., 1. - 1e-9, 4., 5., f64::NAN] {
        assert!(cut(&s, y, 1.).is_err());
    }
    assert!(cut(&s, 0., 0.).is_err());
    assert!(subdivide_planar_face(
        &s,
        999,
        Point3::new(0., 0., 0.),
        Vec3::new(1., 0., 0.),
        GeometryTolerance::default()
    )
    .is_err());
}
#[test]
fn placed_cut_graph_and_tiny_geometry() {
    let tol = Tolerance::default();
    let s = extrude_polygon(&profile(), Vec3::new(0., 0., 2.), tol).unwrap();
    let tr = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let s = s.transformed(tr, tol).unwrap();
    let r = subdivide_planar_face(
        &s,
        0,
        tr.point(Point3::new(0., 0., 0.)),
        tr.vector(Vec3::new(1., 0., 0.)),
        GeometryTolerance::default(),
    )
    .unwrap();
    r.solid.validate(tol).unwrap();
    assert!((r.solid.volume().unwrap() - 176.).abs() < 1e-9);
    let mut p = profile();
    for ring in std::iter::once(&mut p.outer).chain(p.holes.iter_mut()) {
        for pt in ring {
            pt[0] *= 1e-6;
            pt[1] *= 1e-6;
        }
    }
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = extrude_polygon(&p, Vec3::new(0., 0., 2e-6), tol.absolute()).unwrap();
    let r = subdivide_planar_face(
        &s,
        0,
        Point3::new(0., 0., 0.),
        Vec3::new(1e300, 0., 0.),
        tol,
    )
    .unwrap();
    r.solid.validate(tol.absolute()).unwrap();
    assert!((r.solid.volume().unwrap() - 176e-18).abs() < 1e-29);
}
#[test]
fn crossed_arc_hole_refines_inward_cylinder_walls() {
    let ring = |radius| {
        [0., std::f64::consts::PI]
            .map(|start_angle| PlanarSegment::Arc {
                center: [0., 0.],
                radius,
                start_angle,
                sweep: std::f64::consts::PI,
            })
            .to_vec()
    };
    let s = extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: ring(4.),
            holes: vec![ring(1.)],
        },
        2.,
        Tolerance::default(),
    )
    .unwrap();
    let r = subdivide_planar_face(
        &s,
        0,
        Point3::new(0., 0., 0.),
        Vec3::new(0., 1., 0.),
        GeometryTolerance::default(),
    )
    .unwrap();
    check(&s, &r, 2, 2);
    assert!(r
        .faces
        .iter()
        .all(|&i| r.solid.shell.faces[i].wires.len() == 1));
}
#[test]
fn repeated_arc_hits_and_periodic_circle_crossings_are_rejected() {
    let t = GeometryTolerance::default();
    let p = ArcLineProfile {
        origin: Point3::new(0., 0., 0.),
        segments: [0., std::f64::consts::PI]
            .map(|start_angle| PlanarSegment::Arc {
                center: [0., 0.],
                radius: 4.,
                start_angle,
                sweep: std::f64::consts::PI,
            })
            .to_vec(),
    };
    let s = extrude_arc_line(&p, 2., t.absolute()).unwrap();
    assert!(cut(&s, 1., 1.).is_err());
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4.,
            height: 2.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(cut(&s, 1., 1.).is_err());
}
