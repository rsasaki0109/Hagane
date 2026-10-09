use hagane::*;
use serde_json::{json, Value};
fn fixture() -> WorkflowDocument {
    serde_json::from_str(include_str!("../docs/workflow-skew-extrusion-example.json")).unwrap()
}
#[test]
fn skew_history_preserves_exact_volume_openings_and_closure_across_scales() {
    for scale in [1e-6, 1., 1000.] {
        let mut doc = fixture();
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
        let solid = doc.rebuild().unwrap();
        let t = Tolerance::new(doc.tolerance.linear).unwrap();
        solid.validate(t).unwrap();
        let expected = 4408. * 24. * scale.powi(3);
        assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
        assert_eq!(solid.shell.faces.len(), 14);
        let bounds = solid.bounds();
        assert!((bounds.min - Point3::new(-40., -42., -12.) * scale).norm() < t.linear);
        assert!((bounds.max - Point3::new(58., 30., 12.) * scale).norm() < t.linear);
        let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
        for (p, expected) in [
            (Point3::new(9., -6., 0.), PointLocation::Inside),
            (Point3::new(-19., -6., 0.), PointLocation::Outside),
            (Point3::new(-13., -6., 0.), PointLocation::Boundary),
        ] {
            assert_eq!(
                classify_point_in_solid(&solid, p * scale, policy).unwrap(),
                expected
            );
        }
        assert!(
            (solid.tessellate(0.05 * scale, t).unwrap().signed_volume() - expected).abs()
                < expected * 1e-12
        );
        let mut session = WorkflowSession::new();
        assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 1);
        assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
        if let WorkflowOperation::Extrusion { offset, .. } = &mut doc.operations[0] {
            offset[0] = 24. * scale;
        }
        let edit = session.rebuild(&doc).unwrap();
        assert_eq!(edit.stats.reused_operations, 0);
        assert_eq!(
            edit.solid.mesh_json(0.05 * scale, t).unwrap(),
            doc.rebuild().unwrap().mesh_json(0.05 * scale, t).unwrap()
        );
    }
}
#[test]
fn skew_blind_tools_and_invalid_offsets_are_rejected_without_committing_or_ignoring() {
    let mut session = WorkflowSession::new();
    let mut doc = fixture();
    session.rebuild(&doc).unwrap();
    doc.operations.push(WorkflowOperation::Bore {
        id: "bore-1".into(),
        input: "extrusion-1".into(),
        mode: WorkflowBoreMode::Blind,
        center: [0., 0.],
        radius: 4.,
        depth: Some(8.),
    });
    let diagnostic = session.rebuild(&doc).unwrap_err();
    assert_eq!(diagnostic.code, "unsupported_skew_bore");
    assert_eq!(diagnostic.operation_id.as_deref(), Some("bore-1"));
    assert_eq!(diagnostic.category, "unsupported");
    let mut nearly_vertical = doc.clone();
    if let WorkflowOperation::Extrusion { offset, .. } = &mut nearly_vertical.operations[0] {
        *offset = [1e-12, 0.];
    }
    assert_eq!(
        session.rebuild(&nearly_vertical).unwrap_err().code,
        "unsupported_skew_bore"
    );
    doc.operations.pop();
    assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
    if let WorkflowOperation::Extrusion { offset, .. } = &mut doc.operations[0] {
        offset[0] = f64::NAN;
    }
    assert_eq!(session.rebuild(&doc).unwrap_err().code, "invalid_offset");
    let mut value = serde_json::to_value(fixture()).unwrap();
    value["operations"][0]["offset"] = json!([1.]);
    let report: Value =
        serde_json::from_str(&evaluate_workflow_json(&value.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], false);
    assert!(report.get("mesh").is_none());
    let old: WorkflowDocument = serde_json::from_str(include_str!(
        "../docs/workflow-extrusion-openings-example.json"
    ))
    .unwrap();
    assert!(serde_json::to_value(&old).unwrap()["operations"][0]
        .get("offset")
        .is_none());
    old.rebuild().unwrap();
}
