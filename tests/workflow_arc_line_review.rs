use hagane::*;
use serde_json::{json, Value};
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

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
fn document(with_holes: bool) -> Value {
    json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[{"kind":"arc_line_extrusion","id":"capsule","outer":outer(),"holes":if with_holes{holes()}else{vec![]},"height":8.},{"kind":"bore","id":"new-hole","input":"capsule","mode":"through","center":[25.,20.],"radius":1.}]})
}
fn typed(value: &Value) -> WorkflowDocument {
    serde_json::from_value(value.clone()).unwrap()
}
fn segment(value: &Value) -> PlanarSegment {
    if value["kind"] == "line" {
        PlanarSegment::Line {
            a: serde_json::from_value(value["start"].clone()).unwrap(),
            b: serde_json::from_value(value["end"].clone()).unwrap(),
        }
    } else {
        PlanarSegment::Arc {
            center: serde_json::from_value(value["center"].clone()).unwrap(),
            radius: value["radius"].as_f64().unwrap(),
            start_angle: value["start_angle"].as_f64().unwrap(),
            sweep: value["sweep"].as_f64().unwrap(),
        }
    }
}
fn direct_stock(with_holes: bool) -> Solid {
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., -4.),
            outer: outer().iter().map(segment).collect(),
            holes: if with_holes {
                holes()
                    .iter()
                    .map(|h| h.iter().map(segment).collect())
                    .collect()
            } else {
                vec![]
            },
        },
        8.,
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap()
}
fn signature(s: &Solid) -> String {
    format!("{s:?}")
}
fn check(s: &Solid, hole_count: usize, removed_area: f64) {
    s.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    let expected = (200. + 25. * PI - removed_area) * 8.;
    assert!((s.volume().unwrap() - expected).abs() < 1e-8);
    assert_eq!(
        s.vertices.len() as isize - s.edges.len() as isize
            + s.shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>(),
        2 - 2 * hole_count as isize
    );
    assert!((s.bounds().min - Point3::new(10., 15., -4.)).norm() < 1e-10);
    assert!((s.bounds().max - Point3::new(40., 25., 4.)).norm() < 1e-10);
    let mut uses = vec![Vec::new(); s.edges.len()];
    for f in &s.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push(c.forward == (f.orientation == 1));
            let curve = &s.edges[c.edge].curve;
            let range = curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - curve.try_evaluate(t).unwrap())
                    .norm()
                        < 1e-10
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0] != u[1]));
    let text = export_step_bounded_analytic_mm(s, 1e-6).unwrap();
    let imported = import_step_bounded_analytic_mm(&text, Tolerance::new(1e-6).unwrap()).unwrap();
    assert!((s.volume().unwrap() - imported.volume().unwrap()).abs() < 1e-8);
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
fn noncentered_capsule_initial_openings_and_bores_replay_actual_region_and_cache() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for with_holes in [false, true] {
        let value = document(with_holes);
        let full = typed(&value);
        let mut root_value = value.clone();
        root_value["operations"].as_array_mut().unwrap().truncate(1);
        let root_doc = typed(&root_value);
        let mut session = WorkflowSession::new();
        let stock = session.rebuild(&root_doc).unwrap();
        let initial_area = if with_holes { 4. + PI * 2.25 } else { 0. };
        let initial_holes = if with_holes { 2 } else { 0 };
        check(&stock.solid, initial_holes, initial_area);
        assert_eq!(
            signature(&stock.solid),
            signature(&direct_stock(with_holes))
        );
        let accepted = session.rebuild(&full).unwrap();
        assert_eq!(
            (
                accepted.stats.reused_operations,
                accepted.stats.rebuilt_operations
            ),
            (1, 1)
        );
        check(&accepted.solid, initial_holes + 1, initial_area + PI);
        assert_eq!(
            signature(&accepted.solid),
            signature(&full.rebuild().unwrap())
        );
        let direct = bore_normal_arc_line_prism(
            &direct_stock(with_holes),
            Point3::new(25., 20., 0.),
            1.,
            tol,
        )
        .unwrap();
        assert_eq!(signature(&accepted.solid), signature(direct.kept()));
        let repeat = session.rebuild(&full).unwrap();
        assert!(Arc::ptr_eq(&accepted.solid, &repeat.solid));
        let mut late = value.clone();
        late["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":"late","input":"new-hole","mode":"through","center":[15.,20.],"radius":1.}));
        let late_doc = typed(&late);
        let appended = session.rebuild(&late_doc).unwrap();
        assert_eq!(
            (
                appended.stats.reused_operations,
                appended.stats.rebuilt_operations
            ),
            (2, 1)
        );
        check(&appended.solid, initial_holes + 2, initial_area + 2. * PI);
        assert_eq!(
            signature(&appended.solid),
            signature(&late_doc.rebuild().unwrap())
        );
        late["operations"][2]["radius"] = json!(0.75);
        let edited = typed(&late);
        let changed = session.rebuild(&edited).unwrap();
        assert_eq!(
            (
                changed.stats.reused_operations,
                changed.stats.rebuilt_operations
            ),
            (2, 1)
        );
        assert_eq!(
            signature(&changed.solid),
            signature(&edited.rebuild().unwrap())
        );
        let shortened = session.rebuild(&full).unwrap();
        assert!(Arc::ptr_eq(&accepted.solid, &shortened.solid));
        let root = session.rebuild(&root_doc).unwrap();
        assert!(Arc::ptr_eq(&stock.solid, &root.solid));
        assert_eq!(
            classify_point_in_solid(&accepted.solid, Point3::new(25., 20., 0.), tol).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            classify_point_in_solid(&accepted.solid, Point3::new(25., 23., 0.), tol).unwrap(),
            PointLocation::Inside
        );
        let mut reversed = value.clone();
        let reverse_ring = |ring: &[Value]| {
            ring.iter()
                .rev()
                .map(|s| {
                    if s["kind"] == "line" {
                        line(
                            serde_json::from_value(s["end"].clone()).unwrap(),
                            serde_json::from_value(s["start"].clone()).unwrap(),
                        )
                    } else {
                        let mut a = s.clone();
                        a["start_angle"] = json!(
                            s["start_angle"].as_f64().unwrap() + s["sweep"].as_f64().unwrap()
                        );
                        a["sweep"] = json!(-s["sweep"].as_f64().unwrap());
                        a
                    }
                })
                .collect::<Vec<_>>()
        };
        reversed["operations"][0]["outer"] = json!(reverse_ring(&outer()));
        reversed["operations"][0]["holes"] = json!(if with_holes {
            holes().iter().map(|h| reverse_ring(h)).collect::<Vec<_>>()
        } else {
            vec![]
        });
        check(
            &typed(&reversed).rebuild().unwrap(),
            initial_holes + 1,
            initial_area + PI,
        );
    }
}

#[test]
fn invalid_quarter_loops_hole_contacts_and_blind_edits_preserve_accepted_arc() {
    let value = document(true);
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    let original = signature(&accepted.solid);
    let mut bad = Vec::new();
    for sweep in [0., PI, FRAC_PI_2 + 1e-6] {
        let mut v = value.clone();
        v["operations"][0]["outer"][1]["sweep"] = json!(sweep);
        bad.push(v);
    }
    let mut disconnected = value.clone();
    disconnected["operations"][0]["outer"][0]["end"] = json!([34., 15.]);
    bad.push(disconnected);
    for shift in [11., 50.] {
        let mut invalid = value.clone();
        for segment in invalid["operations"][0]["holes"][0].as_array_mut().unwrap() {
            for field in ["start", "end"] {
                let x = segment[field][0].as_f64().unwrap();
                segment[field][0] = json!(x + shift);
            }
        }
        bad.push(invalid);
    }
    let mut lines = value.clone();
    lines["operations"][0]["outer"] = json!([
        line([10., 15.], [40., 15.]),
        line([40., 15.], [40., 25.]),
        line([40., 25.], [10., 25.]),
        line([10., 25.], [10., 15.])
    ]);
    lines["operations"][0]["holes"] = json!([]);
    bad.push(lines);
    for (center, radius) in [
        ([19., 20.], 0.5),
        ([30., 20.], 0.5),
        ([25., 24.], 1.),
        ([27.5, 20.], 1.),
    ] {
        let mut v = value.clone();
        v["operations"][1]["center"] = json!(center);
        v["operations"][1]["radius"] = json!(radius);
        bad.push(v);
    }
    let mut blind = value.clone();
    blind["operations"][1]["mode"] = json!("blind");
    blind["operations"][1]["depth"] = json!(2.);
    bad.push(blind);
    let mut unknown = value.clone();
    unknown["operations"][0]["outer"][0]["a"] = json!([15., 15.]);
    bad.push(unknown);
    let mut radius = value.clone();
    radius["operations"][0]["outer"][1]["radius"] = json!("5");
    bad.push(radius);
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
            (2, 0)
        );
        assert!(Arc::ptr_eq(&accepted.solid, &retained.solid));
        assert_eq!(signature(&retained.solid), original);
    }
}

#[test]
fn initial_profile_and_subsequent_bores_share_hole_and_segment_budgets() {
    let value = document(true);
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    let mut holes = value.clone();
    let mut previous = "new-hole".to_owned();
    for i in 0..14 {
        let id = format!("extra-{i}");
        holes["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":id,"input":previous,"mode":"through","center":[25.,22.],"radius":0.1}));
        previous = id;
    }
    let failure = session.rebuild(&typed(&holes)).unwrap_err();
    assert_eq!(failure.code, "unsupported_history");
    assert!(Arc::ptr_eq(
        &accepted.solid,
        &session.rebuild(&good).unwrap().solid
    ));
    let mut full = document(false);
    full["operations"].as_array_mut().unwrap().truncate(1);
    full["operations"][0]["outer"]=json!((0..128).map(|i|json!({"kind":"arc","center":[25.,20.],"radius":10.,"start_angle":2.*PI*i as f64/128.,"sweep":2.*PI/128.})).collect::<Vec<_>>());
    let full_doc = typed(&full);
    let root = session.rebuild(&full_doc).unwrap();
    assert!((root.solid.volume().unwrap() - PI * 100. * 8.).abs() < 1e-8);
    full["operations"].as_array_mut().unwrap().push(json!({"kind":"bore","id":"over-segments","input":"capsule","mode":"through","center":[25.,20.],"radius":1.}));
    let failure = session.rebuild(&typed(&full)).unwrap_err();
    assert_eq!(failure.code, "unsupported_history");
    assert!(Arc::ptr_eq(
        &root.solid,
        &session.rebuild(&full_doc).unwrap().solid
    ));
}
