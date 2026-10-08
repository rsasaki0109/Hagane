use hagane::*;
fn block(min: Point3, size: Vec3) -> Solid {
    make_box(BoxSpec { min, size }, Tolerance::default()).unwrap()
}
fn source() -> Solid {
    block(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.))
}
fn difference(a: &Solid, b: &Solid) -> Solid {
    match subtract_convex_solids(a, b, GeometryTolerance::default()).unwrap() {
        SolidDifference::Solid(s) => s,
        SolidDifference::Empty => panic!("expected retained material"),
    }
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
fn partial_box_subtraction_sews_one_nonconvex_boundary() {
    let a = source();
    let b = block(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.));
    let s = difference(&a, &b);
    check(&s);
    assert!((s.volume().unwrap() - 156.).abs() < 1e-10);
    assert_eq!(s.bounds(), a.bounds());
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(2., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(-2., 0., 0.), GeometryTolerance::default())
            .unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn through_box_hole_retains_closed_single_shell() {
    let a = source();
    let b = block(Point3::new(-1., -1., -3.), Vec3::new(2., 2., 6.));
    let s = difference(&a, &b);
    check(&s);
    assert!((s.volume().unwrap() - 176.).abs() < 1e-10);
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(0., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(1., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Boundary
    );
}
#[test]
fn empty_unchanged_and_unsupported_multishell_results_are_distinct() {
    let t = GeometryTolerance::default();
    let a = source();
    let covering = block(Point3::new(-10., -10., -10.), Vec3::new(20., 20., 20.));
    assert!(matches!(
        subtract_convex_solids(&a, &covering, t).unwrap(),
        SolidDifference::Empty
    ));
    let far = block(Point3::new(20., 20., 20.), Vec3::new(2., 2., 2.));
    assert_eq!(difference(&a, &far).volume().unwrap(), 192.);
    let cavity = block(Point3::new(-1., -1., -1.), Vec3::new(2., 2., 2.));
    assert!(subtract_convex_solids(&a, &cavity, t).is_err());
    let slab = block(Point3::new(-1., -5., -5.), Vec3::new(2., 10., 10.));
    assert!(subtract_convex_solids(&a, &slab, t).is_err());
    assert!(subtract_convex_solids(&a, &a, t).is_err());
}
#[test]
fn triangular_prism_subtraction_has_analytic_volume() {
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
    let s = difference(&a, &b);
    check(&s);
    assert!((s.volume().unwrap() - 48.).abs() < 1e-10);
}
#[test]
fn rigid_multi_plane_cut_and_tiny_hole_are_supported() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let a = source().transformed(tr, t.absolute()).unwrap();
    let b = block(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.))
        .transformed(tr, t.absolute())
        .unwrap();
    let s = difference(&a, &b);
    check(&s);
    assert!((s.volume().unwrap() - 156.).abs() < 1e-9);
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
            min: Point3::new(-1e-6, -1e-6, -3e-6),
            size: Vec3::new(2e-6, 2e-6, 6e-6),
        },
        t.absolute(),
    )
    .unwrap();
    let SolidDifference::Solid(s) = subtract_convex_solids(&a, &b, t).unwrap() else {
        panic!("expected retained material")
    };
    s.validate(t.absolute()).unwrap();
    assert!((s.volume().unwrap() - 176e-18).abs() < 1e-29);
}
#[test]
fn curve_nonconvex_and_near_contact_operands_are_rejected() {
    let t = GeometryTolerance::default();
    let a = source();
    let c = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -3.),
            radius: 1.,
            height: 6.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(subtract_convex_solids(&a, &c, t).is_err());
    let b = block(Point3::new(4. - 1e-9, -1., -1.), Vec3::new(2., 2., 2.));
    assert!(subtract_convex_solids(&a, &b, t).is_err());
    let result = difference(
        &a,
        &block(Point3::new(-1., -1., -3.), Vec3::new(2., 2., 6.)),
    );
    assert!(subtract_convex_solids(&result, &b, t).is_err());
}
