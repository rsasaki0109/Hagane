use hagane::*;
fn check(s: &Solid) {
    s.validate(Tolerance::default()).unwrap();
    let m = s.tessellate(0.01, Tolerance::default()).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for tri in &m.triangles {
        let p = tri.map(|i| m.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(m.normals[tri[0]]) > 0.);
        for i in 0..3 {
            let a = key(p[i]);
            let b = key(p[(i + 1) % 3]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(k).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
}
fn block(min: [f64; 3], size: [f64; 3]) -> BoxSpec {
    BoxSpec {
        min: Point3::new(min[0], min[1], min[2]),
        size: Vec3::new(size[0], size[1], size[2]),
    }
}
fn arranged(a: BoxSpec, b: BoxSpec, op: BoxBooleanOperation) -> Solid {
    let BoxBooleanResult::Solid(s) = boolean_boxes(a, b, op, GeometryTolerance::default()).unwrap()
    else {
        panic!("expected solid")
    };
    s
}
fn merge(s: &Solid) -> Solid {
    let r = merge_coplanar_faces(s, GeometryTolerance::default()).unwrap();
    check(&r);
    assert_eq!(r.bounds(), s.bounds());
    assert!((r.volume().unwrap() - s.volume().unwrap()).abs() < 1e-9);
    let twice = merge_coplanar_faces(&r, GeometryTolerance::default()).unwrap();
    assert_eq!(twice.shell.faces.len(), r.shell.faces.len());
    assert_eq!(twice.edges.len(), r.edges.len());
    r
}
#[test]
fn full_face_union_becomes_six_faces_without_changing_boundary() {
    let s = arranged(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    assert_eq!(s.shell.faces.len(), 10);
    let r = merge(&s);
    assert_eq!(r.shell.faces.len(), 6);
    for p in [
        Point3::new(4., 2., 2.),
        Point3::new(0., 2., 2.),
        Point3::new(9., 2., 2.),
    ] {
        assert_eq!(
            classify_point_in_solid(&r, p, GeometryTolerance::default()).unwrap(),
            classify_point_in_solid(&s, p, GeometryTolerance::default()).unwrap()
        );
    }
}
#[test]
fn partial_face_contact_creates_one_planar_face_with_a_hole() {
    let s = arranged(
        block([0.; 3], [4.; 3]),
        block([4., 1., 1.], [2.; 3]),
        BoxBooleanOperation::Union,
    );
    let r = merge(&s);
    assert_eq!(r.shell.faces.len(), 11);
    assert_eq!(
        r.shell.faces.iter().filter(|f| f.wires.len() == 2).count(),
        1
    );
    assert!(r.edges.len() < s.edges.len());
    for p in [
        Point3::new(4., 2., 2.),
        Point3::new(4., 0.5, 2.),
        Point3::new(5., 2., 2.),
    ] {
        assert_eq!(
            classify_point_in_solid(&r, p, GeometryTolerance::default()).unwrap(),
            classify_point_in_solid(&s, p, GeometryTolerance::default()).unwrap()
        );
    }
}
#[test]
fn through_hole_caps_and_disconnected_coplanar_regions() {
    let s = arranged(
        block([-4., -3., -2.], [8., 6., 4.]),
        block([-1., -1., -3.], [2., 2., 6.]),
        BoxBooleanOperation::Difference,
    );
    let r = merge(&s);
    assert_eq!(r.shell.faces.len(), 10);
    assert_eq!(
        r.shell.faces.iter().filter(|f| f.wires.len() == 2).count(),
        2
    );
    assert_eq!(
        classify_point_in_solid(&r, Point3::new(0., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    // A notch leaves two edge-disconnected regions on the same supporting plane.
    let s = arranged(
        block([0.; 3], [6.; 3]),
        block([2., -1., 3.], [2., 8., 4.]),
        BoxBooleanOperation::Difference,
    );
    let r = merge(&s);
    assert_eq!(r.shell.faces.len(), 10);
}
#[test]
fn identical_frames_merge_after_rigid_placement() {
    let t = GeometryTolerance::default();
    let a = make_box(block([-4., -3., -2.], [8., 6., 4.]), t.absolute()).unwrap();
    let b = make_box(block([1., -1., -1.], [6.; 3]), t.absolute()).unwrap();
    let s = union_convex_solids(&a, &b, t).unwrap();
    let before = s.shell.faces.len();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let placed = s.transformed(tr, t.absolute()).unwrap();
    let r = merge(&placed);
    assert!(r.shell.faces.len() < before);
}
#[test]
fn invalid_curved_and_tiny_inputs() {
    let t = GeometryTolerance::default();
    let a = make_box(block([0.; 3], [4.; 3]), t.absolute()).unwrap();
    let r = merge(&a);
    assert_eq!(r.shell.faces.len(), 6);
    let c = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 1.,
            height: 3.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(merge_coplanar_faces(&c, t).is_err());
    let mut invalid = a.clone();
    invalid.shell.faces[0].orientation *= -1;
    assert!(merge_coplanar_faces(&invalid, t).is_err());
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let BoxBooleanResult::Solid(s) = boolean_boxes(
        block([0.; 3], [4e-6; 3]),
        block([4e-6, 0., 0.], [4e-6; 3]),
        BoxBooleanOperation::Union,
        t,
    )
    .unwrap() else {
        panic!("expected material")
    };
    let r = merge_coplanar_faces(&s, t).unwrap();
    r.validate(t.absolute()).unwrap();
    assert_eq!(r.shell.faces.len(), 6);
    assert!((r.volume().unwrap() - 128e-18).abs() < 1e-28);
}
#[test]
fn nearly_parallel_feature_planes_remain_separate() {
    let t = GeometryTolerance::default();
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [4., 0.], [8., 1e-6], [8., 4.], [0., 4.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 3.),
        t.absolute(),
    )
    .unwrap();
    let r = merge(&s);
    assert_eq!(r.shell.faces.len(), 7);
}
#[test]
fn plane_identity_checks_different_origins_frames_and_near_planes() {
    let u = Vec3::new(0.6, 0.8, 0.);
    let v = Vec3::new(0., 0., 1.);
    let a = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u,
        v,
    };
    let b = Surface::Plane {
        origin: u * 4.,
        u: v,
        v: u * (-1.),
    };
    assert!(same_plane_support(&a, &b).unwrap());
    assert!(same_plane_support(&b, &a).unwrap());
    let near = Surface::Plane {
        origin: Vec3::new(0., 0., 1e-12),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let xy = Surface::Plane {
        origin: Vec3::new(0., 0., 0.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    assert!(!same_plane_support(&xy, &near).unwrap());
    let tilted = Surface::Plane {
        origin: Vec3::new(0., 0., 0.),
        u: Vec3::new(1., 0., 1e-12),
        v: Vec3::new(0., 1., 0.),
    };
    assert!(!same_plane_support(&xy, &tilted).unwrap());
    let invalid = Surface::Plane {
        origin: Vec3::new(0., 0., 0.),
        u: Vec3::new(0., 0., 0.),
        v,
    };
    assert!(same_plane_support(&a, &invalid).is_err());
}
#[test]
fn independently_reframed_tilted_faces_merge_and_preserve_pcurves() {
    let t = GeometryTolerance::default();
    let a = make_box(block([-4., -3., -2.], [8., 6., 4.]), t.absolute()).unwrap();
    let b = make_box(block([1., -1., -1.], [6.; 3]), t.absolute()).unwrap();
    let mut s = union_convex_solids(&a, &b, t)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(),
            t.absolute(),
        )
        .unwrap();
    let before = s.shell.faces.len();
    for (i, f) in s.shell.faces.iter_mut().enumerate() {
        if i % 2 == 0 {
            continue;
        }
        let Surface::Plane { origin, u, v } = f.surface else {
            unreachable!()
        };
        f.surface = Surface::Plane {
            origin,
            u: v,
            v: u * (-1.),
        };
        for c in f.wires.iter_mut().flat_map(|w| &mut w.coedges) {
            let PCurve::Affine { origin, direction } = c.pcurve else {
                unreachable!()
            };
            c.pcurve = PCurve::Affine {
                origin: [origin[1], -origin[0]],
                direction: [direction[1], -direction[0]],
            };
        }
    }
    s.validate(t.absolute()).unwrap();
    let r = merge(&s);
    assert!(r.shell.faces.len() < before);
}
#[test]
fn tilted_planes_with_distinct_origins_merge() {
    let t = GeometryTolerance::default();
    let s = arranged(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    let tr = Transform::new(
        Point3::new(0., 0., 0.),
        [
            Vec3::new(0.6, 0.8, 0.),
            Vec3::new(-0.8, 0.6, 0.),
            Vec3::new(0., 0., 1.),
        ],
        t.absolute(),
    )
    .unwrap();
    let placed = s.transformed(tr, t.absolute()).unwrap();
    let r = merge(&placed);
    assert_eq!(r.shell.faces.len(), 6);
}
#[test]
fn rotated_and_reflected_uv_parameterizations_preserve_material() {
    for reflected in [false, true] {
        let mut s = arranged(
            block([0.; 3], [4.; 3]),
            block([4., 0., 0.], [4.; 3]),
            BoxBooleanOperation::Union,
        );
        for (i, f) in s.shell.faces.iter_mut().enumerate() {
            if i % 2 == 0 {
                continue;
            }
            let Surface::Plane { origin, u, v } = f.surface else {
                unreachable!()
            };
            if reflected {
                f.surface = Surface::Plane { origin, u: v, v: u };
                f.orientation *= -1;
                for w in &mut f.wires {
                    w.coedges.reverse();
                    for c in &mut w.coedges {
                        c.forward = !c.forward;
                        let PCurve::Affine { origin, direction } = c.pcurve else {
                            unreachable!()
                        };
                        c.pcurve = PCurve::Affine {
                            origin: [origin[1], origin[0]],
                            direction: [direction[1], direction[0]],
                        };
                    }
                }
            } else {
                f.surface = Surface::Plane {
                    origin,
                    u: u * 0.6 + v * 0.8,
                    v: u * (-0.8) + v * 0.6,
                };
                let map = |p: [f64; 2]| [0.6 * p[0] + 0.8 * p[1], -0.8 * p[0] + 0.6 * p[1]];
                for c in f.wires.iter_mut().flat_map(|w| &mut w.coedges) {
                    let PCurve::Affine { origin, direction } = c.pcurve else {
                        unreachable!()
                    };
                    c.pcurve = PCurve::Affine {
                        origin: map(origin),
                        direction: map(direction),
                    };
                }
            }
        }
        s.validate(Tolerance::default()).unwrap();
        let r = merge(&s);
        assert_eq!(r.shell.faces.len(), 6);
    }
}
