use hagane::*;
fn check(
    s: &Solid,
    face: usize,
    anchor: Point3,
    direction: Vec3,
    cuts: usize,
) -> PlanarFaceSubdivision {
    let t = GeometryTolerance::default();
    let r = subdivide_planar_face(s, face, anchor, direction, t).unwrap();
    assert_eq!(r.faces.len(), 2);
    assert_eq!(r.cut_edges.len(), cuts);
    r.solid.validate(t.absolute()).unwrap();
    assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < s.volume().unwrap() * 1e-12);
    assert!((r.solid.bounds().min - s.bounds().min).norm() < 1e-12);
    assert!((r.solid.bounds().max - s.bounds().max).norm() < 1e-12);
    let m = r.solid.tessellate(0.01, t.absolute()).unwrap();
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
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(m.normals[tri[0]]) > 0.);
        for i in 0..3 {
            if let Surface::FramedCylinder { frame, radius, .. } =
                r.solid.shell.faces[m.face_ids[ti]].surface
            {
                let q = frame.local_point((p[i] + p[(i + 1) % 3]) * 0.5);
                assert!(radius - q.x.hypot(q.y) <= 0.01 + 1e-10);
            }
            let a = key(p[i]);
            let b = key(p[(i + 1) % 3]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(k).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
    r
}
fn cylinder() -> Solid {
    make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4.,
            height: 2.,
        },
        Tolerance::default(),
    )
    .unwrap()
}
#[test]
fn periodic_disk_cuts_include_original_seam_and_diameter() {
    let s = cylinder();
    for y in [0., 0.2, -0.2, 3.5] {
        for d in [1., -7.] {
            check(&s, 0, Point3::new(0., y, 0.), Vec3::new(d, 0., 0.), 1);
        }
    }
    check(&s, 1, Point3::new(0., 0., 2.), Vec3::new(0., 1., 0.), 1);
    assert_eq!(s.shell.faces.len(), 3);
    assert!(matches!(s.edges[0].curve, Curve::Circle { .. }));
}
#[test]
fn periodic_annulus_and_opposite_cap_cut() {
    let s = make_tube(
        TubeSpec {
            base: Point3::new(0., 0., 0.),
            outer_radius: 4.,
            inner_radius: 2.,
            height: 2.,
        },
        Tolerance::default(),
    )
    .unwrap();
    let r = check(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), 2);
    assert!(r
        .faces
        .iter()
        .all(|&i| r.solid.shell.faces[i].wires.len() == 1));
    check(
        &r.solid,
        1,
        Point3::new(0., 0.7, 2.),
        Vec3::new(-1., 0., 0.),
        2,
    );
    // Uncut inner periodic wires keep one owner, including mixed planar trims.
    let r = check(&s, 0, Point3::new(0., 3., 0.), Vec3::new(1., 0., 0.), 1);
    assert_eq!(
        r.faces
            .iter()
            .map(|&i| r.solid.shell.faces[i].wires.len())
            .sum::<usize>(),
        3
    );
}
#[test]
fn two_periodic_bores_and_straight_outer_edges_share_exact_cuts() {
    let s = subtract_through_cylinders(
        BoxSpec {
            min: Point3::new(-6., -4., 0.),
            size: Vec3::new(12., 8., 2.),
        },
        &[-2., 2.].map(|x| CylinderSpec {
            base: Point3::new(x, 0., -1.),
            radius: 1.,
            height: 4.,
        }),
        Tolerance::default(),
    )
    .unwrap();
    let r = check(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), 3);
    assert!(r
        .faces
        .iter()
        .all(|&i| r.solid.shell.faces[i].wires.len() == 1));
    assert!((r.solid.volume().unwrap() - (192. - 4. * std::f64::consts::PI)).abs() < 1e-10);
}
#[test]
fn periodic_refinement_is_frame_aware_and_scale_aware() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let s = cylinder().transformed(tr, t.absolute()).unwrap();
    check(
        &s,
        0,
        tr.point(Point3::new(0., 0., 0.)),
        tr.vector(Vec3::new(1., 0., 0.)),
        1,
    );
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4e-6,
            height: 2e-6,
        },
        t.absolute(),
    )
    .unwrap();
    let r =
        subdivide_planar_face(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1e300, 0., 0.), t).unwrap();
    r.solid.validate(t.absolute()).unwrap();
    assert!((r.solid.volume().unwrap() - 32e-18 * std::f64::consts::PI).abs() < 1e-29);
    r.solid.tessellate(1e-8, t.absolute()).unwrap();
}
#[test]
fn contacts_bad_rims_and_invalid_inputs_fail_without_mutation() {
    let s = cylinder();
    let t = GeometryTolerance::default();
    for y in [4., 4. - 1e-9, 5., f64::NAN] {
        assert!(
            subdivide_planar_face(&s, 0, Point3::new(0., y, 0.), Vec3::new(1., 0., 0.), t).is_err()
        );
    }
    let mut s = s.clone();
    if let Curve::Circle { radius, .. } = &mut s.edges[0].curve {
        *radius += 1e-12;
    }
    s.validate(t.absolute()).unwrap();
    assert!(
        subdivide_planar_face(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), t).is_err()
    );
    assert!(matches!(s.edges[0].curve, Curve::Circle { .. }));
    assert_eq!(s.shell.faces.len(), 3);
}
