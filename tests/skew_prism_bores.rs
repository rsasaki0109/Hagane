use hagane::*;
#[test]
fn skew_through_bores_preserve_exact_solid_and_incremental_history() {
    for scale in [1e-6, 1., 1000.] {
        let mut doc: WorkflowDocument =
            serde_json::from_str(include_str!("../docs/workflow-skew-extrusion-example.json"))
                .unwrap();
        doc.tolerance.linear *= scale;
        if let WorkflowOperation::Extrusion {
            outer,
            holes,
            height,
            offset,
            ..
        } = &mut doc.operations[0]
        {
            for p in outer.iter_mut().chain(holes.iter_mut().flatten()) {
                p[0] *= scale;
                p[1] *= scale;
            }
            *height *= scale;
            offset[0] *= scale;
            offset[1] *= scale;
        }
        for (id, input, center, radius) in [
            ("bore-1", "extrusion-1", [0., 0.], 4.),
            ("bore-2", "bore-1", [24., 0.], 3.),
        ] {
            doc.operations.push(WorkflowOperation::Bore {
                id: id.into(),
                input: input.into(),
                center: [center[0] * scale, center[1] * scale],
                radius: radius * scale,
                mode: WorkflowBoreMode::Through,
                depth: None,
            });
        }
        let t = Tolerance::new(doc.tolerance.linear).unwrap();
        let solid = doc.rebuild().unwrap();
        solid.validate(t).unwrap();
        let expected = (4408. - std::f64::consts::PI * 25.) * 24. * scale.powi(3);
        assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
        assert_eq!(solid.shell.faces.len(), 16);
        let mesh = solid.tessellate(0.01 * scale, t).unwrap();
        // Weld display vertices only for verification; every geometric edge
        // must have two oppositely directed uses, including cap/cylinder seams.
        let key = |p: Point3| {
            [
                (p.x / (1e-9 * scale)).round() as i64,
                (p.y / (1e-9 * scale)).round() as i64,
                (p.z / (1e-9 * scale)).round() as i64,
            ]
        };
        let mut uses = std::collections::BTreeMap::new();
        for (triangle, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
            let p = triangle.map(|i| mesh.positions[i]);
            for i in 0..3 {
                let a = key(p[i]);
                let b = key(p[(i + 1) % 3]);
                let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let entry = uses.entry(edge).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += sign;
            }
            if *face >= 14 {
                let (cx, radius) = if *face == 14 {
                    (0., 4. * scale)
                } else {
                    (24. * scale, 3. * scale)
                };
                for i in 0..3 {
                    let midpoint = (p[i] + p[(i + 1) % 3]) * 0.5;
                    let radial = (midpoint.x - cx).hypot(midpoint.y);
                    assert!(radius - radial <= 0.01 * scale + t.linear);
                }
            }
        }
        assert!(uses
            .values()
            .all(|&(count, balance)| count == 2 && balance == 0));

        assert!(
            (mesh.signed_volume() - expected).abs()
                < 2. * std::f64::consts::PI * 0.01 * scale * 7. * scale * 24. * scale
        );
        for (p, location) in [
            (Point3::new(0., 0., 0.), PointLocation::Outside),
            (Point3::new(4., 0., 0.), PointLocation::Boundary),
            (Point3::new(8., 0., 0.), PointLocation::Inside),
        ] {
            assert_eq!(
                classify_point_in_solid(
                    &solid,
                    p * scale,
                    GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap()
                )
                .unwrap(),
                location
            );
        }
        let mut session = WorkflowSession::new();
        session.rebuild(&doc).unwrap();
        if let WorkflowOperation::Bore { radius, .. } = &mut doc.operations[2] {
            *radius = 2. * scale;
        }
        let result = session.rebuild(&doc).unwrap();
        assert_eq!(result.stats.reused_operations, 2);
        assert_eq!(
            result.solid.mesh_json(0.05 * scale, t).unwrap(),
            doc.rebuild().unwrap().mesh_json(0.05 * scale, t).unwrap()
        );
    }
}
#[test]
fn swept_boundary_contacts_and_interior_crossings_are_rejected() {
    let t = Tolerance::new(1e-8).unwrap();
    let outer = [[-40., -30.], [40., -30.], [40., 30.], [-40., 30.]];
    let hole = vec![[-2., -2.], [2., -2.], [2., 2.], [-2., 2.]];
    let tool = BoxBore {
        center: [10., 0.],
        radius: 1.,
        depth: None,
    };
    // Both cap footprints are valid, but the moving opening crosses the tool halfway.
    assert!(
        subtract_skew_polygon_region_prism_bores(&outer, &[hole], 24., [20., 0.], &[tool], t)
            .is_err()
    );
    for radius in [10., 10. - 5e-8, 11.] {
        assert!(subtract_skew_polygon_region_prism_bores(
            &outer,
            &[],
            24.,
            [20., 0.],
            &[BoxBore {
                center: [-10., 0.],
                radius,
                depth: None
            }],
            t
        )
        .is_err());
    }
    assert!(subtract_skew_polygon_region_prism_bores(
        &outer,
        &[],
        24.,
        [20., 0.],
        &[BoxBore {
            radius: 9.,
            center: [-10., 0.],
            depth: None
        }],
        t
    )
    .is_ok());
    assert!(
        subtract_skew_polygon_region_prism_bores(&outer, &[], 24., [f64::NAN, 0.], &[], t).is_err()
    );
    assert!(subtract_skew_polygon_region_prism_bores(
        &outer,
        &[],
        24.,
        [20., 0.],
        &[BoxBore {
            depth: Some(8.),
            ..tool
        }],
        t
    )
    .is_err());
    let concave = [
        [-30., -20.],
        [-10., -20.],
        [-10., 10.],
        [10., 10.],
        [10., -20.],
        [30., -20.],
        [30., 20.],
        [-30., 20.],
    ];
    assert!(subtract_skew_polygon_region_prism_bores(
        &concave,
        &[],
        24.,
        [40., 0.],
        &[BoxBore {
            center: [20., 0.],
            ..tool
        }],
        t
    )
    .is_err());
}

#[test]
fn reversed_winding_negative_offsets_and_failed_edits_preserve_geometry() {
    let t = Tolerance::new(1e-8).unwrap();
    let outer = vec![[-40., -30.], [40., -30.], [40., 30.], [-40., 30.]];
    let reverse: Vec<_> = outer.iter().rev().copied().collect();
    let tool = BoxBore {
        center: [0., 0.],
        radius: 4.,
        depth: None,
    };
    for offset in [[-18., 12.], [1e-12, 0.]] {
        for boundary in [&outer, &reverse] {
            let solid =
                subtract_skew_polygon_region_prism_bores(boundary, &[], 24., offset, &[tool], t)
                    .unwrap();
            assert!(
                (solid.volume().unwrap() - (4800. - 16. * std::f64::consts::PI) * 24.).abs() < 1e-8
            );
            solid.validate(t).unwrap();
        }
    }
    let mut doc: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-skew-bores-example.json")).unwrap();
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&doc).unwrap();
    let original = doc.clone();
    if let WorkflowOperation::Bore { center, .. } = &mut doc.operations[1] {
        *center = [-28., 0.];
    }
    let diagnostic = session.rebuild(&doc).unwrap_err();
    assert_eq!(diagnostic.operation_id.as_deref(), Some("bore-1"));
    assert!(diagnostic.measured_clearance.unwrap() <= diagnostic.required_clearance.unwrap());
    let recovered = session.rebuild(&original).unwrap();
    assert_eq!(recovered.stats.rebuilt_operations, 0);
    assert!(std::sync::Arc::ptr_eq(&accepted.solid, &recovered.solid));
}
