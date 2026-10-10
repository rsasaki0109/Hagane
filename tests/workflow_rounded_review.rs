use hagane::*;
use serde_json::{json, Value};
use std::f64::consts::PI;
use std::sync::Arc;

fn document() -> Value {
    json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[{"kind":"rounded_box","id":"rounded","size":[80.,60.,20.],"corner_radius":3.},{"kind":"bore","id":"first","input":"rounded","mode":"through","center":[-10.,0.],"radius":2.},{"kind":"bore","id":"second","input":"first","mode":"through","center":[10.,5.],"radius":3.}]})
}
fn typed(value: &Value) -> WorkflowDocument {
    serde_json::from_value(value.clone()).unwrap()
}
fn prefix(value: &Value, count: usize) -> WorkflowDocument {
    let mut v = value.clone();
    v["operations"].as_array_mut().unwrap().truncate(count);
    typed(&v)
}
fn signature(s: &Solid) -> String {
    format!("{s:?}")
}
fn expected(radius: f64, bores: &[f64]) -> f64 {
    96000.
        - 4. * radius * radius * (1. - PI / 4.) * 20.
        - PI * 20. * bores.iter().map(|r| r * r).sum::<f64>()
}
fn check(s: &Solid, radius: f64, bores: &[f64]) {
    s.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((s.volume().unwrap() - expected(radius, bores)).abs() < 1e-7);
    let euler = s.vertices.len() as isize - s.edges.len() as isize
        + s.shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>();
    assert_eq!(euler, 2 - 2 * bores.len() as isize);
    assert_eq!(
        s.shell
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::FramedCylinder { .. }))
            .count(),
        4 + 4 * bores.len()
    );
    let mut uses = vec![Vec::new(); s.edges.len()];
    for face in &s.shell.faces {
        for c in face.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push(c.forward == (face.orientation == 1));
            let edge = &s.edges[c.edge];
            let range = edge.curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - edge.curve.try_evaluate(t).unwrap())
                    .norm()
                        < 1e-9
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0] != u[1]));
    let step = export_step_bounded_analytic_mm(s, 1e-6).unwrap();
    let imported = import_step_bounded_analytic_mm(&step, Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((imported.volume().unwrap() - s.volume().unwrap()).abs() < 1e-7);
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (
            imported.vertices.len(),
            imported.edges.len(),
            imported.shell.faces.len()
        )
    );
}

#[test]
fn rounded_replay_is_actual_fillet_and_bore_geometry_with_real_prefix_reuse() {
    let value = document();
    let full = typed(&value);
    let stock_doc = prefix(&value, 1);
    let first_doc = prefix(&value, 2);
    let mut session = WorkflowSession::new();
    let stock = session.rebuild(&stock_doc).unwrap();
    check(&stock.solid, 3., &[]);
    let stock_signature = signature(&stock.solid);
    let first = session.rebuild(&first_doc).unwrap();
    assert_eq!(
        (
            first.stats.reused_operations,
            first.stats.rebuilt_operations
        ),
        (1, 1)
    );
    check(&first.solid, 3., &[2.]);
    let accepted = session.rebuild(&full).unwrap();
    assert_eq!(
        (
            accepted.stats.reused_operations,
            accepted.stats.rebuilt_operations
        ),
        (2, 1)
    );
    check(&accepted.solid, 3., &[2., 3.]);
    assert_eq!(
        signature(&accepted.solid),
        signature(&full.rebuild().unwrap())
    );
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let actual_stock = make_box(
        BoxSpec {
            min: Point3::new(-40., -30., -10.),
            size: Vec3::new(80., 60., 20.),
        },
        tol.absolute(),
    )
    .unwrap();
    let actual_stock =
        fillet_parallel_box_edges(&actual_stock, &[(8, 3.), (9, 3.), (10, 3.), (11, 3.)], tol)
            .unwrap()
            .into_solid();
    assert_eq!(signature(&stock.solid), signature(&actual_stock));
    let actual_first =
        bore_normal_arc_line_prism(&actual_stock, Point3::new(-10., 0., 0.), 2., tol).unwrap();
    let actual_second =
        bore_normal_arc_line_prism(actual_first.kept(), Point3::new(10., 5., 0.), 3., tol).unwrap();
    assert_eq!(signature(&accepted.solid), signature(actual_second.kept()));
    let repeat = session.rebuild(&full).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &repeat.solid));
    assert_eq!(repeat.stats.reused_operations, 3);
    let mut changed = value.clone();
    changed["operations"][2]["radius"] = json!(2.5);
    let changed_doc = typed(&changed);
    let late = session.rebuild(&changed_doc).unwrap();
    assert_eq!(
        (late.stats.reused_operations, late.stats.rebuilt_operations),
        (2, 1)
    );
    assert!(!Arc::ptr_eq(&accepted.solid, &late.solid));
    assert_eq!(
        signature(&late.solid),
        signature(&changed_doc.rebuild().unwrap())
    );
    check(&late.solid, 3., &[2., 2.5]);
    let shortened = session.rebuild(&first_doc).unwrap();
    assert!(Arc::ptr_eq(&first.solid, &shortened.solid));
    assert_eq!(
        (
            shortened.stats.reused_operations,
            shortened.stats.rebuilt_operations
        ),
        (2, 0)
    );
    let root = session.rebuild(&stock_doc).unwrap();
    assert!(Arc::ptr_eq(&stock.solid, &root.solid));
    assert_eq!(signature(&stock.solid), stock_signature);
    changed["operations"][0]["corner_radius"] = json!(4.);
    let changed_root = typed(&changed);
    let rebuilt = session.rebuild(&changed_root).unwrap();
    assert_eq!(
        (
            rebuilt.stats.reused_operations,
            rebuilt.stats.rebuilt_operations
        ),
        (0, 3)
    );
    assert_eq!(
        rebuilt.stats.rebuilt_operation_ids,
        vec!["rounded", "first", "second"]
    );
    assert_eq!(
        signature(&rebuilt.solid),
        signature(&changed_root.rebuild().unwrap())
    );
    check(&rebuilt.solid, 4., &[2., 2.5]);
}

#[test]
fn rejected_rounded_edits_leave_real_accepted_snapshots_and_document_intact() {
    let value = document();
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    let original = signature(&accepted.solid);
    let mut bad = Vec::new();
    for r in [0., -1., 30.] {
        let mut v = value.clone();
        v["operations"][0]["corner_radius"] = json!(r);
        bad.push(v);
    }
    let mut blind = value.clone();
    blind["operations"][2]["mode"] = json!("blind");
    blind["operations"][2]["depth"] = json!(3.);
    bad.push(blind);
    let mut contact = value.clone();
    contact["operations"][2]["center"] = json!([-5., 0.]);
    bad.push(contact);
    let mut corner = value.clone();
    corner["operations"][2]["center"] = json!([39., 29.]);
    bad.push(corner);
    let mut tolerance = value.clone();
    tolerance["tolerance"]["linear"] = json!(1e-14);
    bad.push(tolerance);
    let mut unknown = value.clone();
    unknown["operations"][0]["radius"] = json!(4.);
    bad.push(unknown);
    let mut wrong = value.clone();
    wrong["operations"][0]["corner_radius"] = json!("3");
    bad.push(wrong);
    for candidate in bad {
        let report: Value =
            serde_json::from_str(&session.evaluate_json(&candidate.to_string()).unwrap()).unwrap();
        assert_eq!(report["ok"], false, "{candidate}");
        assert!(report.get("mesh").is_none());
        let retained = session.rebuild(&good).unwrap();
        assert_eq!(
            (
                retained.stats.reused_operations,
                retained.stats.rebuilt_operations
            ),
            (3, 0)
        );
        assert!(Arc::ptr_eq(&accepted.solid, &retained.solid));
        assert_eq!(signature(&accepted.solid), original);
    }
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&value.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["document"], value);
    assert_eq!(report["rebuild"]["reused_operations"], 3);
}

#[test]
fn sixteen_disjoint_bores_replay_and_seventeenth_edit_rolls_back() {
    let mut value = document();
    value["operations"].as_array_mut().unwrap().truncate(1);
    let mut previous = "rounded".to_owned();
    for x in [-30., -10., 10., 30.] {
        for y in [-20., -7., 7., 20.] {
            let id = format!("hole-{}", value["operations"].as_array().unwrap().len());
            value["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":id,"input":previous,"mode":"through","center":[x,y],"radius":1.}));
            previous = id;
        }
    }
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    check(&accepted.solid, 3., &[1.; 16]);
    let mut extra = value.clone();
    extra["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":"seventeenth","input":previous,"mode":"through","center":[0.,0.],"radius":1.}));
    assert!(session.rebuild(&typed(&extra)).is_err());
    let retained = session.rebuild(&good).unwrap();
    assert!(Arc::ptr_eq(&accepted.solid, &retained.solid));
    assert_eq!(
        (
            retained.stats.reused_operations,
            retained.stats.rebuilt_operations
        ),
        (17, 0)
    );
}
