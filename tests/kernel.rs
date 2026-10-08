use hagane::*;
use std::f64::consts::{PI, TAU};
fn tol() -> Tolerance {
    Tolerance::default()
}
fn block() -> BoxSpec {
    BoxSpec {
        min: Point3::new(-40.0, -30.0, -12.0),
        size: Vec3::new(80.0, 60.0, 24.0),
    }
}
fn cutter() -> CylinderSpec {
    CylinderSpec {
        base: Point3::new(0.0, 0.0, -20.0),
        radius: 14.0,
        height: 40.0,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9 * b.abs().max(1.0), "{a} != {b}");
}
#[test]
fn box_metrics() {
    let s = make_box(block(), tol()).unwrap();
    s.validate(tol()).unwrap();
    close(s.volume().unwrap(), 80.0 * 60.0 * 24.0);
    assert_eq!(
        s.bounds(),
        Bounds {
            min: block().min,
            max: block().min + block().size
        }
    );
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (8, 12, 6)
    );
}
#[test]
fn cylinder_metrics() {
    let c = cutter();
    let s = make_cylinder(c, tol()).unwrap();
    close(s.volume().unwrap(), PI * c.radius * c.radius * c.height);
    assert_eq!(
        s.bounds(),
        Bounds {
            min: Point3::new(-14.0, -14.0, -20.0),
            max: Point3::new(14.0, 14.0, 20.0)
        }
    );
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (2, 3, 3)
    );
}
#[test]
fn exact_difference() {
    let s = subtract_through_cylinder(block(), cutter(), tol()).unwrap();
    s.validate(tol()).unwrap();
    close(
        s.volume().unwrap(),
        80.0 * 60.0 * 24.0 - PI * 14.0 * 14.0 * 24.0,
    );
    assert_eq!((s.edges.len(), s.shell.faces.len()), (15, 7));
    assert_eq!(s.shell.faces[0].wires.len(), 2);
    assert_eq!(s.shell.faces[1].wires.len(), 2);
    assert_eq!(s.shell.faces[6].orientation, -1);
}
#[test]
fn offset_hole_and_translation() {
    let mut b = block();
    b.min = b.min + Vec3::new(100.0, 50.0, 25.0);
    let mut c = cutter();
    c.base = c.base + Vec3::new(105.0, 47.0, 25.0);
    let s = subtract_through_cylinder(b, c, tol()).unwrap();
    close(
        s.volume().unwrap(),
        80.0 * 60.0 * 24.0 - PI * 14.0 * 14.0 * 24.0,
    );
    let m = s.tessellate(0.01, tol()).unwrap();
    assert!((m.signed_volume() - s.volume().unwrap()).abs() < 30.0);
    mesh_closed(&m);
}
#[test]
fn partial_and_cap_contact_rejected() {
    for (z, h) in [
        (-12.0, 40.0),
        (-10.0, 30.0),
        (-20.0, 32.0),
        (-20.0, 10.0),
        (30.0, 40.0),
        (-12.0 - 0.5e-8, 40.0),
    ] {
        let mut c = cutter();
        c.base.z = z;
        c.height = h;
        assert!(matches!(
            subtract_through_cylinder(block(), c, tol()),
            Err(Error::Unsupported(_))
        ));
    }
}
#[test]
fn side_contact_and_near_contact_rejected() {
    for x in [26.0, 26.0 - 0.5e-8, 27.0, 60.0] {
        let mut c = cutter();
        c.base.x = x;
        assert!(matches!(
            subtract_through_cylinder(block(), c, tol()),
            Err(Error::Unsupported(_))
        ));
    }
}
#[test]
fn invalid_dimensions() {
    for v in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-10] {
        let mut b = block();
        b.size.x = v;
        assert!(make_box(b, tol()).is_err());
        let mut c = cutter();
        c.radius = v;
        assert!(make_cylinder(c, tol()).is_err());
        c = cutter();
        c.height = v;
        assert!(make_cylinder(c, tol()).is_err());
    }
}
#[test]
fn small_dimensions_with_explicit_tolerance() {
    let t = Tolerance::new(1e-12).unwrap();
    let b = BoxSpec {
        min: Point3::new(0.0, 0.0, 0.0),
        size: Vec3::new(1e-5, 1e-5, 1e-5),
    };
    let c = CylinderSpec {
        base: Point3::new(5e-6, 5e-6, -1e-6),
        radius: 1e-6,
        height: 1.2e-5,
    };
    let s = subtract_through_cylinder(b, c, t).unwrap();
    assert!((s.volume().unwrap() - (1e-15 - PI * 1e-12 * 1e-5)).abs() < 1e-28);
    s.tessellate(1e-8, t).unwrap();
}
#[test]
fn malformed_topology_rejected() {
    let s = make_box(block(), tol()).unwrap();
    let mut x = s.clone();
    x.edges[0].vertices[1] = 99;
    assert!(x.validate(tol()).is_err());
    let mut x = s.clone();
    x.shell.faces.pop();
    assert!(x.validate(tol()).is_err());
    let mut x = s.clone();
    x.shell.faces[0].orientation = 1;
    assert!(x.validate(tol()).is_err());
    let mut x = s.clone();
    x.shell.faces[0].wires[0].coedges[0].forward = false;
    assert!(x.validate(tol()).is_err());
    let mut x = s;
    x.shell.faces[0].wires[0].coedges[0].pcurve = PCurve::Affine {
        origin: [2.0, 3.0],
        direction: [0.0, 0.0],
    };
    assert!(x.validate(tol()).is_err());
}
#[test]
fn tessellation_closed_and_volume_converges() {
    for s in [
        make_box(block(), tol()).unwrap(),
        make_cylinder(cutter(), tol()).unwrap(),
        subtract_through_cylinder(block(), cutter(), tol()).unwrap(),
    ] {
        let coarse = s.tessellate(0.1, tol()).unwrap();
        let fine = s.tessellate(0.001, tol()).unwrap();
        mesh_closed(&fine);
        let exact = s.volume().unwrap();
        let e = (fine.signed_volume() - exact).abs();
        assert!(e < (coarse.signed_volume() - exact).abs() + 1e-8);
        assert!(e / exact < 0.0003);
        for (tri, &fi) in fine.triangles.iter().zip(&fine.face_ids) {
            let p = tri.map(|i| fine.positions[i]);
            let cross = (p[1] - p[0]).cross(p[2] - p[0]);
            assert!(cross.norm() > 1e-12);
            assert!(cross.dot(fine.normals[tri[0]]) > 0.0);
            assert!(fi < s.shell.faces.len());
        }
    }
}
fn mesh_closed(m: &Mesh) {
    use std::collections::BTreeMap;
    let key = |p: Point3| {
        [
            (p.x * 1e7).round() as i64,
            (p.y * 1e7).round() as i64,
            (p.z * 1e7).round() as i64,
        ]
    };
    let mut edges = BTreeMap::new();
    for tri in &m.triangles {
        for i in 0..3 {
            let a = key(m.positions[tri[i]]);
            let b = key(m.positions[tri[(i + 1) % 3]]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let entry = edges.entry(k).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += sign;
        }
    }
    for (n, sum) in edges.values() {
        assert_eq!(*n, 2);
        assert_eq!(*sum, 0);
    }
}
#[test]
fn chord_error_bound() {
    for radius in [1e-5, 1.0, 14.0, 1000.0] {
        for error in [radius * 0.01, radius * 0.0001] {
            let n = circle_segments(radius, error).unwrap();
            let sag = radius * (1.0 - (TAU / n as f64 / 2.0).cos());
            assert!(sag <= error * (1.0 + 1e-10));
        }
    }
    let s = subtract_through_cylinder(block(), cutter(), tol()).unwrap();
    let m = s.tessellate(0.02, tol()).unwrap();
    for (tri, &face) in m.triangles.iter().zip(&m.face_ids) {
        if face != 6 {
            continue;
        }
        let a = m.positions[tri[0]];
        let b = m.positions[tri[1]];
        let mid = (a + b) * 0.5;
        assert!(14.0 - mid.x.hypot(mid.y) <= 0.02 + 1e-10);
    }
}
#[test]
fn tessellation_bad_requests() {
    for e in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-20] {
        assert!(make_cylinder(cutter(), tol())
            .unwrap()
            .tessellate(e, tol())
            .is_err());
    }
}
#[test]
fn geometry_and_transform() {
    let curve = Curve::Circle {
        center: Point3::new(1.0, 2.0, 3.0),
        radius: 4.0,
    };
    assert!(tol().coincident(curve.evaluate(PI / 2.0), Point3::new(1.0, 6.0, 3.0)));
    let plane = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 2.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    assert_eq!(
        line_plane(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), &plane)
            .unwrap()
            .z,
        2.0
    );
    assert!(line_plane(Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), &plane).is_err());
    let tr = Transform::new(
        Point3::new(1.0, 2.0, 3.0),
        [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        tol(),
    )
    .unwrap();
    assert_eq!(
        tr.point(Point3::new(2.0, 0.0, 0.0)),
        Point3::new(1.0, 4.0, 3.0)
    );
}
#[test]
fn extrusion() {
    let s = extrude(
        Profile::Rectangle {
            origin: block().min,
            width: 80.0,
            depth: 60.0,
        },
        24.0,
        tol(),
    )
    .unwrap();
    close(s.volume().unwrap(), 115200.0);
    let s = extrude(
        Profile::Disk {
            center: cutter().base,
            radius: 14.0,
        },
        40.0,
        tol(),
    )
    .unwrap();
    close(s.volume().unwrap(), PI * 196.0 * 40.0);
    assert!(extrude(
        Profile::Disk {
            center: cutter().base,
            radius: 14.0
        },
        -2.0,
        tol()
    )
    .is_err());
}
#[test]
fn invalid_tolerance() {
    for v in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Tolerance::new(v).is_err());
        assert!(make_box(block(), Tolerance { linear: v }).is_err());
    }
}

#[test]
fn cylinder_plane_intersections() {
    let c = Surface::Cylinder {
        center: Point3::new(3.0, 4.0, -20.0),
        radius: 14.0,
        height: 40.0,
    };
    let plane = |z| Surface::Plane {
        origin: Point3::new(0.0, 0.0, z),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    for z in [-20.0, -12.0, 12.0, 20.0] {
        let Curve::Circle { center, radius } = cylinder_plane(&c, &plane(z), tol()).unwrap() else {
            panic!("expected circle")
        };
        assert_eq!(center, Point3::new(3.0, 4.0, z));
        assert_eq!(radius, 14.0);
    }
    assert!(matches!(
        cylinder_plane(&c, &plane(21.0), tol()),
        Err(Error::Unsupported(_))
    ));
    let tilted = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 0.0, 1.0),
    };
    assert!(matches!(
        cylinder_plane(&c, &tilted, tol()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn line_intersection_invalid_inputs() {
    let plane = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    for direction in [Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, f64::NAN)] {
        assert!(matches!(
            line_plane(Point3::new(1.0, 2.0, 3.0), direction, &plane),
            Err(Error::InvalidInput(_))
        ));
    }
}
#[test]
fn topology_genus_and_cut_bounds() {
    let s = subtract_through_cylinder(block(), cutter(), tol()).unwrap();
    let face_chi: isize = s
        .shell
        .faces
        .iter()
        .map(|f| 2 - f.wires.len() as isize)
        .sum();
    assert_eq!(
        s.vertices.len() as isize - s.edges.len() as isize + face_chi,
        0
    );
    assert_eq!(s.bounds(), make_box(block(), tol()).unwrap().bounds());
    let m = s.tessellate(0.01, tol()).unwrap();
    for (tri, face) in m.triangles.iter().zip(&m.face_ids) {
        if *face > 1 {
            continue;
        }
        // No triangle crosses the circular trim's polygonal interior.
        let p = tri.map(|i| m.positions[i]);
        let center = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
        assert!(center.x.hypot(center.y) >= 14.0 - 0.01 - 1e-10);
    }
}
#[test]
fn far_origin_loses_small_features_explicitly() {
    let b = BoxSpec {
        min: Point3::new(1e20, 1e20, 1e20),
        size: Vec3::new(1.0, 1.0, 1.0),
    };
    assert!(make_box(b, tol()).is_err());
}
