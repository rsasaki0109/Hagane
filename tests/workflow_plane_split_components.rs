use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn document(side: &str) -> WorkflowDocument {
    serde_json::from_value(json!({
        "schema_version":1,"units":"mm",
        "tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},
        "operations":[
            {"kind":"rounded_box","id":"stock","size":[80.,60.,20.],"corner_radius":8.},
            {"kind":"bore","id":"opening","input":"stock","mode":"through","center":[0.,0.],"radius":8.},
            {"kind":"plane_split","id":"cut","input":"opening","offset":0.,"normal_angle":0.37,"side":side}
        ]
    })).unwrap()
}

fn check_closed(solid: &Solid, expected: f64) {
    solid.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((solid.volume().unwrap() - expected).abs() < 1e-8);
    assert_eq!(
        solid.vertices.len() as isize - solid.edges.len() as isize
            + solid.shell.faces.len() as isize,
        2
    );
    let mut uses = vec![Vec::new(); solid.edges.len()];
    for face in &solid.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].push(c.forward == (face.orientation == 1));
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0] != u[1]));
    let step = export_step_bounded_analytic_mm(solid, 1e-6).unwrap();
    let restored = import_step_bounded_analytic_mm(&step, Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((restored.volume().unwrap() - expected).abs() < 1e-8);
}

#[test]
fn crossed_opening_is_an_actual_notch_on_both_selected_workflow_sides() {
    let expected =
        (96000. - 1280. * (4. - std::f64::consts::PI) - 1280. * std::f64::consts::PI) / 2.;
    for side in ["negative", "positive"] {
        let d = document(side);
        let solid = d.rebuild().unwrap();
        check_closed(&solid, expected);
        // A crossed opening joins the outer trim, rather than remaining a hole.
        let caps: Vec<_> = solid
            .shell
            .faces
            .iter()
            .filter(
                |f| matches!(f.surface, Surface::Plane { u, v, .. } if u.cross(v).z.abs() > 0.99),
            )
            .collect();
        assert_eq!(caps.len(), 2);
        assert!(caps.iter().all(|f| f.wires.len() == 1));
        let angle: f64 = 0.37;
        let (sin, cos) = angle.sin_cos();
        assert!(solid.vertices.iter().all(|v| {
            let gap = cos * v.point.x + sin * v.point.y;
            if side == "negative" {
                gap <= 1e-6
            } else {
                gap >= -1e-6
            }
        }));
        let report: Value = serde_json::from_str(
            &evaluate_workflow_json(&serde_json::to_string(&d).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(report["ok"], true);
        let export: Value = serde_json::from_str(
            &export_workflow_step_mm_json(&serde_json::to_string(&d).unwrap()).unwrap(),
        )
        .unwrap();
        let exported = import_step_bounded_analytic_mm(
            export["step"].as_str().unwrap(),
            Tolerance::new(1e-6).unwrap(),
        )
        .unwrap();
        check_closed(&exported, expected);
    }
}

#[test]
fn crossed_cut_cache_and_later_bore_use_retained_material_atomically() {
    let mut d = document("negative");
    let mut session = WorkflowSession::new();
    let cut = session.rebuild(&d).unwrap();
    let expected = cut.solid.volume().unwrap() - 80. * std::f64::consts::PI;
    d.operations.push(WorkflowOperation::Bore {
        id: "later".into(),
        input: "cut".into(),
        mode: WorkflowBoreMode::Through,
        entry: WorkflowBoreEntry::Top,
        center: [-20., 0.],
        radius: 2.,
        depth: None,
    });
    let accepted = session.rebuild(&d).unwrap();
    assert_eq!(
        (
            accepted.stats.reused_operations,
            accepted.stats.rebuilt_operations
        ),
        (3, 1)
    );
    accepted
        .solid
        .validate(Tolerance::new(1e-6).unwrap())
        .unwrap();
    assert!((accepted.solid.volume().unwrap() - expected).abs() < 1e-8);
    assert_eq!(
        accepted.solid.vertices.len() as isize - accepted.solid.edges.len() as isize
            + accepted
                .solid
                .shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>(),
        0
    );
    let mut invalid = d.clone();
    if let WorkflowOperation::PlaneSplit { side, .. } = &mut invalid.operations[2] {
        *side = WorkflowSplitSide::Positive;
    }
    let error = session.rebuild(&invalid).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("later"));
    assert_eq!(error.category, "invalid_input");
    let unchanged = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &unchanged.solid));
    assert_eq!(
        (
            unchanged.stats.reused_operations,
            unchanged.stats.rebuilt_operations
        ),
        (4, 0)
    );
    if let WorkflowOperation::PlaneSplit { offset, .. } = &mut d.operations[2] {
        *offset = 3.;
    }
    let edited = session.rebuild(&d).unwrap();
    assert_eq!(
        (
            edited.stats.reused_operations,
            edited.stats.rebuilt_operations
        ),
        (2, 2)
    );
    assert_eq!(
        format!("{:?}", edited.solid),
        format!("{:?}", d.rebuild().unwrap())
    );
    d.operations.truncate(2);
    let source = session.rebuild(&d).unwrap();
    assert_eq!(
        (
            source.stats.reused_operations,
            source.stats.rebuilt_operations
        ),
        (2, 0)
    );
    assert!((source.solid.volume().unwrap() - 2. * cut.solid.volume().unwrap()).abs() < 1e-8);
}

#[test]
fn multi_body_selected_side_rejects_without_choosing_or_losing_a_component() {
    let points = [
        [-5., -4.],
        [6., -4.],
        [6., 6.],
        [2., 6.],
        [2., 0.],
        [-2., 0.],
        [-2., 6.],
        [-6., 6.],
        [-6., -3.],
    ];
    let mut outer: Vec<Value> = points
        .windows(2)
        .map(|p| json!({"kind":"line","start":p[0],"end":p[1]}))
        .collect();
    outer.push(json!({"kind":"arc","center":[-5.,-3.],"radius":1.,
        "start_angle":std::f64::consts::PI,"sweep":std::f64::consts::FRAC_PI_2}));
    let mut d: WorkflowDocument = serde_json::from_value(json!({
        "schema_version":1,"units":"mm",
        "tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},
        "operations":[
            {"kind":"arc_line_extrusion","id":"stock","height":5.,"outer":outer},
            {"kind":"plane_split","id":"cut","input":"stock","offset":2.,
                "normal_angle":std::f64::consts::FRAC_PI_2,"side":"negative"}
        ]
    }))
    .unwrap();
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&d).unwrap();
    accepted
        .solid
        .validate(Tolerance::new(1e-6).unwrap())
        .unwrap();
    assert!(accepted
        .solid
        .vertices
        .iter()
        .all(|v| v.point.y <= 2. + 1e-6));
    let step_before = export_step_bounded_analytic_mm(&accepted.solid, 1e-6).unwrap();
    let good = d.clone();
    if let WorkflowOperation::PlaneSplit { side, .. } = &mut d.operations[1] {
        *side = WorkflowSplitSide::Positive;
    }
    let error = session.rebuild(&d).unwrap_err();
    assert_eq!(error.category, "unsupported");
    assert_eq!(error.operation_id.as_deref(), Some("cut"));
    assert!(error.message.contains("exactly one connected component"));
    let retained = session.rebuild(&good).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &retained.solid));
    assert_eq!(
        (
            retained.stats.reused_operations,
            retained.stats.rebuilt_operations
        ),
        (2, 0)
    );
    assert_eq!(
        step_before,
        export_step_bounded_analytic_mm(&retained.solid, 1e-6).unwrap()
    );
}

#[test]
fn microscopic_crossed_opening_retains_child_scale_volume_accuracy() {
    let scale = 1e-5;
    let policy = GeometryTolerance::new(1e-11, 1e-10, 0.).unwrap();
    let stock = make_box(
        BoxSpec {
            min: Point3::new(-40. * scale, -30. * scale, 0.),
            size: Vec3::new(80. * scale, 60. * scale, 20. * scale),
        },
        policy.absolute(),
    )
    .unwrap();
    let rounded = fillet_parallel_box_edges(
        &stock,
        &[
            (8, 8. * scale),
            (9, 8. * scale),
            (10, 8. * scale),
            (11, 8. * scale),
        ],
        policy,
    )
    .unwrap();
    let opening =
        bore_normal_arc_line_prism(rounded.solid(), Point3::new(0., 0., 0.), 8. * scale, policy)
            .unwrap();
    let (sin, cos) = 0.37_f64.sin_cos();
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(0., 0., 1.),
        v: Vec3::new(sin, -cos, 0.),
    };
    let split =
        split_normal_arc_line_prism_by_plane_components(opening.kept(), &plane, policy).unwrap();
    assert_eq!(split.sections().len(), 2);
    assert_eq!((split.negative().len(), split.positive().len()), (1, 1));
    let expected = 45440. * scale.powi(3);
    for child in split.negative().iter().chain(split.positive()) {
        child.validate(policy.absolute()).unwrap();
        assert!((child.volume().unwrap() - expected).abs() <= 8192. * f64::EPSILON * expected);
    }
}
