use hagane::*;
fn shape(scale: f64, t: Tolerance) -> Solid {
    extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -12. * scale),
            outer: vec![
                [-30., -20.],
                [30., -20.],
                [30., 20.],
                [10., 20.],
                [10., -5.],
                [-10., -5.],
                [-10., 20.],
                [-30., 20.],
            ]
            .into_iter()
            .map(|p| [p[0] * scale, p[1] * scale])
            .collect(),
            holes: vec![],
        },
        Vec3::new(0., 0., 24. * scale),
        t,
    )
    .unwrap()
}
fn plane(y: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(0., y, 0.),
        u: Vec3::new(0., 0., 1.),
        v: Vec3::new(1., 0., 0.),
    }
}
#[test]
fn disconnected_arms_are_individually_closed_and_conserve_volume() {
    for scale in [1e-6, 1., 1000.] {
        let policy = GeometryTolerance::new(1e-8 * scale, 1e-10, 0.).unwrap();
        let t = policy.absolute();
        let solid = shape(scale, t);
        assert!(split_solid_by_plane(&solid, &plane(0.), policy).is_err());
        let split = split_solid_by_plane_components(&solid, &plane(0.), policy).unwrap();
        assert_eq!(
            (
                split.negative.len(),
                split.positive.len(),
                split.section.len()
            ),
            (1, 2, 2)
        );
        assert!(
            (split.negative[0].volume().unwrap() - 26400. * scale.powi(3)).abs()
                < solid.volume().unwrap() * 1e-12
        );
        for component in &split.positive {
            assert!(
                (component.volume().unwrap() - 9600. * scale.powi(3)).abs()
                    < solid.volume().unwrap() * 1e-12
            );
        }
        let mut total = 0.;
        for component in split.negative.iter().chain(&split.positive) {
            component.validate(t).unwrap();
            total += component.volume().unwrap();
            let mesh = component.tessellate(0.05 * scale, t).unwrap();
            assert!(
                (mesh.signed_volume() - component.volume().unwrap()).abs()
                    < solid.volume().unwrap() * 1e-12
            );
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
                    let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                    let e = uses.entry(edge).or_insert((0, 0));
                    e.0 += 1;
                    e.1 += sign;
                }
            }
            assert!(uses.values().all(|&(n, b)| n == 2 && b == 0));
        }
        assert!((total - solid.volume().unwrap()).abs() < total * 1e-12);
        for p in [Point3::new(-20., 10., 0.), Point3::new(20., 10., 0.)] {
            assert_eq!(
                split
                    .positive
                    .iter()
                    .filter(|s| classify_point_in_solid(s, p * scale, policy).unwrap()
                        == PointLocation::Inside)
                    .count(),
                1
            );
        }
        for component in &split.positive {
            assert_eq!(
                classify_point_in_solid(component, Point3::new(0., 10., 0.) * scale, policy)
                    .unwrap(),
                PointLocation::Outside
            );
        }
        // Plane normal reversal swaps the independently closed sides.
        let mut reversed = plane(0.);
        if let Surface::Plane { v, .. } = &mut reversed {
            *v = *v * -1.;
        }
        let reverse = split_solid_by_plane_components(&solid, &reversed, policy).unwrap();
        assert_eq!((reverse.negative.len(), reverse.positive.len()), (2, 1));
    }
}
#[test]
fn placements_contacts_curves_and_single_component_compatibility() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let solid = shape(1., t);
    for y in [-5., -5. + 5e-8, 20., 30., f64::NAN] {
        assert!(split_solid_by_plane_components(&solid, &plane(y), policy).is_err());
    }
    let curved = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -12.),
            radius: 10.,
            height: 24.,
        },
        t,
    )
    .unwrap();
    assert!(split_solid_by_plane_components(&curved, &plane(0.), policy).is_err());
    let placement = Transform::translation(Vec3::new(100., -70., 30.)).unwrap();
    let placed = solid.transformed(placement, t).unwrap();
    let mut cut = plane(-70.);
    if let Surface::Plane { origin, .. } = &mut cut {
        origin.x = 100.;
        origin.z = 30.;
    }
    let result = split_solid_by_plane_components(&placed, &cut, policy).unwrap();
    assert_eq!(result.positive.len(), 2);
    assert!(
        (result
            .positive
            .iter()
            .map(|s| s.volume().unwrap())
            .sum::<f64>()
            - 19200.)
            .abs()
            < 1e-8
    );
    let single = split_solid_by_plane_components(&solid, &plane(-10.), policy).unwrap();
    assert_eq!((single.negative.len(), single.positive.len()), (1, 1));
    let old = split_solid_by_plane(&solid, &plane(-10.), policy).unwrap();
    assert_eq!(
        single.negative[0].mesh_json(0.05, t).unwrap(),
        old.negative.mesh_json(0.05, t).unwrap()
    );
}
#[test]
fn component_grouping_preserves_polygon_openings_in_one_arm() {
    let policy = GeometryTolerance::default();
    let t = policy.absolute();
    let solid = extrude_polygon(
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
            holes: vec![vec![[-22., 8.], [-18., 8.], [-18., 12.], [-22., 12.]]],
        },
        Vec3::new(0., 0., 24.),
        t,
    )
    .unwrap();
    let split = split_solid_by_plane_components(&solid, &plane(0.), policy).unwrap();
    assert_eq!(split.positive.len(), 2);
    let mut volumes: Vec<_> = split.positive.iter().map(|s| s.volume().unwrap()).collect();
    volumes.sort_by(f64::total_cmp);
    assert!((volumes[0] - 9216.).abs() < 1e-8);
    assert!((volumes[1] - 9600.).abs() < 1e-8);
    for component in &split.positive {
        component.validate(t).unwrap();
        assert_eq!(
            classify_point_in_solid(component, Point3::new(-20., 10., 0.), policy).unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        split
            .positive
            .iter()
            .filter(
                |s| classify_point_in_solid(s, Point3::new(-20., 6., 0.), policy).unwrap()
                    == PointLocation::Inside
            )
            .count(),
        1
    );
    assert!(
        (split.negative[0].volume().unwrap() + volumes.iter().sum::<f64>()
            - solid.volume().unwrap())
        .abs()
            < 1e-8
    );
}
