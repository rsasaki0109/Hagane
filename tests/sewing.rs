use hagane::*;
fn box_solid() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 6., 2.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
fn check(s: &Solid, r: &Solid, t: Tolerance) {
    r.validate(t).unwrap();
    assert!((r.volume().unwrap() - s.volume().unwrap()).abs() < s.volume().unwrap() * 1e-12);
    assert_eq!(r.bounds(), s.bounds());
    let m = r.tessellate(0.01, t).unwrap();
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
#[test]
fn independent_box_faces_sew_with_permuted_order() {
    let s = box_solid();
    let mut p = planar_face_patches(&s, Tolerance::default()).unwrap();
    p.reverse();
    let r = sew_planar_faces(&p, GeometryTolerance::default()).unwrap();
    check(&s, &r, Tolerance::default());
    assert_eq!(r.vertices.len(), 8);
    assert_eq!(r.edges.len(), 12);
}
#[test]
fn mismatched_boundary_subdivisions_are_shared_in_both_faces() {
    let s = box_solid();
    let mut p = planar_face_patches(&s, Tolerance::default()).unwrap();
    let ring = &mut p[0].rings[0];
    let a = ring[0];
    let b = ring[1];
    ring.insert(1, a + (b - a) * 0.25);
    ring.insert(2, a + (b - a) * 0.75);
    let r = sew_planar_faces(&p, GeometryTolerance::default()).unwrap();
    check(&s, &r, Tolerance::default());
    assert_eq!(r.vertices.len(), 10);
    assert_eq!(r.edges.len(), 14);
    assert_eq!(r.shell.faces.len(), 6);
}
#[test]
fn concavity_holes_skew_and_rigid_placement_preserve_orientation() {
    let t = Tolerance::default();
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 3.], [4., 3.], [4., 8.], [0., 8.]],
            holes: vec![vec![[1., 1.], [2., 1.], [2., 6.], [1., 6.]]],
        },
        Vec3::new(2., -3., 5.),
        t,
    )
    .unwrap();
    for s in [
        s.clone(),
        s.transformed(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(), t)
            .unwrap(),
    ] {
        let p = planar_face_patches(&s, t).unwrap();
        let r = sew_planar_faces(&p, GeometryTolerance::default()).unwrap();
        check(&s, &r, t);
    }
}
#[test]
fn tiny_sewing_uses_explicit_tolerance() {
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8e-6, 6e-6, 2e-6),
        },
        t.absolute(),
    )
    .unwrap();
    let p = planar_face_patches(&s, t.absolute()).unwrap();
    let r = sew_planar_faces(&p, t).unwrap();
    r.validate(t.absolute()).unwrap();
    assert!((r.volume().unwrap() - 96e-18).abs() < 1e-29);
}
#[test]
fn open_duplicate_reversed_and_disconnected_shells_are_rejected() {
    let t = GeometryTolerance::default();
    let p = planar_face_patches(&box_solid(), t.absolute()).unwrap();
    let mut cases = vec![p[..5].to_vec()];
    let mut duplicate = p.clone();
    duplicate.push(p[0].clone());
    cases.push(duplicate);
    let mut reversed = p.clone();
    reversed[0].orientation *= -1;
    cases.push(reversed);
    let other = box_solid()
        .transformed(
            Transform::translation(Vec3::new(20., 0., 0.)).unwrap(),
            t.absolute(),
        )
        .unwrap();
    let mut disconnected = p.clone();
    disconnected.extend(planar_face_patches(&other, t.absolute()).unwrap());
    cases.push(disconnected);
    for p in cases {
        assert!(sew_planar_faces(&p, t).is_err());
    }
}
#[test]
fn near_coincidences_near_junctions_and_bad_inputs_are_errors() {
    let t = GeometryTolerance::default();
    let p = planar_face_patches(&box_solid(), t.absolute()).unwrap();
    let mut near = p.clone();
    near[0].rings[0][0].x += 1e-9;
    assert!(sew_planar_faces(&near, t).is_err());
    let mut junction = p.clone();
    let ring = &mut junction[0].rings[0];
    let a = ring[0];
    let b = ring[1];
    let mut m = (a + b) * 0.5;
    m.y += 1e-9;
    ring.insert(1, m);
    assert!(sew_planar_faces(&junction, t).is_err());
    let mut nan = p.clone();
    nan[0].rings[0][0].x = f64::NAN;
    assert!(sew_planar_faces(&nan, t).is_err());
    assert!(sew_planar_faces(&[], t).is_err());
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 2.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(planar_face_patches(&s, t.absolute()).is_err());
}
