use hagane::*;
use std::f64::consts::PI;
#[test]
fn exact_skew_blind_bores_have_real_floors_and_closed_bounded_meshes() {
    for scale in [1e-6, 1., 1000.] {
        let outer: Vec<_> = [[-40., -30.], [40., -30.], [40., 30.], [-40., 30.]]
            .map(|p| [p[0] * scale, p[1] * scale])
            .to_vec();
        let tools = [
            BoxBore {
                center: [44. * scale, 0.],
                radius: 4. * scale,
                depth: Some(8. * scale),
            },
            BoxBore {
                center: [0., 10. * scale],
                radius: 3. * scale,
                depth: None,
            },
        ];
        let t = Tolerance::new(1e-8 * scale).unwrap();
        for profile in [outer.clone(), outer.iter().rev().copied().collect()] {
            let solid = subtract_skew_polygon_region_prism_bores(
                &profile,
                &[],
                24. * scale,
                [18. * scale, 0.],
                &tools,
                t,
            )
            .unwrap();
            solid.validate(t).unwrap();
            let expected = (4800. * 24. - PI * (16. * 8. + 9. * 24.)) * scale.powi(3);
            assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
            assert_eq!(solid.shell.faces.len(), 9);
            let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
            for (p, expected) in [
                (Point3::new(44., 0., 8.), PointLocation::Outside),
                (Point3::new(44., 0., 4.), PointLocation::Boundary),
                (Point3::new(44., 0., 0.), PointLocation::Inside),
                (Point3::new(48., 0., 8.), PointLocation::Boundary),
                (Point3::new(0., 10., 0.), PointLocation::Outside),
            ] {
                assert_eq!(
                    classify_point_in_solid(&solid, p * scale, policy).unwrap(),
                    expected
                );
            }
            let bounds = solid.bounds();
            assert!((bounds.min - Point3::new(-40., -30., -12.) * scale).norm() < t.linear);
            assert!((bounds.max - Point3::new(58., 30., 12.) * scale).norm() < t.linear);
            let mesh = solid.tessellate(0.01 * scale, t).unwrap();
            assert!(
                (mesh.signed_volume() - expected).abs()
                    < 2. * PI * 0.01 * (4. * 8. + 3. * 24.) * scale.powi(3)
            );
            let key = |p: Point3| {
                [
                    (p.x / (1e-9 * scale)).round() as i64,
                    (p.y / (1e-9 * scale)).round() as i64,
                    (p.z / (1e-9 * scale)).round() as i64,
                ]
            };
            let mut edges = std::collections::BTreeMap::new();
            for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                let p = tri.map(|i| mesh.positions[i]);
                for i in 0..3 {
                    let a = key(p[i]);
                    let b = key(p[(i + 1) % 3]);
                    let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                    let use_count = edges.entry(edge).or_insert((0, 0));
                    use_count.0 += 1;
                    use_count.1 += sign;
                    if *face == 6 || *face == 8 {
                        let (cx, cy, r) = if *face == 6 {
                            (44. * scale, 0., 4. * scale)
                        } else {
                            (0., 10. * scale, 3. * scale)
                        };
                        let mid = (p[i] + p[(i + 1) % 3]) * 0.5;
                        assert!(r - (mid.x - cx).hypot(mid.y - cy) <= 0.01 * scale + t.linear);
                    }
                }
            }
            assert!(edges
                .values()
                .all(|&(count, balance)| count == 2 && balance == 0));
        }
    }
}
#[test]
fn depth_interval_detects_openings_contacts_and_breakthrough() {
    let outer = [[-40., -30.], [40., -30.], [40., 30.], [-40., 30.]];
    let t = Tolerance::new(1e-8).unwrap();
    let holes = vec![vec![[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]]];
    let tool = BoxBore {
        center: [10., 0.],
        radius: 1.,
        depth: Some(4.),
    };
    assert!(
        subtract_skew_polygon_region_prism_bores(&outer, &holes, 24., [20., 0.], &[tool], t)
            .is_ok()
    );
    for depth in [Some(16.), None] {
        assert!(subtract_skew_polygon_region_prism_bores(
            &outer,
            &holes,
            24.,
            [20., 0.],
            &[BoxBore { depth, ..tool }],
            t
        )
        .is_err());
    }
    for depth in [0., f64::NAN, 24., 24. - 5e-8, 25.] {
        assert!(subtract_skew_polygon_region_prism_bores(
            &outer,
            &[],
            24.,
            [20., 0.],
            &[BoxBore {
                depth: Some(depth),
                ..tool
            }],
            t
        )
        .is_err());
    }
    // A shallow cut outside the lower stock is valid; increasing depth hits its sloping side.
    for depth in [Some(8.), Some(16.), None] {
        let result = subtract_skew_polygon_region_prism_bores(
            &outer,
            &[],
            24.,
            [18., 0.],
            &[BoxBore {
                center: [44., 0.],
                radius: 4.,
                depth,
            }],
            t,
        );
        assert_eq!(result.is_ok(), depth == Some(8.));
    }
}
#[test]
fn blind_depth_edits_reuse_prefixes_and_reject_atomically() {
    let mut doc: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-skew-blind-example.json")).unwrap();
    let mut session = WorkflowSession::new();
    session.rebuild(&doc).unwrap();
    if let WorkflowOperation::Bore { depth, .. } = &mut doc.operations[1] {
        *depth = Some(12.);
    }
    let edited = session.rebuild(&doc).unwrap();
    assert_eq!(edited.stats.reused_operations, 1);
    assert!((edited.solid.volume().unwrap() - (4408. * 24. - 16. * PI * 12.)).abs() < 1e-8);
    let accepted = doc.clone();
    if let WorkflowOperation::Bore { depth, .. } = &mut doc.operations[1] {
        *depth = Some(24.);
    }
    assert_eq!(session.rebuild(&doc).unwrap_err().code, "floor_thickness");
    let recovered = session.rebuild(&accepted).unwrap();
    assert_eq!(recovered.stats.rebuilt_operations, 0);
    assert!(std::sync::Arc::ptr_eq(&edited.solid, &recovered.solid));
    let t = Tolerance::new(1e-8).unwrap();
    assert_eq!(
        edited.solid.mesh_json(0.05, t).unwrap(),
        accepted.rebuild().unwrap().mesh_json(0.05, t).unwrap()
    );
    // Workflow's tool envelope must include translated stock, as the public API does.
    if let WorkflowOperation::Bore { center, depth, .. } = &mut doc.operations[1] {
        *center = [44., 0.];
        *depth = Some(8.);
    }
    doc.rebuild().unwrap();
}
