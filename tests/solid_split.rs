use hagane::*;
fn plane(z: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    }
}
fn cube() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-4., -3., -2.),
            size: Vec3::new(8., 6., 4.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
fn check(s: &Solid, r: &SolidPlaneSplit, t: Tolerance) {
    r.negative.validate(t).unwrap();
    r.positive.validate(t).unwrap();
    assert!(
        (r.negative.volume().unwrap() + r.positive.volume().unwrap() - s.volume().unwrap()).abs()
            < s.volume().unwrap() * 1e-10
    );
    for s in [&r.negative, &r.positive] {
        let m = s.tessellate(0.01, t).unwrap();
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
                let (k, sgn) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let u = uses.entry(k).or_insert((0, 0));
                u.0 += 1;
                u.1 += sgn;
            }
        }
        assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
    }
}
#[test]
fn horizontal_box_partition_conserves_volume_and_closes_caps() {
    let t = GeometryTolerance::default();
    let s = cube();
    let r = split_solid_by_plane(&s, &plane(0.5), t).unwrap();
    check(&s, &r, t.absolute());
    assert_eq!(r.section.len(), 1);
    assert!((r.negative.volume().unwrap() - 120.).abs() < 1e-10);
    assert!((r.positive.volume().unwrap() - 72.).abs() < 1e-10);
    assert_eq!(r.negative.bounds().max.z, 0.5);
    assert_eq!(r.positive.bounds().min.z, 0.5);
    assert_eq!(s.shell.faces.len(), 6);
}
#[test]
fn oblique_plane_and_reversed_normal_preserve_complementary_regions() {
    let t = GeometryTolerance::default();
    let s = cube();
    let frame = Frame3::new(
        Point3::new(0., 0., 0.),
        [
            Vec3::new(1., 0., -1.).normalized().unwrap(),
            Vec3::new(0., 1., 0.),
            Vec3::new(1., 0., 1.).normalized().unwrap(),
        ],
        t.absolute(),
    )
    .unwrap();
    let [u, v, _] = frame.axes();
    let p = Surface::Plane {
        origin: frame.origin(),
        u,
        v,
    };
    let r = split_solid_by_plane(&s, &p, t).unwrap();
    check(&s, &r, t.absolute());
    assert!((r.negative.volume().unwrap() - 96.).abs() < 1e-10);
    let reversed = Surface::Plane {
        origin: frame.origin(),
        u,
        v: v * -1.,
    };
    let rr = split_solid_by_plane(&s, &reversed, t).unwrap();
    check(&s, &rr, t.absolute());
    assert_eq!(
        classify_point_in_solid(&r.positive, Point3::new(2., 0., 0.), t).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        classify_point_in_solid(&rr.negative, Point3::new(2., 0., 0.), t).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn concave_hollow_section_keeps_exact_hole_and_skew() {
    let t = GeometryTolerance::default();
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 3.], [4., 3.], [4., 8.], [0., 8.]],
            holes: vec![vec![[1., 1.], [2., 1.], [2., 6.], [1., 6.]]],
        },
        Vec3::new(2., -3., 5.),
        t.absolute(),
    )
    .unwrap();
    let r = split_solid_by_plane(&s, &plane(2.), t).unwrap();
    check(&s, &r, t.absolute());
    assert_eq!(r.section.len(), 1);
    assert_eq!(r.section[0].rings.len(), 2);
    assert!((r.negative.volume().unwrap() - 78.).abs() < 1e-9);
    assert!((r.positive.volume().unwrap() - 117.).abs() < 1e-9);
    assert_eq!(
        classify_point_in_solid(&r.negative, Point3::new(1.9, 1.4, 1.), t).unwrap(),
        PointLocation::Outside
    );
}
#[test]
fn contact_disjoint_curved_and_disconnected_results_are_rejected() {
    let t = GeometryTolerance::default();
    let s = cube();
    for z in [-2., -2. + 1e-9, 2., 3.] {
        assert!(split_solid_by_plane(&s, &plane(z), t).is_err());
    }
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(split_solid_by_plane(&s, &plane(2.), t).is_err());
    let mut p = plane(0.);
    if let Surface::Plane { origin, .. } = &mut p {
        origin.x = f64::NAN;
    }
    assert!(split_solid_by_plane(&cube(), &p, t).is_err());
}
#[test]
fn disconnected_kept_material_is_explicitly_rejected() {
    let t = GeometryTolerance::default();
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![
                [-4., -3.],
                [4., -3.],
                [4., 3.],
                [2., 3.],
                [2., -1.],
                [-2., -1.],
                [-2., 3.],
                [-4., 3.],
            ],
            holes: vec![],
        },
        Vec3::new(0., 0., 2.),
        t.absolute(),
    )
    .unwrap();
    let p = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 0., -1.),
    };
    assert!(split_solid_by_plane(&s, &p, t).is_err());
}
#[test]
fn rigid_placement_and_microscopic_partition() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let s = cube().transformed(tr, t.absolute()).unwrap();
    let p = plane(0.5).transformed(tr).unwrap();
    let r = split_solid_by_plane(&s, &p, t).unwrap();
    check(&s, &r, t.absolute());
    assert!((r.negative.volume().unwrap() - 120.).abs() < 1e-9);
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8e-6, 6e-6, 4e-6),
        },
        t.absolute(),
    )
    .unwrap();
    let r = split_solid_by_plane(&s, &plane(1e-6), t).unwrap();
    r.negative.validate(t.absolute()).unwrap();
    r.positive.validate(t.absolute()).unwrap();
    assert!((r.negative.volume().unwrap() - 48e-18).abs() < 1e-29);
    r.negative.tessellate(1e-8, t.absolute()).unwrap();
    r.positive.tessellate(1e-8, t.absolute()).unwrap();
}
