use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

fn euler(s: &Solid) -> isize {
    s.vertices.len() as isize - s.edges.len() as isize
        + s.shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>()
}
fn check_closed(s: &Solid, scale: f64, world_scale: f64) {
    let tol = Tolerance::new(1e-6 * scale).unwrap();
    s.validate(tol).unwrap();
    let guard = 8192. * f64::EPSILON * world_scale;
    let mut uses = vec![Vec::new(); s.edges.len()];
    for (fi, f) in s.shell.faces.iter().enumerate() {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push((fi, c.forward == (f.orientation == 1)));
            let curve = &s.edges[c.edge].curve;
            let range = curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - curve.try_evaluate(t).unwrap())
                    .norm()
                        <= guard
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0].1 != u[1].1));
    let error = 0.02 * scale;
    let mesh = s.tessellate(error, tol).unwrap();
    let mut nodes: Vec<_> = s.vertices.iter().map(|v| v.point).collect();
    for (ei, e) in s.edges.iter().enumerate() {
        if let Curve::Arc { radius, sweep, .. } = e.curve {
            let fi = uses[ei]
                .iter()
                .map(|u| u.0)
                .find(|&i| matches!(s.shell.faces[i].surface, Surface::FramedCylinder { .. }))
                .unwrap();
            let n = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
            assert!(radius * (1. - (sweep / (2. * n as f64)).cos()) <= error);
            let range = e.curve.range();
            for k in 1..n {
                nodes.push(
                    e.curve
                        .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / n as f64)
                        .unwrap(),
                );
            }
        }
    }
    let ids: Vec<_> = mesh
        .positions
        .iter()
        .map(|p| {
            let found: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, q)| (**q - *p).norm() <= guard)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(found.len(), 1);
            found[0]
        })
        .collect();
    let mut incidence = BTreeMap::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for k in 0..3 {
            let a = ids[tri[k]];
            let b = ids[tri[(k + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = incidence.entry(key).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
        let midpoint = (p[0] + p[1] + p[2]) * (1. / 3.);
        match s.shell.faces[fi].surface {
            Surface::FramedCylinder { frame, radius, .. } => {
                let q = frame.local_point(midpoint);
                assert!((q.x.hypot(q.y) - radius).abs() <= error + guard);
            }
            Surface::Plane { origin, u, v } => {
                assert!((midpoint - origin).dot(u.cross(v)).abs() <= guard)
            }
            _ => panic!("actual analytic BRep"),
        }
    }
    assert!(incidence.values().all(|&(n, d)| n == 2 && d == 0));
    assert_eq!(
        nodes.len() as isize - incidence.len() as isize + mesh.triangles.len() as isize,
        euler(s)
    );
}

use serde_json::{json, Value};
use std::sync::Arc;

fn circle(center: [f64; 2], radius: f64, phase: f64) -> Vec<Value> {
    (0..4).map(|i|json!({"kind":"arc","center":center,"radius":radius,"start_angle":phase+i as f64*PI/2.,"sweep":PI/2.})).collect()
}
fn typed(value: Value) -> PrismWorkflowDocument {
    serde_json::from_value(value).unwrap()
}
fn document(scale: f64) -> PrismWorkflowDocument {
    typed(
        json!({"schema_version":1,"document_type":"hagane_prism_workflow","units":"mm","tolerance":{"linear":1e-6*scale,"angular":1e-10,"relative":0.},"output":"repeat","display_chord_tolerance":0.05*scale,"operations":[
{"kind":"rounded_box","id":"stock","size":[20.*scale,16.*scale,5.*scale],"corner_radius":2.*scale},
{"kind":"arc_line_extrusion","id":"annulus","outer":circle([0.,0.],4.*scale,0.17),"holes":[circle([0.,0.],2.*scale,0.09)],"height":5.*scale},
{"kind":"boolean","id":"cut","left":"stock","right":"annulus","operation":"difference","axis":[0.,0.,1.]},
{"kind":"select","id":"island","input":"cut","component":1},
{"kind":"arc_line_extrusion","id":"drill","outer":circle([0.4*scale,0.],0.5*scale,0.23),"height":5.*scale},
{"kind":"boolean","id":"repeat","left":"island","right":"drill","operation":"difference","axis":[0.,0.,1.]}]}),
    )
}
fn volume(shape: &PrismWorkflowShape) -> f64 {
    shape.components().iter().map(|b| b.volume().unwrap()).sum()
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 2e-10 * expected.abs(),
        "{actual} vs {expected}"
    );
}
fn check_shape(shape: &PrismWorkflowShape, scale: f64, world: f64) {
    for body in shape.components() {
        check_closed(body, scale, world);
        let step = export_step_bounded_analytic_mm(body, 1e-6 * scale).unwrap();
        let imported =
            import_step_bounded_analytic_mm(&step, Tolerance::new(1e-6 * scale).unwrap()).unwrap();
        close(imported.volume().unwrap(), body.volume().unwrap());
        assert_eq!(euler(&imported), euler(body));
        check_closed(&imported, scale, world);
    }
}
fn snapshots(s: &PrismWorkflowSession, n: usize) -> Vec<Arc<PrismWorkflowShape>> {
    (0..n)
        .map(|i| s.prefix_snapshot(i).unwrap().clone())
        .collect()
}
fn assert_stable(
    s: &PrismWorkflowSession,
    d: &PrismWorkflowDocument,
    nodes: &[Arc<PrismWorkflowShape>],
    history: (usize, usize),
) {
    assert_eq!(s.accepted_document(), Some(d));
    assert_eq!((s.undo_count(), s.redo_count()), history);
    for (i, node) in nodes.iter().enumerate() {
        assert!(Arc::ptr_eq(s.prefix_snapshot(i).unwrap(), node));
    }
}
#[test]
fn explicit_annular_island_selection_and_repeat_boolean_use_actual_closed_bodies() {
    for scale in [1e-4, 1., 10.] {
        let d = document(scale);
        let mut session = PrismWorkflowSession::new();
        let report = session.rebuild(&d).unwrap();
        assert_eq!(report.stats.evaluated_nodes, 6);
        assert_eq!(report.stats.reused_nodes, 0);
        let cut = session.prefix_snapshot(2).unwrap();
        assert_eq!(cut.components().len(), 2);
        let outer = &cut.components()[0];
        let island = &cut.components()[1];
        close(outer.volume().unwrap(), (1520. - 60. * PI) * scale.powi(3));
        close(island.volume().unwrap(), 20. * PI * scale.powi(3));
        assert_eq!(euler(outer), 0);
        assert_eq!(euler(island), 2);
        assert!(Arc::ptr_eq(
            &session.prefix_snapshot(3).unwrap().components()[0],
            island
        ));
        assert_eq!(report.shape.components().len(), 1);
        close(volume(&report.shape), 18.75 * PI * scale.powi(3));
        assert_eq!(euler(&report.shape.components()[0]), 0);
        check_shape(cut, scale, 30. * scale);
        check_shape(&report.shape, scale, 30. * scale);
        let source = session.prefix_snapshot(3).unwrap().components()[0].as_ref();
        let tool = session.prefix_snapshot(4).unwrap().components()[0].as_ref();
        let direct = boolean_normal_prism_regions(
            source,
            tool,
            Vec3::new(0., 0., 1.),
            d.geometry_tol().unwrap(),
        )
        .unwrap();
        assert_eq!(
            format!("{:?}", report.shape.components()[0]),
            format!("{:?}", direct.difference()[0])
        );
    }
    let mut d = document(1.);
    let angle = 0.31_f64;
    for op in &mut d.operations {
        match op {
            PrismWorkflowOperation::RoundedBox { placement, .. }
            | PrismWorkflowOperation::ArcLineExtrusion { placement, .. } => {
                placement.y_angle = angle;
                placement.translation = [12., -3., 5.];
            }
            PrismWorkflowOperation::Boolean { axis, .. } => *axis = [angle.sin(), 0., angle.cos()],
            _ => {}
        }
    }
    let posed = d.rebuild().unwrap();
    close(volume(&posed), 18.75 * PI);
    check_shape(&posed, 1., 40.);
}
fn permute(step: &str) -> String {
    let mut records: Vec<_> = step.lines().filter(|l| l.starts_with('#')).collect();
    records.reverse();
    let mut reordered = String::new();
    let mut used = false;
    for line in step.lines() {
        if line.starts_with('#') {
            if !used {
                reordered.push_str(&records.join("\n"));
                reordered.push('\n');
                used = true;
            }
        } else {
            reordered.push_str(line);
            reordered.push('\n');
        }
    }
    let chars: Vec<_> = reordered.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut quote = false;
    while i < chars.len() {
        if chars[i] == '\'' {
            quote = !quote;
        }
        if chars[i] == '#' && !quote {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let id: usize = chars[start..i].iter().collect::<String>().parse().unwrap();
            out.push_str(&format!("#{}", 10000 - id));
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn reordered_actual_step_stock_reloads_then_partitions_in_its_world_frame() {
    let original = document(1.).rebuild().unwrap();
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let tol = Tolerance::new(1e-6).unwrap();
    let source = original.components()[0]
        .transformed(transform, tol)
        .unwrap();
    let tool = extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: (0..4)
                .map(|i| PlanarSegment::Arc {
                    center: [-0.75, 0.],
                    radius: 0.25,
                    start_angle: 0.13 + i as f64 * PI / 2.,
                    sweep: PI / 2.,
                })
                .collect(),
            holes: vec![],
        },
        5.,
        tol,
    )
    .unwrap()
    .transformed(transform, tol)
    .unwrap();
    let axis = transform.vector(Vec3::new(0., 0., 1.));
    let axis = [axis.x, axis.y, axis.z];
    let d = typed(
        json!({"schema_version":1,"document_type":"hagane_prism_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"next","display_chord_tolerance":0.05,"operations":[{"kind":"step_stock","id":"imported","step":permute(&export_step_bounded_analytic_mm(&source,1e-6).unwrap()),"axis":axis},{"kind":"step_stock","id":"tool","step":export_step_bounded_analytic_mm(&tool,1e-6).unwrap(),"axis":axis},{"kind":"boolean","id":"next","left":"imported","right":"tool","operation":"difference","axis":axis}]}),
    );
    let mut session = PrismWorkflowSession::new();
    let rebuilt = session.rebuild(&d).unwrap();
    close(volume(session.prefix_snapshot(0).unwrap()), 18.75 * PI);
    close(volume(&rebuilt.shape), 18.4375 * PI);
    assert_eq!(euler(&rebuilt.shape.components()[0]), -2);
    check_shape(&rebuilt.shape, 1., 40.);
    let direct = boolean_normal_prism_regions(
        session.prefix_snapshot(0).unwrap().components()[0].as_ref(),
        session.prefix_snapshot(1).unwrap().components()[0].as_ref(),
        Vec3::new(axis[0], axis[1], axis[2]),
        d.geometry_tol().unwrap(),
    )
    .unwrap();
    assert_eq!(
        format!("{:?}", rebuilt.shape.components()[0]),
        format!("{:?}", direct.difference()[0])
    );
}
#[test]
fn cache_reuses_real_prefix_for_output_and_display_edits_and_replays_the_whole_suffix() {
    let d = document(1.);
    let mut session = PrismWorkflowSession::new();
    let first = session.rebuild(&d).unwrap();
    let nodes = snapshots(&session, 6);
    let same = session.rebuild(&d).unwrap();
    assert!(Arc::ptr_eq(&first.shape, &same.shape));
    assert_eq!(
        (same.stats.reused_nodes, same.stats.evaluated_nodes),
        (6, 0)
    );
    let mut display = d.clone();
    display.display_chord_tolerance = 0.01;
    let count = |s: &PrismWorkflowShape, d: &PrismWorkflowDocument| -> Result<usize> {
        s.components().iter().try_fold(0, |n, b| {
            Ok(n + b
                .tessellate(
                    d.display_chord_tolerance,
                    Tolerance::new(d.tolerance.linear)?,
                )?
                .triangles
                .len())
        })
    };
    let (_, coarse) = session.rebuild_with(&d, count).unwrap();
    let (fine, fine_count) = session.rebuild_with(&display, count).unwrap();
    assert_eq!(fine.stats.evaluated_nodes, 0);
    assert!(fine_count > coarse);
    for (i, node) in nodes.iter().enumerate() {
        assert!(Arc::ptr_eq(session.prefix_snapshot(i).unwrap(), node));
    }
    let mut output = display.clone();
    output.output = "stock".into();
    let report = session.rebuild(&output).unwrap();
    assert_eq!(report.stats.reused_nodes, 6);
    assert!(Arc::ptr_eq(&report.shape, &nodes[0]));
    close(volume(&report.shape), 1520. + 20. * PI);
    let mut edited = d.clone();
    if let PrismWorkflowOperation::ArcLineExtrusion { holes, .. } = &mut edited.operations[1] {
        for segment in &mut holes[0] {
            if let WorkflowProfileSegment::Arc { radius, .. } = segment {
                *radius = 1.5;
            }
        }
    }
    let rebuilt = session.rebuild(&edited).unwrap();
    assert_eq!(
        (rebuilt.stats.reused_nodes, rebuilt.stats.evaluated_nodes),
        (1, 5)
    );
    assert!(Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &nodes[0]));
    assert!(!Arc::ptr_eq(session.prefix_snapshot(4).unwrap(), &nodes[4]));
    close(volume(&rebuilt.shape), 10. * PI);
    assert_eq!(
        format!("{:?}", rebuilt.shape.as_ref()),
        format!("{:?}", edited.rebuild().unwrap())
    );
    if let PrismWorkflowOperation::RoundedBox { corner_radius, .. } = &mut edited.operations[0] {
        *corner_radius = 2.5;
    }
    assert_eq!(session.rebuild(&edited).unwrap().stats.evaluated_nodes, 6);
}
#[test]
fn failed_candidates_and_history_callbacks_preserve_documents_nodes_and_history() {
    let d = document(1.);
    let mut session = PrismWorkflowSession::new();
    session.rebuild(&d).unwrap();
    let nodes = snapshots(&session, 6);
    let history = (session.undo_count(), session.redo_count());
    let mut candidate = d.clone();
    candidate.output = "stock".into();
    let error = session
        .rebuild_with::<(), _>(&candidate, |_, _| {
            Err(Error::Unsupported("review display callback failed"))
        })
        .unwrap_err();
    assert_eq!(error.code, "geometry_rejected");
    assert_stable(&session, &d, &nodes, history);
    let mut cases = Vec::new();
    let mut out = d.clone();
    out.output = "missing".into();
    cases.push(out);
    let mut forward = d.clone();
    if let PrismWorkflowOperation::Boolean { left, .. } = &mut forward.operations[2] {
        *left = "repeat".into();
    }
    cases.push(forward);
    let mut select = d.clone();
    if let PrismWorkflowOperation::Select { component, .. } = &mut select.operations[3] {
        *component = 2;
    }
    cases.push(select);
    let mut multi = d.clone();
    multi.operations.push(PrismWorkflowOperation::Boolean {
        id: "bad_parent".into(),
        left: "cut".into(),
        right: "annulus".into(),
        operation: PrismWorkflowBooleanKind::Difference,
        axis: [0., 0., 1.],
    });
    multi.output = "bad_parent".into();
    let error = multi.rebuild().unwrap_err();
    assert_eq!(error.code, "component_selection_required");
    assert_eq!(error.operation_id.as_deref(), Some("bad_parent"));
    cases.push(multi);
    let mut display = d.clone();
    display.display_chord_tolerance = 1e-16;
    cases.push(display);
    for bad in cases {
        assert!(session.rebuild(&bad).is_err());
        assert_stable(&session, &d, &nodes, history);
    }
    let mut edited = d.clone();
    edited.display_chord_tolerance = 0.02;
    session.rebuild(&edited).unwrap();
    let mut output = edited.clone();
    output.output = "stock".into();
    session.rebuild(&output).unwrap();
    let nodes = snapshots(&session, 6);
    let history = (session.undo_count(), session.redo_count());
    assert!(session
        .undo_with::<(), _>(|_, _| Err(Error::Unsupported("undo display failed")))
        .is_err());
    assert_stable(&session, &output, &nodes, history);
    let undone = session.undo().unwrap();
    assert_eq!(session.accepted_document(), Some(&edited));
    assert!(Arc::ptr_eq(&undone.shape, &nodes[5]));
    let history = (session.undo_count(), session.redo_count());
    assert!(session
        .redo_with::<(), _>(|_, _| Err(Error::Unsupported("redo export failed")))
        .is_err());
    assert_stable(&session, &edited, &nodes, history);
    let redone = session.redo().unwrap();
    assert!(Arc::ptr_eq(&redone.shape, &nodes[0]));
    assert_eq!(session.accepted_document(), Some(&output));
}
#[test]
fn schema_resources_empty_parents_and_unsupported_stock_are_explicit() {
    let d = document(1.);
    let value = serde_json::to_value(&d).unwrap();
    for field in ["unexpected", "model"] {
        let mut bad = value.clone();
        bad[field] = json!(1);
        assert!(serde_json::from_value::<PrismWorkflowDocument>(bad).is_err());
    }
    let mut unknown = value.clone();
    unknown["operations"][0]["radius"] = json!(2.);
    assert!(serde_json::from_value::<PrismWorkflowDocument>(unknown).is_err());
    for (field, v) in [
        ("schema_version", json!(2)),
        ("document_type", json!("wrong")),
        ("units", json!("m")),
        ("display_chord_tolerance", json!(0.)),
    ] {
        let mut bad = value.clone();
        bad[field] = v;
        assert!(typed(bad).rebuild().is_err());
    }
    {
        let mut bad = d.clone();
        // A root-only invalid radius reaches actual geometry admission.
        bad.operations.truncate(1);
        bad.output = "stock".into();
        if let PrismWorkflowOperation::RoundedBox { corner_radius, .. } = &mut bad.operations[0] {
            *corner_radius = 0.;
        }
        let error = bad.rebuild().unwrap_err();
        assert_eq!(error.operation_id.as_deref(), Some("stock"));
    }
    let mut height = d.clone();
    if let PrismWorkflowOperation::RoundedBox { size, .. } = &mut height.operations[0] {
        size[2] = 0.;
    }
    assert!(height.rebuild().is_err());
    let mut tolerance = d.clone();
    tolerance.tolerance.linear = f64::NAN;
    assert_eq!(tolerance.rebuild().unwrap_err().code, "invalid_tolerance");
    let mut profile = d.clone();
    if let PrismWorkflowOperation::ArcLineExtrusion { outer, .. } = &mut profile.operations[1] {
        outer.clear();
    }
    assert!(profile.rebuild().is_err());
    let mut unsupported_step = d.clone();
    unsupported_step.operations = vec![PrismWorkflowOperation::StepStock {
        id: "oversized".into(),
        step: " ".repeat(1_048_577),
        axis: [0., 0., 1.],
    }];
    unsupported_step.output = "oversized".into();
    assert!(unsupported_step.rebuild().is_err());
    let mut duplicated = d.clone();
    if let PrismWorkflowOperation::ArcLineExtrusion { id, .. } = &mut duplicated.operations[1] {
        *id = "stock".into();
    }
    assert_eq!(
        duplicated.rebuild().unwrap_err().code,
        "invalid_operation_id"
    );
    let mut max = d.clone();
    for i in 0..10 {
        max.operations.push(PrismWorkflowOperation::Select {
            id: format!("view{i}"),
            input: "stock".into(),
            component: 0,
        });
    }
    assert!(max.rebuild().is_ok());
    max.operations.push(PrismWorkflowOperation::Select {
        id: "seventeenth".into(),
        input: "stock".into(),
        component: 0,
    });
    assert_eq!(max.rebuild().unwrap_err().code, "operation_limit");
    let mut empty = d.clone();
    empty
        .operations
        .push(PrismWorkflowOperation::ArcLineExtrusion {
            id: "far".into(),
            outer: serde_json::from_value(json!(circle([30., 0.], 1., 0.13))).unwrap(),
            holes: vec![],
            height: 5.,
            placement: PrismWorkflowPlacement::default(),
        });
    empty.operations.push(PrismWorkflowOperation::Boolean {
        id: "empty".into(),
        left: "stock".into(),
        right: "far".into(),
        operation: PrismWorkflowBooleanKind::Intersection,
        axis: [0., 0., 1.],
    });
    empty.output = "empty".into();
    let mut session = PrismWorkflowSession::new();
    assert!(session
        .rebuild(&empty)
        .unwrap()
        .shape
        .components()
        .is_empty());
    let nodes = snapshots(&session, 8);
    let history = (session.undo_count(), session.redo_count());
    let mut bad = empty.clone();
    bad.operations.push(PrismWorkflowOperation::Boolean {
        id: "from_empty".into(),
        left: "empty".into(),
        right: "annulus".into(),
        operation: PrismWorkflowBooleanKind::Union,
        axis: [0., 0., 1.],
    });
    bad.output = "from_empty".into();
    assert_eq!(
        session.rebuild(&bad).unwrap_err().code,
        "component_selection_required"
    );
    assert_stable(&session, &empty, &nodes, history);
    let mut invalid = d.clone();
    if let PrismWorkflowOperation::Boolean { axis, .. } = &mut invalid.operations[2] {
        *axis = [0., 0., 0.];
    }
    assert!(invalid.rebuild().is_err());
    let stock = d.rebuild().unwrap();
    let blind = blind_bore_normal_prism(
        &stock.components()[0],
        Point3::new(-0.75, 0., 5.),
        0.2,
        1.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        d.geometry_tol().unwrap(),
    )
    .unwrap();
    let mut imported = d.clone();
    imported.operations = vec![PrismWorkflowOperation::StepStock {
        id: "blind".into(),
        step: export_step_bounded_analytic_mm(blind.kept(), 1e-6).unwrap(),
        axis: [0., 0., 1.],
    }];
    imported.output = "blind".into();
    assert!(imported.rebuild().is_err());
}

#[test]
fn full_relative_policy_applies_to_native_and_actual_step_stock_before_acceptance() {
    let mut absolute = document(1.);
    absolute.operations.truncate(1);
    absolute.output = "stock".into();
    if let PrismWorkflowOperation::RoundedBox { size, .. } = &mut absolute.operations[0] {
        size[2] = 0.1;
    }
    let mut session = PrismWorkflowSession::new();
    let accepted = session.rebuild(&absolute).unwrap();
    let nodes = snapshots(&session, 1);
    let history = (session.undo_count(), session.redo_count());
    let mut unresolved = absolute.clone();
    unresolved.tolerance.relative = 0.001;
    let error = session.rebuild(&unresolved).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("stock"));
    assert_stable(&session, &absolute, &nodes, history);
    let step = export_step_bounded_analytic_mm(&accepted.shape.components()[0], 1e-6).unwrap();
    let mut imported = absolute.clone();
    imported.operations = vec![PrismWorkflowOperation::StepStock {
        id: "imported".into(),
        step,
        axis: [0., 0., 1.],
    }];
    imported.output = "imported".into();
    assert!(imported.rebuild().is_ok());
    imported.tolerance.relative = 0.001;
    let error = session.rebuild(&imported).unwrap_err();
    assert_eq!(error.operation_id.as_deref(), Some("imported"));
    assert_stable(&session, &absolute, &nodes, history);
}
