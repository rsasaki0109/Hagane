use hagane::*;
fn box_at(min: Point3, size: Vec3) -> Solid {
    make_box(BoxSpec { min, size }, Tolerance::default()).unwrap()
}
fn common(a: &Solid, b: &Solid) -> Solid {
    match intersect_convex_solids(a, b, GeometryTolerance::default()).unwrap() {
        SolidIntersection::Solid(s) => s,
        SolidIntersection::Empty => panic!("expected common material"),
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
fn boxes_match_analytic_overlap_in_both_orders() {
    let a = box_at(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.));
    let b = box_at(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.));
    for (a, b) in [(&a, &b), (&b, &a)] {
        let s = common(a, b);
        check(&s);
        assert!((s.volume().unwrap() - 36.).abs() < 1e-10);
        assert_eq!(
            s.bounds(),
            Bounds {
                min: Point3::new(1., -1., -1.),
                max: Point3::new(4., 3., 2.)
            }
        );
        assert_eq!(
            classify_point_in_solid(&s, Point3::new(2., 0., 0.), GeometryTolerance::default())
                .unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn containment_and_empty_results_are_explicit() {
    let a = box_at(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.));
    let small = box_at(Point3::new(-1., -1., -1.), Vec3::new(2., 2., 2.));
    for (a, b) in [(&a, &small), (&small, &a)] {
        assert!((common(a, b).volume().unwrap() - 8.).abs() < 1e-10);
    }
    let far = box_at(Point3::new(20., 20., 20.), Vec3::new(2., 2., 2.));
    assert!(matches!(
        intersect_convex_solids(&a, &far, GeometryTolerance::default()).unwrap(),
        SolidIntersection::Empty
    ));
}
#[test]
fn contact_coplanar_curved_and_nonconvex_inputs_are_rejected() {
    let t = GeometryTolerance::default();
    let a = box_at(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.));
    for x in [4., 4. - 1e-9] {
        let b = box_at(Point3::new(x, -1., -1.), Vec3::new(2., 2., 2.));
        assert!(intersect_convex_solids(&a, &b, t).is_err());
    }
    assert!(intersect_convex_solids(&a, &a, t).is_err());
    let cylinder = make_cylinder(
        CylinderSpec {
            base: Point3::new(100., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(intersect_convex_solids(&a, &cylinder, t).is_err());
    let nonconvex = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 3.], [4., 3.], [4., 8.], [0., 8.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 5.),
        t.absolute(),
    )
    .unwrap();
    assert!(intersect_convex_solids(&a, &nonconvex, t).is_err());
}
#[test]
fn rotated_diamond_intersection_matches_independent_area_formula() {
    let t = GeometryTolerance::default();
    let a = box_at(Point3::new(-40., -30., -12.), Vec3::new(80., 60., 24.));
    let rotation = Transform::rotation(Vec3::new(0., 0., 1.), std::f64::consts::FRAC_PI_4).unwrap();
    for offset in [-8., -2., 0., 8.] {
        let b = box_at(Point3::new(-32., -32., -20.), Vec3::new(64., 64., 40.))
            .transformed(
                Transform::translation(Vec3::new(offset, 0., 0.))
                    .unwrap()
                    .compose(rotation)
                    .unwrap(),
                t.absolute(),
            )
            .unwrap();
        let s = common(&a, &b);
        check(&s);
        let d = 32. * 2f64.sqrt();
        let expected = (4096.
            - (d - 40. + offset).max(0.).powi(2)
            - (d - 40. - offset).max(0.).powi(2)
            - 2. * (d - 30.).powi(2))
            * 24.;
        assert!((s.volume().unwrap() - expected).abs() < 1e-8);
        for vertex in &s.vertices {
            assert!(
                classify_point_in_solid(&a, vertex.point, t).unwrap() != PointLocation::Outside
            );
            assert!(
                classify_point_in_solid(&b, vertex.point, t).unwrap() != PointLocation::Outside
            );
        }
    }
}
#[test]
fn placed_and_tiny_convex_intersections_preserve_volume() {
    let t = GeometryTolerance::default();
    let a = box_at(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.));
    let b = box_at(Point3::new(1., -1., -1.), Vec3::new(6., 6., 6.));
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let a = a.transformed(tr, t.absolute()).unwrap();
    let b = b.transformed(tr, t.absolute()).unwrap();
    let s = common(&a, &b);
    check(&s);
    assert!((s.volume().unwrap() - 36.).abs() < 1e-9);
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
    let SolidIntersection::Solid(s) = intersect_convex_solids(&a, &b, t).unwrap() else {
        panic!("expected common material")
    };
    s.validate(t.absolute()).unwrap();
    assert!((s.volume().unwrap() - 36e-18).abs() < 1e-29);
}
#[test]
fn triangular_prism_overlap_and_separation_with_overlapping_bounds() {
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
    let b = box_at(Point3::new(1., 1., 1.), Vec3::new(8., 8., 8.));
    for (a, b) in [(&a, &b), (&b, &a)] {
        let s = common(a, b);
        check(&s);
        assert!((s.volume().unwrap() - 24.).abs() < 1e-10);
    }
    let separated = box_at(Point3::new(4., 4., 1.), Vec3::new(1., 1., 1.));
    for (a, b) in [(&a, &separated), (&separated, &a)] {
        assert!(matches!(
            intersect_convex_solids(a, b, t).unwrap(),
            SolidIntersection::Empty
        ));
    }
}
#[test]
fn generated_vertex_contacts_and_invalid_operands_fail() {
    for offset in [32. * 2f64.sqrt() - 40., 70. - 32. * 2f64.sqrt(), f64::NAN] {
        assert!(convex_intersection_demo_json(offset).is_err());
    }
    let a = box_at(Point3::new(-4., -3., -2.), Vec3::new(8., 6., 4.));
    let mut b = a.clone();
    b.shell.faces.pop();
    assert!(intersect_convex_solids(&a, &b, GeometryTolerance::default()).is_err());
}
