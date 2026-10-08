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
fn arrangement(a: BoxSpec, b: BoxSpec, op: BoxBooleanOperation) -> Solid {
    let BoxBooleanResult::Solid(s) = boolean_boxes(a, b, op, GeometryTolerance::default()).unwrap()
    else {
        panic!("expected material")
    };
    s
}
fn simplify(s: &Solid) -> Solid {
    let t = GeometryTolerance::default();
    let r = simplify_straight_edges(s, t).unwrap();
    check(&r);
    assert_eq!(r.bounds(), s.bounds());
    assert_eq!(r.shell.faces.len(), s.shell.faces.len());
    assert!((r.volume().unwrap() - s.volume().unwrap()).abs() < 1e-9);
    let twice = simplify_straight_edges(&r, t).unwrap();
    assert_eq!(twice.edges.len(), r.edges.len());
    assert_eq!(twice.vertices.len(), r.vertices.len());
    r
}
#[test]
fn merged_box_has_minimal_straight_topology_and_preserves_classification() {
    let s = arrangement(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    let merged = merge_coplanar_faces(&s, GeometryTolerance::default()).unwrap();
    assert!(merged.edges.len() > 12);
    let r = simplify(&merged);
    assert_eq!(
        (r.shell.faces.len(), r.edges.len(), r.vertices.len()),
        (6, 12, 8)
    );
    for p in [
        Point3::new(4., 2., 2.),
        Point3::new(4., 0., 2.),
        Point3::new(9., 0., 0.),
    ] {
        assert_eq!(
            classify_point_in_solid(&r, p, GeometryTolerance::default()).unwrap(),
            classify_point_in_solid(&merged, p, GeometryTolerance::default()).unwrap()
        );
    }
}
#[test]
fn face_branches_and_feature_corners_are_preserved() {
    let s = arrangement(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    let r = simplify(&s);
    assert_eq!(r.edges.len(), s.edges.len());
    assert_eq!(r.vertices.len(), s.vertices.len());
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [4., 0.], [8., 1e-6], [8., 4.], [0., 4.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 3.),
        Tolerance::default(),
    )
    .unwrap();
    let r = simplify(&s);
    assert_eq!(r.edges.len(), s.edges.len());
    assert_eq!(r.vertices.len(), s.vertices.len());
}
#[test]
fn planar_holes_and_concave_contact_boundaries_remain_valid() {
    let s = arrangement(
        block([-4., -3., -2.], [8., 6., 4.]),
        block([-1., -1., -3.], [2., 2., 6.]),
        BoxBooleanOperation::Difference,
    );
    let s = merge_coplanar_faces(&s, GeometryTolerance::default()).unwrap();
    let r = simplify(&s);
    assert_eq!(
        (r.shell.faces.len(), r.edges.len(), r.vertices.len()),
        (10, 24, 16)
    );
    assert_eq!(
        r.shell.faces.iter().filter(|f| f.wires.len() == 2).count(),
        2
    );
    assert_eq!(
        classify_point_in_solid(&r, Point3::new(0., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    let s = arrangement(
        block([0.; 3], [4.; 3]),
        block([4., 1., 1.], [2.; 3]),
        BoxBooleanOperation::Union,
    );
    let r = simplify(&merge_coplanar_faces(&s, GeometryTolerance::default()).unwrap());
    assert_eq!(
        r.shell.faces.iter().filter(|f| f.wires.len() == 2).count(),
        1
    );
    assert_eq!(
        classify_point_in_solid(&r, Point3::new(4., 2., 2.), GeometryTolerance::default()).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn transformed_and_tiny_solids_preserve_geometry() {
    let t = GeometryTolerance::default();
    let s = arrangement(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    let s = merge_coplanar_faces(&s, t).unwrap();
    let placed = s
        .transformed(
            Transform::translation(Vec3::new(100., -200., 300.)).unwrap(),
            t.absolute(),
        )
        .unwrap();
    assert_eq!(simplify(&placed).edges.len(), 12);
    let tilted = s
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(),
            t.absolute(),
        )
        .unwrap();
    let r = simplify(&tilted);
    assert!(r.edges.len() <= tilted.edges.len());
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
    let s = merge_coplanar_faces(&s, t).unwrap();
    let r = simplify_straight_edges(&s, t).unwrap();
    r.validate(t.absolute()).unwrap();
    assert_eq!(r.edges.len(), 12);
    assert!((r.volume().unwrap() - 128e-18).abs() < 1e-28);
}
#[test]
fn curved_invalid_and_unchanged_inputs_are_distinguished() {
    let t = GeometryTolerance::default();
    let a = make_box(block([0.; 3], [4.; 3]), t.absolute()).unwrap();
    assert_eq!(simplify(&a).edges.len(), 12);
    let c = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 1.,
            height: 3.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(simplify_straight_edges(&c, t).is_err());
    let mut invalid = a.clone();
    invalid.vertices[0].point.x = f64::NAN;
    assert!(simplify_straight_edges(&invalid, t).is_err());
}
#[test]
fn reversed_uv_frames_preserve_normalized_shared_edge_parameters() {
    let t = GeometryTolerance::default();
    let s = arrangement(
        block([0.; 3], [4.; 3]),
        block([4., 0., 0.], [4.; 3]),
        BoxBooleanOperation::Union,
    );
    let mut s = merge_coplanar_faces(&s, t).unwrap();
    for (i, f) in s.shell.faces.iter_mut().enumerate() {
        if i % 2 == 0 {
            continue;
        }
        let Surface::Plane { origin, u, v } = f.surface else {
            unreachable!()
        };
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
    }
    s.validate(t.absolute()).unwrap();
    let r = simplify(&s);
    assert_eq!(r.edges.len(), 12);
    for f in &r.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            for parameter in [0., 0.25, 0.5, 0.75, 1.] {
                let uv = c.pcurve.evaluate(parameter);
                let p = f.surface.evaluate(uv[0], uv[1]);
                let q = r.edges[c.edge].curve.evaluate(parameter);
                assert!((p - q).norm() < 1e-10);
            }
        }
    }
}
