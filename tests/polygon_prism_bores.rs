use hagane::*;
use serde_json::{json, Value};
const OUTER: [[f64; 2]; 8] = [
    [-40., -20.],
    [-30., -30.],
    [30., -30.],
    [40., -20.],
    [40., 20.],
    [30., 30.],
    [-30., 30.],
    [-40., 20.],
];
#[test]
fn exact_prism_cuts_have_analytic_volume_closed_shared_topology_and_real_floor() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
        let outer: Vec<_> = OUTER.iter().map(|p| [p[0] * scale, p[1] * scale]).collect();
        let bores = [
            BoxBore {
                center: [0., 0.],
                radius: 14. * scale,
                depth: Some(16. * scale),
            },
            BoxBore {
                center: [24. * scale, 0.],
                radius: 4. * scale,
                depth: None,
            },
        ];
        for points in [outer.clone(), outer.iter().rev().copied().collect()] {
            let s = subtract_polygon_prism_bores(&points, 24. * scale, &bores, t).unwrap();
            s.validate(t).unwrap();
            assert_eq!(
                (s.shell.faces.len(), s.edges.len(), s.vertices.len()),
                (13, 30, 20)
            );
            let expected =
                (4600. * 24. - std::f64::consts::PI * (196. * 16. + 16. * 24.)) * scale.powi(3);
            assert!((s.volume().unwrap() - expected).abs() < expected * 1e-12);
            for (point, expected) in [
                (Point3::new(0., 0., 0.), PointLocation::Outside),
                (Point3::new(0., 0., -8.), PointLocation::Inside),
                (Point3::new(0., 0., -4.), PointLocation::Boundary),
                (Point3::new(24., 0., -8.), PointLocation::Outside),
                (Point3::new(38., 28., 0.), PointLocation::Outside),
                (Point3::new(-24., 0., 0.), PointLocation::Inside),
            ] {
                assert_eq!(
                    classify_point_in_solid(&s, point * scale, policy).unwrap(),
                    expected
                );
            }
            let mesh = s.tessellate(0.01 * scale, t).unwrap();
            assert!(
                (mesh.signed_volume() - expected).abs()
                    < (std::f64::consts::TAU * 0.01 * (14. * 16. + 4. * 24.)) * scale.powi(3)
            );
        }
    }
}
#[test]
fn concave_profile_rejects_tools_in_notch_and_contact_inside_its_bounding_box() {
    let outer = [
        [-40., -30.],
        [40., -30.],
        [40., 0.],
        [0., 0.],
        [0., 30.],
        [-40., 30.],
    ];
    let t = Tolerance::default();
    let good = BoxBore {
        center: [-20., 10.],
        radius: 5.,
        depth: None,
    };
    let s = subtract_polygon_prism_bores(&outer, 24., &[good], t).unwrap();
    s.validate(t).unwrap();
    assert!((s.volume().unwrap() - (3600. * 24. - std::f64::consts::PI * 25. * 24.)).abs() < 1e-8);
    for center in [[20., 10.], [-5., 10.], [-5. - 5e-8, 10.]] {
        assert!(
            subtract_polygon_prism_bores(&outer, 24., &[BoxBore { center, ..good }], t).is_err()
        );
    }
    assert!(subtract_polygon_prism_bores(
        &[[-1., -1.], [1., 1.], [-1., 1.], [1., -1.]],
        24.,
        &[],
        t
    )
    .is_err());
    assert!(subtract_polygon_prism_bores(&OUTER, -24., &[], t).is_err());
    assert!(subtract_polygon_prism_bores(
        &OUTER,
        24.,
        &[BoxBore {
            depth: Some(24.),
            ..good
        }],
        t
    )
    .is_err());
    assert!(subtract_polygon_prism_bores(&OUTER, 24., &[good, good], t).is_err());
}
#[test]
fn extrusion_history_is_editable_incremental_and_rejects_bad_profiles_without_commit() {
    let mut doc: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-extrusion-example.json")).unwrap();
    let mut session = WorkflowSession::new();
    let first = session.rebuild(&doc).unwrap();
    assert_eq!(first.stats.rebuilt_operations, 3);
    if let WorkflowOperation::Extrusion { height, .. } = &mut doc.operations[0] {
        *height = 32.;
    }
    let edit = session.rebuild(&doc).unwrap();
    assert_eq!(edit.stats.reused_operations, 0);
    assert_eq!(
        edit.solid.mesh_json(0.05, Tolerance::default()).unwrap(),
        doc.rebuild()
            .unwrap()
            .mesh_json(0.05, Tolerance::default())
            .unwrap()
    );
    if let WorkflowOperation::Bore { radius, .. } = &mut doc.operations[2] {
        *radius = 5.;
    }
    assert_eq!(session.rebuild(&doc).unwrap().stats.reused_operations, 2);
    let mut bad = doc.clone();
    if let WorkflowOperation::Extrusion { outer, .. } = &mut bad.operations[0] {
        outer[1] = outer[0];
    }
    assert_eq!(
        session.rebuild(&bad).unwrap_err().operation_id.as_deref(),
        Some("extrusion-1")
    );
    assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
    let mut value = serde_json::to_value(&doc).unwrap();
    value["operations"][1]["center"] = json!([38., 28.]);
    let rejected: Value =
        serde_json::from_str(&evaluate_workflow_json(&value.to_string()).unwrap()).unwrap();
    assert_eq!(rejected["diagnostic"]["code"], "side_clearance");
    assert_eq!(rejected["diagnostic"]["operation_id"], "bore-1");
    assert!(rejected.get("mesh").is_none());
    assert_eq!(
        rejected["candidate_segments"].as_array().unwrap().len(),
        100
    );
}
#[test]
fn translated_world_xy_profiles_and_root_only_history_are_exact() {
    let t = Tolerance::default();
    let shifted: Vec<_> = OUTER.iter().map(|p| [p[0] + 100., p[1] - 70.]).collect();
    let solid = subtract_polygon_prism_bores(
        &shifted,
        24.,
        &[BoxBore {
            center: [100., -70.],
            radius: 14.,
            depth: Some(16.),
        }],
        t,
    )
    .unwrap();
    assert!(
        (solid.volume().unwrap() - (4600. * 24. - std::f64::consts::PI * 196. * 16.)).abs() < 1e-8
    );
    assert_eq!(
        classify_point_in_solid(
            &solid,
            Point3::new(100., -70., -8.),
            GeometryTolerance::default()
        )
        .unwrap(),
        PointLocation::Inside
    );
    let mut doc: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-extrusion-example.json")).unwrap();
    let mut session = WorkflowSession::new();
    session.rebuild(&doc).unwrap();
    doc.operations.truncate(1);
    let root = session.rebuild(&doc).unwrap();
    assert_eq!(root.stats.rebuilt_operations, 0);
    assert_eq!(root.stats.reused_operations, 1);
    assert!((root.solid.volume().unwrap() - 110400.).abs() < 1e-8);
}
#[test]
fn polygon_openings_with_mixed_cuts_preserve_volume_material_and_oriented_topology() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
        let outer: Vec<_> = OUTER.iter().map(|p| [p[0] * scale, p[1] * scale]).collect();
        let hole: Vec<_> = [[-34., -8.], [-22., -8.], [-22., 8.], [-34., 8.]]
            .iter()
            .map(|p| [p[0] * scale, p[1] * scale])
            .collect();
        let bores = [
            BoxBore {
                center: [0., 0.],
                radius: 14. * scale,
                depth: Some(16. * scale),
            },
            BoxBore {
                center: [24. * scale, 0.],
                radius: 4. * scale,
                depth: None,
            },
        ];
        for reversed in [false, true] {
            let mut outer = outer.clone();
            let mut hole = hole.clone();
            if reversed {
                outer.reverse();
                hole.reverse();
            }
            let solid =
                subtract_polygon_region_prism_bores(&outer, &[hole], 24. * scale, &bores, t)
                    .unwrap();
            solid.validate(t).unwrap();
            assert_eq!(
                (
                    solid.shell.faces.len(),
                    solid.edges.len(),
                    solid.vertices.len()
                ),
                (17, 42, 28)
            );
            let expected = ((4600. - 192.) * 24. - std::f64::consts::PI * (196. * 16. + 16. * 24.))
                * scale.powi(3);
            assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
            for (p, expected) in [
                (Point3::new(-28., 0., -8.), PointLocation::Outside),
                (Point3::new(-22., 0., 0.), PointLocation::Boundary),
                (Point3::new(-18., 0., 0.), PointLocation::Inside),
                (Point3::new(0., 0., -8.), PointLocation::Inside),
                (Point3::new(0., 0., 0.), PointLocation::Outside),
                (Point3::new(24., 0., -8.), PointLocation::Outside),
            ] {
                assert_eq!(
                    classify_point_in_solid(&solid, p * scale, policy).unwrap(),
                    expected
                );
            }
            let mesh = solid.tessellate(0.01 * scale, t).unwrap();
            assert!(
                (mesh.signed_volume() - expected).abs()
                    < std::f64::consts::TAU * 0.01 * (14. * 16. + 4. * 24.) * scale.powi(3)
            );
        }
    }
}
#[test]
fn profile_opening_contact_overlap_nesting_and_tools_in_void_are_rejected() {
    let t = Tolerance::default();
    let opening = vec![[-34., -8.], [-22., -8.], [-22., 8.], [-34., 8.]];
    for center in [[-8., 0.], [-8. - 5e-8, 0.], [-28., 0.]] {
        assert!(subtract_polygon_region_prism_bores(
            &OUTER,
            std::slice::from_ref(&opening),
            24.,
            &[BoxBore {
                center,
                radius: 14.,
                depth: None
            }],
            t
        )
        .is_err());
    }
    for holes in [
        vec![opening.clone(), opening.clone()],
        vec![
            opening.clone(),
            vec![[-30., -4.], [-26., -4.], [-26., 4.], [-30., 4.]],
        ],
        vec![vec![[-40., -8.], [-22., -8.], [-22., 8.], [-40., 8.]]],
        vec![vec![
            [-40. + 5e-8, -8.],
            [-22., -8.],
            [-22., 8.],
            [-40. + 5e-8, 8.],
        ]],
    ] {
        assert!(subtract_polygon_region_prism_bores(&OUTER, &holes, 24., &[], t).is_err());
    }
    assert!(
        subtract_polygon_region_prism_bores(&OUTER, &vec![opening.clone(); 65], 24., &[], t)
            .is_err()
    );
    assert!(subtract_polygon_region_prism_bores(
        &OUTER,
        std::slice::from_ref(&opening),
        24.,
        &[BoxBore {
            center: [-28., 0.],
            radius: 2.,
            depth: Some(8.)
        }],
        t
    )
    .is_err());
    // A circular tool can enclose an opening despite its center being in material.
    let rectangle = [[-50., -50.], [50., -50.], [50., 50.], [-50., 50.]];
    assert!(subtract_polygon_region_prism_bores(
        &rectangle,
        &[vec![[-3., -3.], [3., -3.], [3., 3.], [-3., 3.]]],
        24.,
        &[BoxBore {
            center: [10., 0.],
            radius: 15.,
            depth: None
        }],
        t
    )
    .is_err());
    let doc: WorkflowDocument = serde_json::from_str(include_str!(
        "../docs/workflow-extrusion-openings-example.json"
    ))
    .unwrap();
    let mut session = WorkflowSession::new();
    session.rebuild(&doc).unwrap();
    let mut bad = serde_json::to_value(&doc).unwrap();
    bad["operations"][1]["center"] = json!([-8., 0.]);
    let rejected: Value =
        serde_json::from_str(&session.evaluate_json(&bad.to_string()).unwrap()).unwrap();
    assert_eq!(rejected["diagnostic"]["code"], "profile_hole_clearance");
    assert_eq!(rejected["diagnostic"]["operation_id"], "bore-1");
    assert_eq!(rejected["diagnostic"]["measured_clearance"], 0.);
    assert!(rejected["diagnostic"]["message"]
        .as_str()
        .unwrap()
        .contains("hole 0"));
    assert!(rejected.get("mesh").is_none());
    assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
    let mut edit = doc.clone();
    if let WorkflowOperation::Extrusion { holes, .. } = &mut edit.operations[0] {
        holes[0][0][0] = -35.;
        holes[0][3][0] = -35.;
    }
    let result = session.rebuild(&edit).unwrap();
    assert_eq!(result.stats.reused_operations, 0);
    assert_eq!(
        result.solid.mesh_json(0.05, t).unwrap(),
        edit.rebuild().unwrap().mesh_json(0.05, t).unwrap()
    );
}
#[test]
fn multiple_and_concave_openings_use_actual_boundaries_not_boxes() {
    let t = Tolerance::default();
    let holes = vec![
        vec![[-34., -8.], [-22., -8.], [-22., 8.], [-34., 8.]],
        vec![[-6., 20.], [6., 20.], [6., 24.], [-6., 24.]],
    ];
    let bores = [
        BoxBore {
            center: [0., 0.],
            radius: 14.,
            depth: Some(16.),
        },
        BoxBore {
            center: [24., 0.],
            radius: 4.,
            depth: None,
        },
    ];
    let multiple = subtract_polygon_region_prism_bores(&OUTER, &holes, 24., &bores, t).unwrap();
    multiple.validate(t).unwrap();
    assert!(
        (multiple.volume().unwrap()
            - ((4600. - 240.) * 24. - std::f64::consts::PI * (196. * 16. + 16. * 24.)))
            .abs()
            < 1e-8
    );
    let concave = vec![vec![
        [-34., -8.],
        [-22., -8.],
        [-22., 0.],
        [-28., 0.],
        [-28., 8.],
        [-34., 8.],
    ]];
    let solid = subtract_polygon_region_prism_bores(
        &OUTER,
        &concave,
        24.,
        &[BoxBore {
            center: [-25., 4.],
            radius: 1.,
            depth: None,
        }],
        t,
    )
    .unwrap();
    solid.validate(t).unwrap();
    assert!(
        (solid.volume().unwrap() - ((4600. - 144.) * 24. - std::f64::consts::PI * 24.)).abs()
            < 1e-8
    );
    assert_eq!(
        classify_point_in_solid(
            &solid,
            Point3::new(-31., 4., 0.),
            GeometryTolerance::default()
        )
        .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(
            &solid,
            Point3::new(-25., 4., 0.),
            GeometryTolerance::default()
        )
        .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(
            &solid,
            Point3::new(-25., 6., 0.),
            GeometryTolerance::default()
        )
        .unwrap(),
        PointLocation::Inside
    );
}
