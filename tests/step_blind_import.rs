use hagane::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::f64::consts::{PI, TAU};

fn document(scale: f64) -> WorkflowDocument {
    let mut value: Value =
        serde_json::from_str(include_str!("../docs/workflow-opposing-blind-example.json")).unwrap();
    fn scaled(value: &mut Value, scale: f64) {
        if let Some(array) = value.as_array_mut() {
            for v in array {
                if let Some(n) = v.as_f64() {
                    *v = json!(n * scale);
                } else {
                    scaled(v, scale);
                }
            }
        }
    }
    value["tolerance"]["linear"] = json!(1e-8 * scale);
    for key in ["outer", "holes", "offset"] {
        scaled(&mut value["operations"][0][key], scale);
    }
    value["operations"][0]["height"] = json!(24. * scale);
    for i in [1, 2] {
        scaled(&mut value["operations"][i]["center"], scale);
        value["operations"][i]["radius"] = json!((if i == 1 { 4. } else { 6. }) * scale);
        value["operations"][i]["depth"] = json!((if i == 1 { 8. } else { 10. }) * scale);
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn opposing_blind_floors_web_and_round_trips_at_three_scales_and_placements() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let stock = document(scale).rebuild().unwrap();
        let placed = Transform::translation(Vec3::new(20. * scale, -7. * scale, 12. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap();
        for transform in [Transform::IDENTITY, placed] {
            let mut source = stock.transformed(transform, t).unwrap();
            source.shell.faces.reverse();
            let input = export_step_mm(&source, t).unwrap();
            let s = import_step_mm(&input, t).unwrap();
            let proof = certify_bored_prism(&s, t).unwrap();
            assert_eq!(proof.bores.len(), 2);
            assert!(proof.minimum_clearance > 10. * t.linear);
            let mut intervals = proof.bore_intervals;
            intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
            for (actual, expected) in intervals.iter().zip([[0., 8.], [14., 24.]]) {
                for k in 0..2 {
                    assert!((actual[k] - expected[k] * scale).abs() < t.linear);
                }
            }
            assert_eq!(
                (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
                (28, 42, 18)
            );
            let expected = (105792. - PI * (16. * 8. + 36. * 10.)) * scale.powi(3);
            assert!((s.volume().unwrap() - expected).abs() < expected * 1e-12);
            assert!((s.bounds().min - source.bounds().min).norm() < t.linear);
            assert!((s.bounds().max - source.bounds().max).norm() < t.linear);
            let policy = GeometryTolerance::try_from(t).unwrap();
            for (z, expected) in [
                (-8., PointLocation::Outside),
                (-4., PointLocation::Boundary),
                (0., PointLocation::Inside),
                (2., PointLocation::Boundary),
                (8., PointLocation::Outside),
            ] {
                let point = transform.point(Point3::new(24., 0., z) * scale);
                assert_eq!(
                    classify_point_in_solid(&s, point, policy).unwrap(),
                    expected
                );
            }
            let mesh = s.tessellate(0.01 * scale, t).unwrap();
            assert!(
                (mesh.signed_volume() - expected).abs()
                    < TAU * 0.01 * (4. * 8. + 6. * 10.) * scale.powi(3)
            );
            let restored = import_step_mm(&export_step_mm(&s, t).unwrap(), t).unwrap();
            assert!((restored.volume().unwrap() - expected).abs() < expected * 1e-12);
            assert!(import_step_planar_mm(&input, t).is_err());
        }
    }
}

#[test]
fn mixed_through_and_blind_tools_preserve_source_boundary_geometry() {
    let t = Tolerance::default();
    let source = subtract_polygon_prism_bores(
        &[
            [-10., -10.],
            [10., -10.],
            [10., 0.],
            [0., 0.],
            [0., 10.],
            [-10., 10.],
        ],
        8.,
        &[
            BoxBore {
                center: [-6., 0.],
                radius: 1.,
                depth: Some(3.),
            },
            BoxBore {
                center: [4., -6.],
                radius: 2.,
                depth: None,
            },
        ],
        t,
    )
    .unwrap();
    let imported = import_step_mm(&export_step_mm(&source, t).unwrap(), t).unwrap();
    let proof = certify_bored_prism(&imported, t).unwrap();
    assert_eq!(proof.bore_intervals, vec![[5., 8.], [0., 8.]]);
    assert!((imported.volume().unwrap() - (2400. - 35. * PI)).abs() < 1e-10);
    let positions = |solid: &Solid| {
        let mut points: Vec<_> = solid.vertices.iter().map(|v| v.point).collect();
        points.sort_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then(a.y.total_cmp(&b.y))
                .then(a.z.total_cmp(&b.z))
        });
        points
    };
    assert_eq!(positions(&imported), positions(&source));
    for (actual, expected) in imported.shell.faces.iter().zip(&source.shell.faces) {
        assert_eq!(actual.orientation, expected.orientation);
        assert_eq!(actual.wires.len(), expected.wires.len());
    }
}

#[test]
fn shallow_skew_blind_bore_outside_lower_footprint_uses_only_actual_depth() {
    let t = Tolerance::default();
    let source = subtract_skew_polygon_region_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        &[],
        4.,
        [8., 0.],
        &[BoxBore {
            center: [14., 0.],
            radius: 1.,
            depth: Some(1.),
        }],
        t,
    )
    .unwrap();
    let imported = import_step_mm(&export_step_mm(&source, t).unwrap(), t).unwrap();
    let proof = certify_bored_prism(&imported, t).unwrap();
    assert_eq!(proof.bore_intervals, vec![[3., 4.]]);
    assert!((imported.volume().unwrap() - (1600. - PI)).abs() < 1e-10);
    assert_eq!(
        classify_point_in_solid(
            &imported,
            Point3::new(14., 0., 3.5),
            GeometryTolerance::try_from(t).unwrap()
        )
        .unwrap(),
        PointLocation::Outside
    );
}

#[test]
fn near_floor_and_near_opposing_web_are_rejected_despite_local_closure() {
    let t = Tolerance::default();
    let tight = Tolerance::new(t.linear / 100.).unwrap();
    let floor = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[BoxBore {
            center: [0., 0.],
            radius: 1.,
            depth: Some(4. - 5. * t.linear),
        }],
        tight,
    )
    .unwrap();
    let mut value: Value = serde_json::to_value(document(1.)).unwrap();
    value["tolerance"]["linear"] = json!(tight.linear);
    value["operations"][2]["depth"] = json!(16. - 5. * t.linear);
    let web = serde_json::from_value::<WorkflowDocument>(value)
        .unwrap()
        .rebuild()
        .unwrap();
    for source in [floor, web] {
        source.validate(t).unwrap();
        assert!(matches!(
            certify_bored_prism(&source, t),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            import_step_mm(&export_step_mm(&source, tight).unwrap(), t),
            Err(Error::Unsupported(_))
        ));
    }
}

// Move a complete top-entry tool boundary, including its retained floor.
// This creates globally invalid tool intersections that local closure misses.
fn move_top_tool(s: &mut Solid, wall: usize, shift: Vec3) {
    let transform = Transform::translation(shift).unwrap();
    let edges: BTreeSet<_> = s.shell.faces[wall].wires[0]
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
    s.shell.faces[wall].surface = s.shell.faces[wall].surface.transformed(transform).unwrap();
    for face in &mut s.shell.faces {
        let Surface::Plane { u, v, .. } = face.surface else {
            continue;
        };
        if face.wires.len() == 1
            && face.wires[0].coedges.len() == 1
            && edges.contains(&face.wires[0].coedges[0].edge)
        {
            face.surface = face.surface.transformed(transform).unwrap();
        } else {
            for wire in &mut face.wires {
                for c in &mut wire.coedges {
                    if edges.contains(&c.edge) {
                        let PCurve::Circle { center, .. } = &mut c.pcurve else {
                            panic!("circle rim")
                        };
                        center[0] += shift.dot(u);
                        center[1] += shift.dot(v);
                    }
                }
            }
        }
    }
}

#[test]
fn overlapping_cut_intervals_with_intersecting_footprints_fail() {
    let t = Tolerance::default();
    let mut value: Value = serde_json::to_value(document(1.)).unwrap();
    value["operations"][2]["center"] = json!([8., 0.]);
    value["operations"][2]["depth"] = json!(18.);
    let mut source = serde_json::from_value::<WorkflowDocument>(value)
        .unwrap()
        .rebuild()
        .unwrap();
    let wall = source
        .shell
        .faces
        .iter()
        .rposition(|f| !matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    move_top_tool(&mut source, wall, Vec3::new(16., 0., 0.));
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

#[test]
fn blind_tool_crossing_an_opening_only_between_entry_and_floor_fails() {
    let t = Tolerance::default();
    let mut source = subtract_skew_polygon_region_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        &[vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]],
        4.,
        [8., 0.],
        &[BoxBore {
            center: [5., 5.],
            radius: 0.5,
            depth: Some(3.),
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
    move_top_tool(&mut source, wall, Vec3::new(0., -5., 0.));
    source.validate(t).unwrap(); // Entry/floor trims are valid; intermediate stock crosses.
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
fn sub_roundoff_floor_is_not_certified_by_ultratight_tolerance() {
    let t = Tolerance::new(1e-15).unwrap();
    let source = subtract_polygon_prism_bores(
        &[[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]],
        4.,
        &[BoxBore {
            center: [0., 0.],
            radius: 1.,
            depth: Some(4. - 1e-13),
        }],
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
