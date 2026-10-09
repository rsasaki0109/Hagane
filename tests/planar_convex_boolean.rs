use hagane::*;
fn stock(t: Tolerance) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-40., -30., -12.),
            size: Vec3::new(80., 60., 24.),
        },
        t,
    )
    .unwrap()
}
fn tool(x: f64, t: Tolerance) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(x, -5., -20.),
            size: Vec3::new(10., 10., 40.),
        },
        t,
    )
    .unwrap()
}
#[test]
fn repeated_difference_preserves_existing_hole_and_exact_common_volume() {
    for scale in [1e-6, 1., 1000.] {
        let policy = GeometryTolerance::new(1e-8 * scale, 1e-10, 0.).unwrap();
        let t = policy.absolute();
        let outer = [[-40., -30.], [40., -30.], [40., 30.], [-40., 30.]]
            .map(|p| [p[0] * scale, p[1] * scale]);
        let hole = vec![
            [-10. * scale, -8. * scale],
            [10. * scale, -8. * scale],
            [10. * scale, 8. * scale],
            [-10. * scale, 8. * scale],
        ];
        let subject = extrude_polygon(
            &PolygonProfile {
                origin: Point3::new(0., 0., -12. * scale),
                outer: outer.to_vec(),
                holes: vec![hole],
            },
            Vec3::new(0., 0., 24. * scale),
            t,
        )
        .unwrap();
        let cutter = make_box(
            BoxSpec {
                min: Point3::new(20. * scale, -5. * scale, -20. * scale),
                size: Vec3::new(10. * scale, 10. * scale, 40. * scale),
            },
            t,
        )
        .unwrap();
        assert!(subtract_convex_solids(&subject, &cutter, policy).is_err());
        let SolidDifference::Solid(result) =
            subtract_convex_from_planar_solid(&subject, &cutter, policy).unwrap()
        else {
            panic!("expected retained solid")
        };
        result.validate(t).unwrap();
        let expected = 105120. * scale.powi(3);
        assert!((result.volume().unwrap() - expected).abs() < expected * 1e-10);
        let SolidIntersection::Solid(common) =
            intersect_planar_solid_with_convex(&subject, &cutter, policy).unwrap()
        else {
            panic!("expected common solid")
        };
        assert!((common.volume().unwrap() - 2400. * scale.powi(3)).abs() < expected * 1e-12);
        for (p, location) in [
            (Point3::new(0., 0., 0.), PointLocation::Outside),
            (Point3::new(25., 0., 0.), PointLocation::Outside),
            (Point3::new(15., 0., 0.), PointLocation::Inside),
            (Point3::new(20., 0., 0.), PointLocation::Boundary),
        ] {
            assert_eq!(
                classify_point_in_solid(&result, p * scale, policy).unwrap(),
                location
            );
        }
        let mesh = result.tessellate(0.05 * scale, t).unwrap();
        assert!((mesh.signed_volume() - expected).abs() < expected * 1e-10);
        let key = |p: Point3| {
            [
                (p.x / (1e-9 * scale)).round() as i64,
                (p.y / (1e-9 * scale)).round() as i64,
                (p.z / (1e-9 * scale)).round() as i64,
            ]
        };
        let mut uses = std::collections::BTreeMap::new();
        for tri in &mesh.triangles {
            for i in 0..3 {
                let a = key(mesh.positions[tri[i]]);
                let b = key(mesh.positions[tri[(i + 1) % 3]]);
                let (e, s) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let entry = uses.entry(e).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += s;
            }
        }
        assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
        let frame = Transform::translation(Vec3::new(4., 9., -3.) * scale).unwrap();
        let placed = subtract_convex_from_planar_solid(
            &subject.transformed(frame, t).unwrap(),
            &cutter.transformed(frame, t).unwrap(),
            policy,
        )
        .unwrap();
        let SolidDifference::Solid(placed) = placed else {
            panic!()
        };
        assert!((placed.volume().unwrap() - expected).abs() < expected * 1e-10);
    }
}
#[test]
fn concave_subject_empty_unchanged_and_unsupported_inputs() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let subject = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -12.),
            outer: vec![
                [-40., -30.],
                [40., -30.],
                [40., 0.],
                [0., 0.],
                [0., 30.],
                [-40., 30.],
            ],
            holes: vec![],
        },
        Vec3::new(0., 0., 24.),
        t,
    )
    .unwrap();
    let cutter = tool(-25., t);
    let SolidDifference::Solid(result) =
        subtract_convex_from_planar_solid(&subject, &cutter, policy).unwrap()
    else {
        panic!()
    };
    assert!((result.volume().unwrap() - (3600. * 24. - 2400.)).abs() < 1e-8);
    assert!(matches!(
        intersect_planar_solid_with_convex(&subject, &tool(80., t), policy).unwrap(),
        SolidIntersection::Empty
    ));
    let SolidDifference::Solid(unchanged) =
        subtract_convex_from_planar_solid(&subject, &tool(80., t), policy).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        unchanged.mesh_json(0.05, t).unwrap(),
        subject.mesh_json(0.05, t).unwrap()
    );
    let enclosing = make_box(
        BoxSpec {
            min: Point3::new(-50., -40., -20.),
            size: Vec3::new(100., 80., 40.),
        },
        t,
    )
    .unwrap();
    assert!(matches!(
        subtract_convex_from_planar_solid(&subject, &enclosing, policy).unwrap(),
        SolidDifference::Empty
    ));
    assert!(subtract_convex_from_planar_solid(&subject, &subject, policy).is_err());
    let curved = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -20.),
            radius: 4.,
            height: 40.,
        },
        t,
    )
    .unwrap();
    assert!(subtract_convex_from_planar_solid(&curved, &stock(t), policy).is_err());
    assert!(subtract_convex_from_planar_solid(&subject, &curved, policy).is_err());
    // A contained cutter would need an independent inner shell (cavity).
    let internal = make_box(
        BoxSpec {
            min: Point3::new(-2., -3., -4.),
            size: Vec3::new(4., 6., 8.),
        },
        t,
    )
    .unwrap();
    assert!(subtract_convex_from_planar_solid(&stock(t), &internal, policy).is_err());
}
