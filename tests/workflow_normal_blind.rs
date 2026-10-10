use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, PI};

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

fn check_retained_curves(source: &Solid, kept: &Solid, guard: f64) {
    for e in &source.edges {
        let range = e.curve.range();
        assert!(
            kept.edges.iter().any(|q| {
                let qr = q.curve.range();
                [false, true].into_iter().any(|reverse| {
                    (0..=16).all(|i| {
                        let t = i as f64 / 16.;
                        let u = if reverse { 1. - t } else { t };
                        (e.curve
                            .try_evaluate(range[0] + (range[1] - range[0]) * t)
                            .unwrap()
                            - q.curve.try_evaluate(qr[0] + (qr[1] - qr[0]) * u).unwrap())
                        .norm()
                            <= guard
                    })
                })
            }),
            "retained original line/arc geometry"
        );
    }
}

use serde_json::{json, Value};
use std::sync::Arc;
fn document(scale: f64) -> WorkflowDocument {
    serde_json::from_value(json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6*scale,"angular":1e-10,"relative":0.},"operations":[
        {"kind":"rounded_box","id":"root","size":[20.*scale,16.*scale,5.*scale],"corner_radius":2.*scale},
        {"kind":"bore","id":"through","input":"root","mode":"through","center":[6.*scale,0.],"radius":0.6*scale},
        {"kind":"bore","id":"top","input":"through","mode":"blind","entry":"top","center":[-4.*scale,-2.*scale],"radius":scale,"depth":2.*scale},
        {"kind":"bore","id":"bottom","input":"top","mode":"blind","entry":"bottom","center":[0.,3.*scale],"radius":0.5*scale,"depth":1.5*scale}
    ]})).unwrap()
}
fn prefix(d: &WorkflowDocument, n: usize) -> WorkflowDocument {
    let mut d = d.clone();
    d.operations.truncate(n);
    d
}
fn signature(s: &Solid) -> String {
    format!("{s:?}")
}
#[test]
fn real_curved_blind_history_retains_prefix_and_matches_direct_append_and_step() {
    for scale in [1e-4, 1., 10.] {
        let d = document(scale);
        let source = prefix(&d, 2).rebuild().unwrap();
        let first = prefix(&d, 3).rebuild().unwrap();
        let final_body = d.rebuild().unwrap();
        let g = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
        let direct = blind_bore_normal_prism(
            &source,
            Point3::new(-4. * scale, -2. * scale, 2.5 * scale),
            scale,
            2. * scale,
            Vec3::new(0., 0., 1.),
            NormalPrismBoreEntry::Positive,
            g,
        )
        .unwrap();
        assert_eq!(signature(&first), signature(direct.kept()));
        let next = append_blind_bores_normal_prism(
            &first,
            &[NormalPrismBlindBoreSpec {
                center: Point3::new(0., 3. * scale, -2.5 * scale),
                radius: 0.5 * scale,
                depth: 1.5 * scale,
                entry: NormalPrismBoreEntry::Negative,
            }],
            Vec3::new(0., 0., 1.),
            g,
        )
        .unwrap();
        assert_eq!(signature(&final_body), signature(next.kept()));
        assert_eq!(
            signature_vertices(&first),
            format!("{:?}", &final_body.vertices[..first.vertices.len()])
        );
        assert_eq!(
            format!("{:?}", first.edges),
            format!("{:?}", &final_body.edges[..first.edges.len()])
        );
        check_retained_curves(&source, &final_body, 8192. * f64::EPSILON * 20. * scale);
        let expected = (1520. + 20. * PI - 1.8 * PI - 2. * PI - 0.375 * PI) * scale.powi(3);
        assert!((final_body.volume().unwrap() - expected).abs() < 1e-10 * expected.abs());
        assert_eq!(euler(&final_body), 0);
        assert_eq!(final_body.vertices.len(), first.vertices.len() + 8);
        assert_eq!(final_body.edges.len(), first.edges.len() + 12);
        assert_eq!(final_body.shell.faces.len(), first.shell.faces.len() + 5);
        check_closed(&final_body, scale, 20. * scale);
        let step = export_step_bounded_analytic_mm(&final_body, 1e-6 * scale).unwrap();
        let imported =
            import_step_bounded_analytic_mm(&step, Tolerance::new(1e-6 * scale).unwrap()).unwrap();
        assert!((imported.volume().unwrap() - expected).abs() < 1e-10 * expected.abs());
        check_closed(&imported, scale, 20. * scale);
    }
}
fn signature_vertices(s: &Solid) -> String {
    format!("{:?}", s.vertices)
}
#[test]
fn blind_cache_suffix_edits_and_failures_preserve_actual_accepted_body() {
    let d = document(1.);
    let mut session = WorkflowSession::new();
    let root = session.rebuild(&prefix(&d, 1)).unwrap();
    let through = session.rebuild(&prefix(&d, 2)).unwrap();
    let top = session.rebuild(&prefix(&d, 3)).unwrap();
    assert_eq!(top.stats.reused_operations, 2);
    let full = session.rebuild(&d).unwrap();
    assert_eq!(full.stats.reused_operations, 3);
    let mut edited = d.clone();
    if let WorkflowOperation::Bore { depth, .. } = &mut edited.operations[3] {
        *depth = Some(1.);
    }
    let changed = session.rebuild(&edited).unwrap();
    assert_eq!(changed.stats.reused_operations, 3);
    assert_eq!(changed.stats.rebuilt_operation_ids, vec!["bottom"]);
    assert_eq!(
        signature(&changed.solid),
        signature(&edited.rebuild().unwrap())
    );
    assert!(Arc::ptr_eq(
        &session.rebuild(&prefix(&d, 3)).unwrap().solid,
        &top.solid
    ));
    assert!(Arc::ptr_eq(
        &session.rebuild(&prefix(&d, 2)).unwrap().solid,
        &through.solid
    ));
    assert!(Arc::ptr_eq(
        &session.rebuild(&prefix(&d, 1)).unwrap().solid,
        &root.solid
    ));
    let accepted = session.rebuild(&d).unwrap();
    let original = signature(&accepted.solid);
    let step = export_step_bounded_analytic_mm(&accepted.solid, 1e-6).unwrap();
    let mut bads = Vec::new();
    let mut overlap = d.clone();
    if let WorkflowOperation::Bore { center, .. } = &mut overlap.operations[3] {
        *center = [-4., -2.];
    }
    bads.push(overlap);
    let mut through_after = d.clone();
    if let WorkflowOperation::Bore { mode, depth, .. } = &mut through_after.operations[3] {
        *mode = WorkflowBoreMode::Through;
        *depth = None;
    }
    bads.push(through_after);
    for depth in [0., 5., 5. - 1e-7] {
        let mut bad = d.clone();
        if let WorkflowOperation::Bore { depth: v, .. } = &mut bad.operations[3] {
            *v = Some(depth);
        }
        bads.push(bad);
    }
    for bad in bads {
        let report: Value = serde_json::from_str(
            &session
                .evaluate_json(&serde_json::to_string(&bad).unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(report["ok"], false);
        let stable = session.rebuild(&d).unwrap();
        assert!(Arc::ptr_eq(&stable.solid, &accepted.solid));
        assert_eq!(signature(&stable.solid), original);
        assert_eq!(
            export_step_bounded_analytic_mm(&stable.solid, 1e-6).unwrap(),
            step
        );
    }
    let mut stock = d.clone();
    if let WorkflowOperation::RoundedBox { corner_radius, .. } = &mut stock.operations[0] {
        *corner_radius = 2.5;
    }
    assert_eq!(session.rebuild(&stock).unwrap().stats.reused_operations, 0);
}

fn line(start: [f64; 2], end: [f64; 2]) -> Value {
    json!({"kind":"line","start":start,"end":end})
}
fn arc(center: [f64; 2], radius: f64, start: f64) -> Value {
    json!({"kind":"arc","center":center,"radius":radius,"start_angle":start,"sweep":FRAC_PI_2})
}
fn outer() -> Vec<Value> {
    vec![
        line([15., 15.], [35., 15.]),
        arc([35., 20.], 5., -FRAC_PI_2),
        arc([35., 20.], 5., 0.),
        line([35., 25.], [15., 25.]),
        arc([15., 20.], 5., FRAC_PI_2),
        arc([15., 20.], 5., PI),
    ]
}
fn holes() -> Vec<Vec<Value>> {
    vec![
        vec![
            line([18., 19.], [20., 19.]),
            line([20., 19.], [20., 21.]),
            line([20., 21.], [18., 21.]),
            line([18., 21.], [18., 19.]),
        ],
        (0..4)
            .map(|i| arc([30., 20.], 1.5, i as f64 * FRAC_PI_2))
            .collect(),
    ]
}
fn capsule_document(with_holes: bool) -> Value {
    json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[{"kind":"arc_line_extrusion","id":"capsule","outer":outer(),"holes":if with_holes{holes()}else{vec![]},"height":8.},{"kind":"bore","id":"new-hole","input":"capsule","mode":"through","center":[25.,20.],"radius":1.}]})
}

#[test]
fn noncentered_arc_profile_initial_openings_and_mixed_blinds_use_actual_body() {
    let mut value = capsule_document(true);
    value["operations"].as_array_mut().unwrap().extend([
        json!({"kind":"bore","id":"top","input":"new-hole","mode":"blind","entry":"top","center":[15.,20.],"radius":1.,"depth":2.}),
        json!({"kind":"bore","id":"bottom","input":"top","mode":"blind","entry":"bottom","center":[35.,20.],"radius":0.75,"depth":3.})]);
    let d: WorkflowDocument = serde_json::from_value(value.clone()).unwrap();
    let source = prefix(&d, 2).rebuild().unwrap();
    let body = d.rebuild().unwrap();
    let expected =
        (200. + 25. * PI - 4. - 2.25 * PI - PI) * 8. - 2. * PI - 0.75_f64.powi(2) * 3. * PI;
    assert!((body.volume().unwrap() - expected).abs() < 1e-9);
    assert_eq!(euler(&body), -4);
    check_closed(&body, 1., 40.);
    check_retained_curves(&source, &body, 8192. * f64::EPSILON * 40.);
    let exported: Value =
        serde_json::from_str(&export_workflow_step_mm_json(&value.to_string()).unwrap()).unwrap();
    let restored = import_step_bounded_analytic_mm(
        exported["step"].as_str().unwrap(),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    assert!((restored.volume().unwrap() - expected).abs() < 1e-9);
    check_closed(&restored, 1., 40.);
}
#[test]
fn workflow_blind_scope_diagnostics_and_total_bore_budget_are_explicit() {
    let d = document(1.);
    let mut after = d.clone();
    if let WorkflowOperation::Bore {
        mode, depth, entry, ..
    } = &mut after.operations[3]
    {
        *mode = WorkflowBoreMode::Through;
        *entry = WorkflowBoreEntry::Top;
        *depth = None;
    }
    let error = after.rebuild().unwrap_err();
    assert_eq!(error.code, "curved_through_after_blind_unsupported");
    assert_eq!(error.operation_id.as_deref(), Some("bottom"));
    let mut overlap = d.clone();
    if let WorkflowOperation::Bore { center, .. } = &mut overlap.operations[3] {
        *center = [-4., -2.];
    }
    let error = overlap.rebuild().unwrap_err();
    assert_eq!(error.code, "bore_clearance");
    assert_eq!(error.operation_id.as_deref(), Some("bottom"));
    let mut many = prefix(&d, 1);
    for i in 0..16 {
        many.operations.push(WorkflowOperation::Bore {
            id: format!("p{i}"),
            input: if i == 0 {
                "root".into()
            } else {
                format!("p{}", i - 1)
            },
            mode: WorkflowBoreMode::Blind,
            entry: if i % 2 == 0 {
                WorkflowBoreEntry::Top
            } else {
                WorkflowBoreEntry::Bottom
            },
            center: [-6. + 4. * (i % 4) as f64, -4.5 + 3. * (i / 4) as f64],
            radius: 0.4,
            depth: Some(1.),
        });
    }
    let accepted = many.rebuild().unwrap();
    assert_eq!(euler(&accepted), 2);
    assert!((accepted.volume().unwrap() - (1520. + 20. * PI - 16. * 0.16 * PI)).abs() < 1e-9);
    many.operations.push(WorkflowOperation::Bore {
        id: "seventeenth".into(),
        input: "p15".into(),
        mode: WorkflowBoreMode::Blind,
        entry: WorkflowBoreEntry::Top,
        center: [0., 0.],
        radius: 0.1,
        depth: Some(1.),
    });
    assert!(many.rebuild().is_err());
}
