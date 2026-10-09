use hagane::*;
fn box_at(min: Point3, size: Vec3, t: Tolerance) -> Solid {
    make_box(BoxSpec { min, size }, t).unwrap()
}
fn check_components(parts: &[Solid], expected: f64, scale: f64, policy: GeometryTolerance) {
    let t = policy.absolute();
    let total: f64 = parts.iter().map(|s| s.volume().unwrap()).sum();
    assert!((total - expected).abs() < expected * 1e-10);
    for part in parts {
        part.validate(t).unwrap();
        let mesh = part.tessellate(0.05 * scale, t).unwrap();
        assert!((mesh.signed_volume() - part.volume().unwrap()).abs() < expected * 1e-10);
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
        assert!(uses
            .values()
            .all(|&(count, balance)| count == 2 && balance == 0));
    }
}
#[test]
fn through_slot_returns_two_closed_solids_with_preserved_volume() {
    for scale in [1e-6, 1., 1000.] {
        let policy = GeometryTolerance::new(1e-8 * scale, 1e-10, 0.).unwrap();
        let t = policy.absolute();
        let stock = box_at(
            Point3::new(-30., -20., -12.) * scale,
            Vec3::new(60., 40., 24.) * scale,
            t,
        );
        let tool = box_at(
            Point3::new(-4., -30., -20.) * scale,
            Vec3::new(8., 60., 40.) * scale,
            t,
        );
        let parts = subtract_convex_from_planar_solid_components(&stock, &tool, policy).unwrap();
        assert_eq!(parts.len(), 2);
        check_components(&parts, 49920. * scale.powi(3), scale, policy);
        for part in &parts {
            assert!(
                (part.volume().unwrap() - 24960. * scale.powi(3)).abs()
                    < stock.volume().unwrap() * 1e-10
            );
            assert_eq!(
                classify_point_in_solid(part, Point3::new(0., 0., 0.), policy).unwrap(),
                PointLocation::Outside
            );
        }
        for p in [Point3::new(-20., 0., 0.), Point3::new(20., 0., 0.)] {
            assert_eq!(
                parts
                    .iter()
                    .filter(|s| classify_point_in_solid(s, p * scale, policy).unwrap()
                        == PointLocation::Inside)
                    .count(),
                1
            );
        }
        let common = intersect_planar_solid_with_convex_components(&stock, &tool, policy).unwrap();
        assert_eq!(common.len(), 1);
        check_components(&common, 7680. * scale.powi(3), scale, policy);
        let translation = Transform::translation(Vec3::new(100., -40., 30.) * scale).unwrap();
        let moved = subtract_convex_from_planar_solid_components(
            &stock.transformed(translation, t).unwrap(),
            &tool.transformed(translation, t).unwrap(),
            policy,
        )
        .unwrap();
        check_components(&moved, 49920. * scale.powi(3), scale, policy);
    }
}
#[test]
fn nonconvex_common_and_difference_retain_all_components() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let stock = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -12.),
            outer: vec![
                [-30., -20.],
                [30., -20.],
                [30., 20.],
                [10., 20.],
                [10., -5.],
                [-10., -5.],
                [-10., 20.],
                [-30., 20.],
            ],
            holes: vec![],
        },
        Vec3::new(0., 0., 24.),
        t,
    )
    .unwrap();
    let tool = box_at(Point3::new(-40., 2., -20.), Vec3::new(80., 10., 40.), t);
    let common = intersect_planar_solid_with_convex_components(&stock, &tool, policy).unwrap();
    assert_eq!(common.len(), 2);
    check_components(&common, 9600., 1., policy);
    for p in [Point3::new(-20., 8., 0.), Point3::new(20., 8., 0.)] {
        assert_eq!(
            common
                .iter()
                .filter(|s| classify_point_in_solid(s, p, policy).unwrap() == PointLocation::Inside)
                .count(),
            1
        );
    }
    let remainder = subtract_convex_from_planar_solid_components(&stock, &tool, policy).unwrap();
    assert_eq!(remainder.len(), 3);
    check_components(&remainder, 36000., 1., policy);
    for part in &remainder {
        assert_eq!(
            classify_point_in_solid(part, Point3::new(20., 8., 0.), policy).unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn empty_unchanged_contacts_curves_and_cavities_are_explicit() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let stock = box_at(Point3::new(-30., -20., -12.), Vec3::new(60., 40., 24.), t);
    let separated = box_at(Point3::new(50., -30., -20.), Vec3::new(8., 60., 40.), t);
    let unchanged =
        subtract_convex_from_planar_solid_components(&stock, &separated, policy).unwrap();
    assert_eq!(unchanged.len(), 1);
    assert_eq!(
        unchanged[0].mesh_json(0.05, t).unwrap(),
        stock.mesh_json(0.05, t).unwrap()
    );
    assert!(
        intersect_planar_solid_with_convex_components(&stock, &separated, policy)
            .unwrap()
            .is_empty()
    );
    let containing = box_at(Point3::new(-40., -30., -20.), Vec3::new(80., 60., 40.), t);
    assert!(
        subtract_convex_from_planar_solid_components(&stock, &containing, policy)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        intersect_planar_solid_with_convex_components(&stock, &containing, policy)
            .unwrap()
            .len(),
        1
    );
    for x in [22., 22. - 5e-8] {
        let tool = box_at(Point3::new(x, -30., -20.), Vec3::new(8., 60., 40.), t);
        assert!(subtract_convex_from_planar_solid_components(&stock, &tool, policy).is_err());
    }
    let cavity = box_at(Point3::new(-2., -3., -4.), Vec3::new(4., 6., 8.), t);
    assert!(matches!(
        subtract_convex_from_planar_solid_components(&stock, &cavity, policy),
        Err(Error::Unsupported(_))
    ));
    let curved = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -20.),
            radius: 4.,
            height: 40.,
        },
        t,
    )
    .unwrap();
    assert!(subtract_convex_from_planar_solid_components(&stock, &curved, policy).is_err());
    assert!(intersect_planar_solid_with_convex_components(&curved, &stock, policy).is_err());
}
#[test]
fn holed_nonconvex_common_preserves_opening_provenance() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let stock = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -12.),
            outer: vec![
                [-30., -20.],
                [30., -20.],
                [30., 20.],
                [10., 20.],
                [10., -5.],
                [-10., -5.],
                [-10., 20.],
                [-30., 20.],
            ],
            holes: vec![vec![[-22., 6.], [-18., 6.], [-18., 8.], [-22., 8.]]],
        },
        Vec3::new(0., 0., 24.),
        t,
    )
    .unwrap();
    let tool = box_at(Point3::new(-40., 2., -20.), Vec3::new(80., 10., 40.), t);
    let common = intersect_planar_solid_with_convex_components(&stock, &tool, policy).unwrap();
    assert_eq!(common.len(), 2);
    check_components(&common, 9408., 1., policy);
    for component in &common {
        assert_eq!(
            classify_point_in_solid(component, Point3::new(-20., 7., 0.), policy).unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        common
            .iter()
            .filter(
                |s| classify_point_in_solid(s, Point3::new(-20., 4., 0.), policy).unwrap()
                    == PointLocation::Inside
            )
            .count(),
        1
    );
    let remainder = subtract_convex_from_planar_solid_components(&stock, &tool, policy).unwrap();
    assert_eq!(remainder.len(), 3);
    check_components(&remainder, 36000., 1., policy);
}
