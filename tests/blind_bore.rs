use hagane::*;
use std::f64::consts::PI;
fn block(scale: f64) -> BoxSpec {
    BoxSpec {
        min: Point3::new(-4. * scale, -3. * scale, -scale),
        size: Vec3::new(8. * scale, 6. * scale, 2. * scale),
    }
}
fn tool(scale: f64) -> CylinderSpec {
    CylinderSpec {
        base: Point3::new(0., 0., -0.5 * scale),
        radius: scale,
        height: 2. * scale,
    }
}
#[test]
fn blind_bores_have_real_floors_closed_topology_volume_and_material() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let tol = Tolerance::new(epsilon).unwrap();
        let policy = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        let b = block(scale);
        let c = tool(scale);
        let solid = subtract_blind_cylinder(b, c, tol).unwrap();
        solid.validate(tol).unwrap();
        assert_eq!(
            (
                solid.shell.faces.len(),
                solid.edges.len(),
                solid.vertices.len()
            ),
            (8, 15, 10)
        );
        let volume = (96. - PI * 1.5) * scale.powi(3);
        assert!((solid.volume().unwrap() - volume).abs() < 1e-10 * scale.powi(3));
        assert_eq!(solid.bounds(), make_box(b, tol).unwrap().bounds());
        for angle in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
            let placed = solid.transformed(transform, tol).unwrap();
            for (point, expected) in [
                (Point3::new(0., 0., 0.), PointLocation::Outside),
                (Point3::new(0., 0., -0.75), PointLocation::Inside),
                (Point3::new(0., 0., -0.5), PointLocation::Boundary),
                (Point3::new(1., 0., 0.), PointLocation::Boundary),
                (Point3::new(2., 0., 0.), PointLocation::Inside),
                (Point3::new(0., 0., -1.), PointLocation::Boundary),
            ] {
                assert_eq!(
                    classify_point_in_solid(&placed, transform.point(point * scale), policy)
                        .unwrap(),
                    expected
                );
            }
        }
        for (delta, expected) in [
            (-2. * epsilon, PointLocation::Inside),
            (2. * epsilon, PointLocation::Outside),
        ] {
            assert_eq!(
                classify_point_in_solid(&solid, Point3::new(0., 0., -0.5 * scale + delta), policy)
                    .unwrap(),
                expected
            );
        }
        let mesh = solid.tessellate(0.002 * scale, tol).unwrap();
        assert!((mesh.signed_volume() - volume).abs() < 0.02 * scale.powi(3));
        let key = |p: Point3| {
            [
                (p.x / scale * 1e8).round() as i64,
                (p.y / scale * 1e8).round() as i64,
                (p.z / scale * 1e8).round() as i64,
            ]
        };
        let mut uses = std::collections::BTreeMap::new();
        let mut floor = 0;
        let mut rim_segments = 0;
        for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
            let p = tri.map(|i| mesh.positions[i]);
            if *face == 7 {
                floor += 1;
                assert!(p.iter().all(|p| (p.z + 0.5 * scale).abs() < epsilon));
                assert!((p[1] - p[0]).cross(p[2] - p[0]).z > 0.);
            }
            for i in 0..3 {
                if *face == 6 && (p[i].z - p[(i + 1) % 3].z).abs() < epsilon {
                    let midpoint = (p[i] + p[(i + 1) % 3]) * 0.5;
                    assert!(scale - midpoint.x.hypot(midpoint.y) <= 0.002 * scale + epsilon);
                    rim_segments += 1;
                }
                let a = key(p[i]);
                let b = key(p[(i + 1) % 3]);
                let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let v = uses.entry(edge).or_insert((0, 0));
                v.0 += 1;
                v.1 += sign;
            }
        }
        assert!(floor > 8);
        assert!(rim_segments > 8);
        assert!(uses.values().all(|&(count, sign)| count == 2 && sign == 0));
    }
}
#[test]
fn different_blind_depths_and_empty_tools_preserve_analytic_volume() {
    let t = Tolerance::default();
    let b = block(1.);
    let tools = [
        CylinderSpec {
            base: Point3::new(-2., 0., -0.5),
            radius: 0.6,
            height: 2.,
        },
        CylinderSpec {
            base: Point3::new(2., 0., 0.3),
            radius: 0.8,
            height: 1.,
        },
    ];
    let solid = subtract_blind_cylinders(b, &tools, t).unwrap();
    solid.validate(t).unwrap();
    assert_eq!((solid.shell.faces.len(), solid.edges.len()), (10, 18));
    assert!((solid.volume().unwrap() - (96. - PI * (0.36 * 1.5 + 0.64 * 0.7))).abs() < 1e-10);
    let empty = subtract_blind_cylinders(b, &[], t).unwrap();
    assert_eq!(
        empty.mesh_json(0.02, t).unwrap(),
        make_box(b, t).unwrap().mesh_json(0.02, t).unwrap()
    );
}
#[test]
fn invalid_contact_breakthrough_and_intersecting_blind_tools_are_rejected() {
    let t = Tolerance::default();
    let b = block(1.);
    let c = tool(1.);
    for base in [
        Point3::new(0., 0., -1.),
        Point3::new(0., 0., -1. + 1e-9),
        Point3::new(0., 0., 1.),
        Point3::new(3., 0., -0.5),
        Point3::new(f64::NAN, 0., 0.),
    ] {
        assert!(subtract_blind_cylinder(b, CylinderSpec { base, ..c }, t).is_err());
    }
    for height in [0., -1., 1.5, 1.5 + 1e-9, f64::INFINITY] {
        assert!(subtract_blind_cylinder(b, CylinderSpec { height, ..c }, t).is_err());
    }
    for radius in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(subtract_blind_cylinder(b, CylinderSpec { radius, ..c }, t).is_err());
    }
    for x in [0., 1., 2., 2. + 1e-9] {
        assert!(subtract_blind_cylinders(
            b,
            &[
                c,
                CylinderSpec {
                    base: Point3::new(x, 0., 0.3),
                    ..c
                }
            ],
            t
        )
        .is_err());
    }
    assert!(subtract_blind_cylinders(b, &vec![c; 257], t).is_err());
}
