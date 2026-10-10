use hagane::*;
use serde_json::{json, Value};
use std::sync::Arc;
fn document() -> WorkflowDocument {
    let q = std::f64::consts::FRAC_PI_2;
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":1e-10},"operations":[{"kind":"arc_line_extrusion","id":"capsule","height":8.,"outer":[{"kind":"line","start":[15.,15.],"end":[35.,15.]},{"kind":"arc","center":[35.,20.],"radius":5.,"start_angle":-q,"sweep":q},{"kind":"arc","center":[35.,20.],"radius":5.,"start_angle":0.,"sweep":q},{"kind":"line","start":[35.,25.],"end":[15.,25.]},{"kind":"arc","center":[15.,20.],"radius":5.,"start_angle":q,"sweep":q},{"kind":"arc","center":[15.,20.],"radius":5.,"start_angle":2.*q,"sweep":q}],"holes":[[{"kind":"line","start":[17.,19.],"end":[19.,19.]},{"kind":"line","start":[19.,19.],"end":[19.,21.]},{"kind":"line","start":[19.,21.],"end":[17.,21.]},{"kind":"line","start":[17.,21.],"end":[17.,19.]}]]},{"kind":"bore","id":"bore-1","input":"capsule","mode":"through","center":[25.,20.],"radius":1.25},{"kind":"bore","id":"bore-2","input":"bore-1","mode":"through","center":[32.,20.],"radius":0.7}]})).unwrap()
}
#[test]
fn noncentered_exact_profile_opening_and_bores_display_and_step() {
    let d = document();
    let body = d.rebuild().unwrap();
    body.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    let expected = (200. + 25. * std::f64::consts::PI
        - 4.
        - std::f64::consts::PI * (1.25f64.powi(2) + 0.7f64.powi(2)))
        * 8.;
    assert!((body.volume().unwrap() - expected).abs() < 1e-10);
    assert_eq!(body.vertices.len(), 36);
    assert_eq!(body.edges.len(), 54);
    assert_eq!(body.shell.faces.len(), 20);
    assert_eq!(
        body.shell
            .faces
            .iter()
            .filter(|f| f.wires.len() == 4)
            .count(),
        2
    );
    let text = serde_json::to_string(&d).unwrap();
    let report: Value = serde_json::from_str(&evaluate_workflow_json(&text).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    let exported: Value =
        serde_json::from_str(&export_workflow_step_mm_json(&text).unwrap()).unwrap();
    let step = exported["step"].as_str().unwrap();
    let imported = import_step_bounded_analytic_mm(step, Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((imported.volume().unwrap() - expected).abs() < 1e-10);
    for f in &imported.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            let edge = &imported.edges[c.edge];
            let r = edge.curve.range();
            for p in [0., 0.39, 1.] {
                let p = r[0] + p * (r[1] - r[0]);
                let uv = c.pcurve.evaluate(p);
                assert!((edge.curve.evaluate(p) - f.surface.evaluate(uv[0], uv[1])).norm() < 1e-6);
            }
        }
    }
}
#[test]
fn unchanged_profiles_reuse_geometry_and_bad_profile_edits_preserve_cache() {
    let mut session = WorkflowSession::new();
    let d = document();
    let first = session.rebuild(&d).unwrap();
    let same = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&first.solid, &same.solid));
    assert_eq!(same.stats.rebuilt_operations, 0);
    let mut edited = d.clone();
    if let WorkflowOperation::Bore { radius, .. } = &mut edited.operations[2] {
        *radius = 0.8;
    }
    let accepted = session.rebuild(&edited).unwrap();
    assert_eq!(accepted.stats.reused_operations, 2);
    assert_eq!(accepted.stats.rebuilt_operation_ids, vec!["bore-2"]);
    let mut bad = edited.clone();
    if let WorkflowOperation::ArcLineExtrusion { outer, .. } = &mut bad.operations[0] {
        if let WorkflowProfileSegment::Line { start, .. } = &mut outer[0] {
            start[0] += 0.1;
        }
    }
    assert!(session.rebuild(&bad).is_err());
    let failed: Value = serde_json::from_str(
        &session
            .evaluate_json(&serde_json::to_string(&bad).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(failed["ok"], false);
    assert!(failed.get("mesh").is_none());
    let stable = session.rebuild(&edited).unwrap();
    assert!(Arc::ptr_eq(&stable.solid, &accepted.solid));
    assert_eq!(stable.stats.rebuilt_operations, 0);
    if let WorkflowOperation::ArcLineExtrusion { height, .. } = &mut edited.operations[0] {
        *height = 9.;
    }
    assert_eq!(session.rebuild(&edited).unwrap().stats.reused_operations, 0);
}
#[test]
fn scope_singular_contacts_signed_orientation_and_resources_are_checked() {
    let mut d = document();
    if let WorkflowOperation::Bore { mode, depth, .. } = &mut d.operations[1] {
        *mode = WorkflowBoreMode::Blind;
        *depth = Some(2.);
    }
    assert_eq!(
        d.rebuild().unwrap_err().code,
        "arc_line_blind_bore_unsupported"
    );
    for sweep in [0., std::f64::consts::PI, f64::NAN] {
        let mut d = document();
        if let WorkflowOperation::ArcLineExtrusion { outer, .. } = &mut d.operations[0] {
            if let WorkflowProfileSegment::Arc { sweep: s, .. } = &mut outer[1] {
                *s = sweep;
            }
        }
        assert!(d.rebuild().is_err());
    }
    let mut d = document();
    if let WorkflowOperation::Bore { center, radius, .. } = &mut d.operations[1] {
        *center = [18., 20.];
        *radius = 0.25;
    }
    assert!(d.rebuild().is_err());
    let mut d = document();
    if let WorkflowOperation::ArcLineExtrusion { outer, .. } = &mut d.operations[0] {
        outer.reverse();
        for s in outer {
            match s {
                WorkflowProfileSegment::Line { start, end } => std::mem::swap(start, end),
                WorkflowProfileSegment::Arc {
                    start_angle, sweep, ..
                } => {
                    *start_angle += *sweep;
                    *sweep = -*sweep;
                }
            }
        }
    }
    assert!(
        (d.rebuild().unwrap().volume().unwrap() - document().rebuild().unwrap().volume().unwrap())
            .abs()
            < 1e-10
    );
    let mut d = document();
    if let WorkflowOperation::ArcLineExtrusion { holes, .. } = &mut d.operations[0] {
        let ring = holes[0].clone();
        *holes = vec![ring; 16];
    }
    assert_eq!(d.rebuild().unwrap_err().code, "unsupported_history");
    let mut d = document();
    d.tolerance.relative = 0.05;
    assert!(d.rebuild().is_err());
}
