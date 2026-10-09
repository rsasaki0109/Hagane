use hagane::*;
use std::collections::BTreeSet;
use std::f64::consts::PI;
fn specimen(scale: f64, t: Tolerance) -> Solid {
    let outer = [
        [-40., -20.],
        [-30., -30.],
        [30., -30.],
        [40., -20.],
        [40., 20.],
        [30., 30.],
        [-30., 30.],
        [-40., 20.],
    ]
    .map(|p| [p[0] * scale, p[1] * scale]);
    let holes = vec![vec![
        [-34. * scale, -8. * scale],
        [-22. * scale, -8. * scale],
        [-22. * scale, 8. * scale],
        [-34. * scale, 8. * scale],
    ]];
    subtract_skew_polygon_region_prism_bores(
        &outer,
        &holes,
        24. * scale,
        [18. * scale, -12. * scale],
        &[
            BoxBore {
                center: [0., 0.],
                radius: 4. * scale,
                depth: None,
            },
            BoxBore {
                center: [24. * scale, 0.],
                radius: 3. * scale,
                depth: None,
            },
        ],
        t,
    )
    .unwrap()
}
#[test]
fn two_bores_polygon_opening_skew_placement_and_three_scales_round_trip() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let policy = GeometryTolerance::try_from(t).unwrap();
        let stock = specimen(scale, t);
        let placed = Transform::translation(Vec3::new(20. * scale, -7. * scale, 12. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap();
        for transform in [Transform::IDENTITY, placed] {
            let original = stock.transformed(transform, t).unwrap();
            for reverse in [false, true] {
                let mut source = original.clone();
                if reverse {
                    source.shell.faces.reverse();
                }
                let s = import_step_mm(&export_step_mm(&source, t).unwrap(), t).unwrap();
                let certificate = certify_bored_prism(&s, t).unwrap();
                assert_eq!(certificate.bores.len(), 2);
                assert!(certificate.minimum_clearance > 10. * t.linear);
                assert_eq!(
                    (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
                    (28, 42, 16)
                );
                let expected = (105792. - PI * 25. * 24.) * scale.powi(3);
                assert!((s.volume().unwrap() - expected).abs() < expected * 1e-12);
                assert!((s.bounds().min - source.bounds().min).norm() < t.linear);
                assert!((s.bounds().max - source.bounds().max).norm() < t.linear);
                for (p, expected) in [
                    (Point3::new(0., 0., 0.), PointLocation::Outside),
                    (Point3::new(24. * scale, 0., 0.), PointLocation::Outside),
                    (
                        Point3::new(-19. * scale, -6. * scale, 0.),
                        PointLocation::Outside,
                    ),
                    (Point3::new(12. * scale, 0., 0.), PointLocation::Inside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&s, transform.point(p), policy).unwrap(),
                        expected
                    );
                }
                let mesh = s.tessellate(0.01 * scale, t).unwrap();
                assert!(
                    (mesh.signed_volume() - s.volume().unwrap()).abs()
                        < std::f64::consts::TAU * 0.01 * 7. * 24. * scale.powi(3)
                );
                let restored = import_step_mm(&export_step_mm(&s, t).unwrap(), t).unwrap();
                assert!(
                    (s.volume().unwrap() - restored.volume().unwrap()).abs() < expected * 1e-12
                );
                assert!(import_step_planar_mm(&export_step_mm(&source, t).unwrap(), t).is_err());
            }
        }
    }
}
#[test]
fn concave_stock_and_reordered_box_caps_are_supported() {
    let t = Tolerance::default();
    let concave = subtract_skew_polygon_region_prism_bores(
        &[
            [-10., -10.],
            [10., -10.],
            [10., 0.],
            [0., 0.],
            [0., 10.],
            [-10., 10.],
        ],
        &[vec![[-8., 2.], [-5., 2.], [-5., 5.], [-8., 5.]]],
        8.,
        [2., -1.],
        &[
            BoxBore {
                center: [4., -5.],
                radius: 1.,
                depth: None,
            },
            BoxBore {
                center: [-4., -6.],
                radius: 1.5,
                depth: None,
            },
        ],
        t,
    )
    .unwrap();
    let imported = import_step_mm(&export_step_mm(&concave, t).unwrap(), t).unwrap();
    assert!((imported.volume().unwrap() - (2328. - 26. * PI)).abs() < 1e-10);
    let mut boxed = subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-4., -3., 0.),
            size: Vec3::new(8., 6., 4.),
        },
        CylinderSpec {
            base: Point3::new(0., 0., -1.),
            radius: 1.,
            height: 6.,
        },
        t,
    )
    .unwrap();
    boxed.shell.faces.rotate_left(2);
    let imported = import_step_mm(&export_step_mm(&boxed, t).unwrap(), t).unwrap();
    let certificate = certify_bored_prism(&imported, t).unwrap();
    assert_eq!(certificate.bores.len(), 1);
    assert!((certificate.minimum_clearance - 2.).abs() < 1e-12);
    assert!((imported.volume().unwrap() - (192. - 4. * PI)).abs() < 1e-11);
}
// Deliberately move just one exact tool boundary, preserving its topology and UV
// curves. This exposes invalid global intersections that local closure misses.
fn move_tool(s: &mut Solid, wall_index: usize, shift: Vec3) {
    let transform = Transform::translation(shift).unwrap();
    let edges: BTreeSet<_> = s.shell.faces[wall_index].wires[0]
        .coedges
        .iter()
        .map(|c| c.edge)
        .collect();
    let vertices: BTreeSet<_> = edges.iter().flat_map(|&e| s.edges[e].vertices).collect();
    for v in vertices {
        s.vertices[v].point = s.vertices[v].point + shift;
    }
    for &e in &edges {
        s.edges[e].curve = s.edges[e].curve.transformed(transform).unwrap();
    }
    s.shell.faces[wall_index].surface = s.shell.faces[wall_index]
        .surface
        .transformed(transform)
        .unwrap();
    for face in &mut s.shell.faces {
        let Surface::Plane { u, v, .. } = face.surface else {
            continue;
        };
        for wire in &mut face.wires {
            for c in &mut wire.coedges {
                if edges.contains(&c.edge) {
                    let PCurve::Circle { center, .. } = &mut c.pcurve else {
                        panic!("rim must be circular")
                    };
                    center[0] += shift.dot(u);
                    center[1] += shift.dot(v);
                }
            }
        }
    }
}
#[test]
fn middle_crossing_is_rejected_even_when_both_caps_and_local_closure_are_valid() {
    let t = Tolerance::default();
    let mut source = subtract_skew_polygon_region_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        &[vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]],
        4.,
        [8., 0.],
        &[BoxBore {
            center: [4., 5.],
            radius: 0.5,
            depth: None,
        }],
        t,
    )
    .unwrap();
    let wall = source
        .shell
        .faces
        .iter()
        .position(|f| !matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    move_tool(&mut source, wall, Vec3::new(0., -5., 0.));
    source.validate(t).unwrap(); // End caps remain separated; the mid-depth openings cross.
    assert!(matches!(
        certify_bored_prism(&source, t),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_mm(&export_step_mm(&source, t).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn near_contacts_side_crossings_and_blind_floors_never_succeed() {
    let t = Tolerance::default();
    let construction = Tolerance::new(t.linear / 100.).unwrap();
    let near = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[
            BoxBore {
                center: [0., 0.],
                radius: 1.,
                depth: None,
            },
            BoxBore {
                center: [2. + 5. * t.linear, 0.],
                radius: 1.,
                depth: None,
            },
        ],
        construction,
    )
    .unwrap();
    near.validate(t).unwrap();
    assert!(matches!(
        import_step_mm(&export_step_mm(&near, construction).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    let blind = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[BoxBore {
            center: [0., 0.],
            radius: 1.,
            depth: Some(2.),
        }],
        t,
    )
    .unwrap();
    assert!(matches!(
        import_step_mm(&export_step_mm(&blind, t).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    let mut contact = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[BoxBore {
            center: [0., 0.],
            radius: 1.,
            depth: None,
        }],
        t,
    )
    .unwrap();
    let wall = contact
        .shell
        .faces
        .iter()
        .position(|f| !matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    move_tool(&mut contact, wall, Vec3::new(9., 0., 0.));
    assert!(contact.validate(t).is_err());
    assert!(export_step_mm(&contact, t).is_err());
    let near_side = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[BoxBore {
            center: [9. - 5. * t.linear, 0.],
            radius: 1.,
            depth: None,
        }],
        construction,
    )
    .unwrap();
    near_side.validate(t).unwrap();
    assert!(matches!(
        import_step_mm(&export_step_mm(&near_side, construction).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn sub_roundoff_pair_clearance_is_not_certified_by_an_ultratight_model_tolerance() {
    let t = Tolerance::new(1e-15).unwrap();
    let source = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[
            BoxBore {
                center: [0., 0.],
                radius: 1.,
                depth: None,
            },
            BoxBore {
                center: [2. + 1e-13, 0.],
                radius: 1.,
                depth: None,
            },
        ],
        t,
    )
    .unwrap();
    source.validate(t).unwrap();
    assert!(matches!(
        certify_bored_prism(&source, t),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_mm(&export_step_mm(&source, t).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
}
