use hagane::*;
fn block(min: Point3, size: Vec3) -> Solid {
    make_box(BoxSpec { min, size }, Tolerance::default()).unwrap()
}
fn source() -> Solid {
    block(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.))
}
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
#[test]
fn overlapping_boxes_form_one_nonconvex_solid_in_either_order() {
    let a = source();
    let b = block(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.));
    for (first, second) in [(&a, &b), (&b, &a)] {
        let s = union_convex_solids(first, second, GeometryTolerance::default()).unwrap();
        check(&s);
        assert!((s.volume().unwrap() - 372.).abs() < 1e-10);
        assert_eq!(s.bounds().min, Point3::new(-4., -3., -2.));
        assert_eq!(s.bounds().max, Point3::new(7., 5., 5.));
        for p in [
            Point3::new(-2., 0., 0.),
            Point3::new(6., 4., 4.),
            Point3::new(2., 0., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(4., 0., 0.),
        ] {
            assert_eq!(
                classify_point_in_solid(&s, p, GeometryTolerance::default()).unwrap(),
                PointLocation::Inside
            );
        }
        assert_eq!(
            classify_point_in_solid(&s, Point3::new(-2., 4., 0.), GeometryTolerance::default())
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn containment_returns_enclosing_operand_in_both_orders() {
    let a = source();
    let b = block(Point3::new(-1., -1., -1.), Vec3::new(2., 2., 2.));
    for (first, second) in [(&a, &b), (&b, &a)] {
        let s = union_convex_solids(first, second, GeometryTolerance::default()).unwrap();
        check(&s);
        assert_eq!(s.volume().unwrap(), 192.);
        assert_eq!(s.bounds(), a.bounds());
    }
}
#[test]
fn disjoint_contact_coplanar_and_curved_operands_fail() {
    let t = GeometryTolerance::default();
    let a = source();
    for x in [20., 4., 4. - 1e-9] {
        let b = block(Point3::new(x, -1., -1.), Vec3::new(2., 2., 2.));
        assert!(union_convex_solids(&a, &b, t).is_err());
        assert!(union_convex_solids(&b, &a, t).is_err());
    }
    assert!(union_convex_solids(&a, &a, t).is_err());
    let c = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -3.),
            radius: 1.,
            height: 6.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(union_convex_solids(&a, &c, t).is_err());
    let mut invalid = a.clone();
    invalid.vertices[0].point.x = f64::NAN;
    assert!(union_convex_solids(&invalid, &a, t).is_err());
    let nonconvex = union_convex_solids(
        &a,
        &block(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.)),
        t,
    )
    .unwrap();
    assert!(union_convex_solids(&nonconvex, &a, t).is_err());
}
#[test]
fn placement_and_small_dimensions_conserve_volume() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let a = source().transformed(tr, t.absolute()).unwrap();
    let b = block(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.))
        .transformed(tr, t.absolute())
        .unwrap();
    let s = union_convex_solids(&a, &b, t).unwrap();
    check(&s);
    assert!((s.volume().unwrap() - 372.).abs() < 1e-9);
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let a = make_box(
        BoxSpec {
            min: Point3::new(-4e-6, -3e-6, -2e-6),
            size: Vec3::new(8e-6, 6e-6, 4e-6),
        },
        t.absolute(),
    )
    .unwrap();
    let b = make_box(
        BoxSpec {
            min: Point3::new(1e-6, -1e-6, -1e-6),
            size: Vec3::new(6e-6, 6e-6, 6e-6),
        },
        t.absolute(),
    )
    .unwrap();
    let s = union_convex_solids(&a, &b, t).unwrap();
    s.validate(t.absolute()).unwrap();
    assert!((s.volume().unwrap() - 372e-18).abs() < 1e-28);
}
#[test]
fn triangular_prism_union_uses_analytic_overlap() {
    let t = GeometryTolerance::default();
    let a = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [6., 0.], [0., 6.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 4.),
        t.absolute(),
    )
    .unwrap();
    let b = block(Point3::new(1., 1., 1.), Vec3::new(8., 8., 8.));
    let s = union_convex_solids(&a, &b, t).unwrap();
    check(&s);
    assert!((s.volume().unwrap() - 560.).abs() < 1e-10);
}
