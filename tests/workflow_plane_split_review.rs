use hagane::*;
use serde_json::{json, Value};
use std::f64::consts::PI;
use std::sync::Arc;

fn document(side: &str) -> Value {
    let (center, radius) = if side == "negative" {
        ([0., 0.], 2.)
    } else {
        ([37., 0.], 0.5)
    };
    json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"operations":[{"kind":"rounded_box","id":"stock","size":[80.,60.,20.],"corner_radius":3.},{"kind":"plane_split","id":"cut","input":"stock","offset":34.,"normal_angle":0.,"side":side},{"kind":"bore","id":"hole","input":"cut","mode":"through","center":center,"radius":radius}]})
}
fn typed(v: &Value) -> WorkflowDocument {
    serde_json::from_value(v.clone()).unwrap()
}
fn prefix(v: &Value, n: usize) -> WorkflowDocument {
    let mut d = v.clone();
    d["operations"].as_array_mut().unwrap().truncate(n);
    typed(&d)
}
fn signature(s: &Solid) -> String {
    format!("{s:?}")
}
fn plane(x: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(x, 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    }
}
fn check(s: &Solid, volume: f64, holes: usize) {
    let t = Tolerance::new(1e-6).unwrap();
    s.validate(t).unwrap();
    assert!((s.volume().unwrap() - volume).abs() < 1e-7);
    assert_eq!(
        s.vertices.len() as isize - s.edges.len() as isize
            + s.shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>(),
        2 - 2 * holes as isize
    );
    let mut uses = vec![Vec::new(); s.edges.len()];
    for f in &s.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push(c.forward == (f.orientation == 1));
            let curve = &s.edges[c.edge].curve;
            let range = curve.range();
            for i in 0..=12 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 12.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - curve.try_evaluate(t).unwrap())
                    .norm()
                        < 1e-9
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0] != u[1]));
    let step = export_step_bounded_analytic_mm(s, 1e-6).unwrap();
    let imported = import_step_bounded_analytic_mm(&step, t).unwrap();
    assert!((imported.volume().unwrap() - volume).abs() < 1e-7);
    assert_eq!(
        (
            imported.vertices.len(),
            imported.edges.len(),
            imported.shell.faces.len()
        ),
        (s.vertices.len(), s.edges.len(), s.shell.faces.len())
    );
}

#[test]
fn rounded_children_and_later_bores_are_actual_split_breps_with_real_cached_prefixes() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let total = 96000. - 4. * 9. * (1. - PI / 4.) * 20.;
    let positive = (6. * 60. - 2. * 9. * (1. - PI / 4.)) * 20.;
    for side in ["negative", "positive"] {
        let value = document(side);
        let full = typed(&value);
        let root_doc = prefix(&value, 1);
        let cut_doc = prefix(&value, 2);
        let source = root_doc.rebuild().unwrap();
        let split = split_normal_arc_line_prism_by_plane(&source, &plane(34.), tol).unwrap();
        let selected = if side == "negative" {
            split.negative()
        } else {
            split.positive()
        };
        let selected_volume = if side == "negative" {
            total - positive
        } else {
            positive
        };
        let (radius, center) = if side == "negative" {
            (2., Point3::new(0., 0., 0.))
        } else {
            (0.5, Point3::new(37., 0., 0.))
        };
        let mut session = WorkflowSession::new();
        let root = session.rebuild(&root_doc).unwrap();
        let cut = session.rebuild(&cut_doc).unwrap();
        assert_eq!(
            (cut.stats.reused_operations, cut.stats.rebuilt_operations),
            (1, 1)
        );
        assert_eq!(signature(&cut.solid), signature(selected));
        check(&cut.solid, selected_volume, 0);
        let accepted = session.rebuild(&full).unwrap();
        assert_eq!(
            (
                accepted.stats.reused_operations,
                accepted.stats.rebuilt_operations
            ),
            (2, 1)
        );
        let direct = bore_normal_arc_line_prism(selected, center, radius, tol).unwrap();
        assert_eq!(signature(&accepted.solid), signature(direct.kept()));
        assert_eq!(
            signature(&accepted.solid),
            signature(&full.rebuild().unwrap())
        );
        check(
            &accepted.solid,
            selected_volume - PI * radius * radius * 20.,
            1,
        );
        let mut changed = value.clone();
        changed["operations"][2]["radius"] = json!(radius * 0.8);
        let changed_doc = typed(&changed);
        let late = session.rebuild(&changed_doc).unwrap();
        assert_eq!(
            (late.stats.reused_operations, late.stats.rebuilt_operations),
            (2, 1)
        );
        assert_eq!(
            signature(&late.solid),
            signature(&changed_doc.rebuild().unwrap())
        );
        let restored = session.rebuild(&cut_doc).unwrap();
        assert!(Arc::ptr_eq(&cut.solid, &restored.solid));
        assert_eq!(
            (
                restored.stats.reused_operations,
                restored.stats.rebuilt_operations
            ),
            (2, 0)
        );
        changed["operations"][1]["offset"] = json!(33.);
        let offset_doc = typed(&changed);
        let offset = session.rebuild(&offset_doc).unwrap();
        assert_eq!(
            (
                offset.stats.reused_operations,
                offset.stats.rebuilt_operations
            ),
            (1, 2)
        );
        assert_eq!(
            signature(&offset.solid),
            signature(&offset_doc.rebuild().unwrap())
        );
        let root_again = session.rebuild(&root_doc).unwrap();
        assert!(Arc::ptr_eq(&root.solid, &root_again.solid));
        let mut opposite = value.clone();
        opposite["operations"].as_array_mut().unwrap().truncate(2);
        opposite["operations"][1]["side"] = json!(if side == "negative" {
            "positive"
        } else {
            "negative"
        });
        let opposite_doc = typed(&opposite);
        session.rebuild(&cut_doc).unwrap();
        let switched = session.rebuild(&opposite_doc).unwrap();
        assert_eq!(
            (
                switched.stats.reused_operations,
                switched.stats.rebuilt_operations
            ),
            (1, 1)
        );
        assert_eq!(
            signature(&switched.solid),
            signature(&opposite_doc.rebuild().unwrap())
        );
    }
}

fn capsule() -> Value {
    let a = |center: [f64; 2], start: f64| json!({"kind":"arc","center":center,"radius":5.,"start_angle":start,"sweep":PI/2.});
    json!({"kind":"arc_line_extrusion","id":"stock","outer":[{"kind":"line","start":[15.,15.],"end":[35.,15.]},a([35.,20.],-PI/2.),a([35.,20.],0.),{"kind":"line","start":[35.,25.],"end":[15.,25.]},a([15.,20.],PI/2.),a([15.,20.],PI)],"height":8.})
}

#[test]
fn noncentered_cuts_and_initial_hole_ownership_use_world_coordinates() {
    let mut value = document("negative");
    value["operations"][0] = capsule();
    value["operations"][1]["offset"] = json!(25.);
    value["operations"][2]["center"] = json!([20., 20.]);
    value["operations"][2]["radius"] = json!(1.);
    let area = (200. + 25. * PI) / 2.;
    check(&typed(&value).rebuild().unwrap(), (area - PI) * 8., 1);
    let circle:Vec<_>=(0..4).map(|i|json!({"kind":"arc","center":[18.,20.],"radius":1.,"start_angle":i as f64*PI/2.,"sweep":PI/2.})).collect();
    value["operations"][0]["holes"] = json!([circle]);
    value["operations"][2]["center"] = json!([22., 20.]);
    check(&typed(&value).rebuild().unwrap(), (area - 2. * PI) * 8., 2);
    value["operations"].as_array_mut().unwrap().truncate(2);
    value["operations"][1]["side"] = json!("positive");
    check(&typed(&value).rebuild().unwrap(), area * 8., 0);
    let mut after = document("negative");
    after["operations"] = json!([after["operations"][0],{"kind":"bore","id":"before","input":"stock","mode":"through","center":[0.,0.],"radius":2.},{"kind":"plane_split","id":"after","input":"before","offset":34.,"normal_angle":0.,"side":"negative"}]);
    let total = 96000. - 36. * (1. - PI / 4.) * 20.;
    let positive = (360. - 18. * (1. - PI / 4.)) * 20.;
    check(
        &typed(&after).rebuild().unwrap(),
        total - positive - PI * 4. * 20.,
        1,
    );
}

#[test]
fn rejected_cuts_schema_blind_and_display_failures_preserve_real_accepted_geometry() {
    let value = document("negative");
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    let signature_before = signature(&accepted.solid);
    let step_before = export_step_bounded_analytic_mm(&accepted.solid, 1e-6).unwrap();
    let mut bad = Vec::new();
    for offset in [40., 37., 0., 41.] {
        let mut v = value.clone();
        v["operations"][1]["offset"] = json!(offset);
        bad.push(v);
    }
    let mut crossing = value.clone();
    crossing["operations"] = json!([crossing["operations"][0],{"kind":"bore","id":"prior","input":"stock","mode":"through","center":[34.,0.],"radius":1.},{"kind":"plane_split","id":"cut","input":"prior","offset":34.,"normal_angle":0.,"side":"negative"}]);
    bad.push(crossing);
    let mut blind = value.clone();
    blind["operations"][2]["mode"] = json!("blind");
    blind["operations"][2]["depth"] = json!(2.);
    bad.push(blind);
    let mut side = value.clone();
    side["operations"][1]["side"] = json!("both");
    bad.push(side);
    let mut unknown = value.clone();
    unknown["operations"][1]["normal"] = json!([1., 0.]);
    bad.push(unknown);
    let mut box_root = value.clone();
    box_root["operations"][0] = json!({"kind":"box","id":"stock","size":[80.,60.,20.]});
    bad.push(box_root);
    let mut polygon = value.clone();
    polygon["operations"][0] = json!({"kind":"extrusion","id":"stock","outer":[[-40.,-30.],[40.,-30.],[40.,30.],[-40.,30.]],"height":20.});
    bad.push(polygon);
    let mut display = value.clone();
    display["operations"][0]["size"] = json!([1e8, 1e8, 1e8]);
    display["operations"][0]["corner_radius"] = json!(1e6);
    display["tolerance"]["linear"] = json!(1.);
    display["operations"][1]["offset"] = json!(0.);
    display["operations"].as_array_mut().unwrap().truncate(2);
    bad.push(display);
    for candidate in bad {
        let display_case = candidate["operations"][0]["size"][0] == json!(1e8);
        if display_case {
            typed(&candidate).rebuild().unwrap();
        }
        let report: Value =
            serde_json::from_str(&session.evaluate_json(&candidate.to_string()).unwrap()).unwrap();
        assert_eq!(report["ok"], false, "{candidate}");
        assert!(report.get("mesh").is_none());
        if display_case {
            assert_eq!(report["diagnostic"]["code"], "display_rejected");
        }
        let retained = session.rebuild(&good).unwrap();
        assert_eq!(
            (
                retained.stats.reused_operations,
                retained.stats.rebuilt_operations
            ),
            (3, 0)
        );
        assert!(Arc::ptr_eq(&accepted.solid, &retained.solid));
        assert_eq!(signature(&retained.solid), signature_before);
        assert_eq!(
            export_step_bounded_analytic_mm(&retained.solid, 1e-6).unwrap(),
            step_before
        );
    }
}

#[test]
fn line_child_continues_actual_operations_and_signed_zero_reuses_prefixes() {
    let mut value = document("positive");
    value["operations"][0] = json!({"kind":"arc_line_extrusion","id":"stock","height":5.,"outer":[{"kind":"line","start":[0.,-4.],"end":[20.,-4.]},{"kind":"line","start":[20.,-4.],"end":[20.,4.]},{"kind":"line","start":[20.,4.],"end":[0.,4.]},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":PI/2.,"sweep":PI/2.},{"kind":"arc","center":[0.,0.],"radius":4.,"start_angle":PI,"sweep":PI/2.}]});
    value["operations"][1]["offset"] = json!(10.);
    value["operations"].as_array_mut().unwrap().truncate(2);
    let good = typed(&value);
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&good).unwrap();
    check(&accepted.solid, 400., 0);
    assert!(accepted
        .solid
        .shell
        .faces
        .iter()
        .all(|f| matches!(f.surface, Surface::Plane { .. })));
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&value.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    for (operation, volume, holes) in [
        (
            json!({"kind":"bore","id":"later","input":"cut","mode":"through","center":[15.,0.],"radius":1.}),
            400. - 5. * PI,
            1,
        ),
        (
            json!({"kind":"plane_split","id":"later","input":"cut","offset":15.,"normal_angle":0.,"side":"negative"}),
            200.,
            0,
        ),
    ] {
        let mut continued = value.clone();
        continued["operations"]
            .as_array_mut()
            .unwrap()
            .push(operation);
        let result = session.rebuild(&typed(&continued)).unwrap();
        check(&result.solid, volume, holes);
        assert_eq!(
            (
                result.stats.reused_operations,
                result.stats.rebuilt_operations
            ),
            (2, 1)
        );
        assert!(Arc::ptr_eq(
            &accepted.solid,
            &session.rebuild(&good).unwrap().solid
        ));
    }
    let mut zero = document("negative");
    zero["operations"].as_array_mut().unwrap().truncate(2);
    zero["operations"][1]["offset"] = json!(-0.0);
    let negative_zero = typed(&zero);
    let accepted = session.rebuild(&negative_zero).unwrap();
    zero["operations"][1]["offset"] = json!(0.0);
    let positive_zero = typed(&zero);
    let repeated = session.rebuild(&positive_zero).unwrap();
    assert_eq!(
        (
            repeated.stats.reused_operations,
            repeated.stats.rebuilt_operations
        ),
        (2, 0)
    );
    assert!(Arc::ptr_eq(&accepted.solid, &repeated.solid));
    assert!(
        (positive_zero.rebuild().unwrap().volume().unwrap() - accepted.solid.volume().unwrap())
            .abs()
            < 1e-8
    );
    for stock in [
        json!({"kind":"box","id":"stock","size":[80.,60.,20.]}),
        json!({"kind":"extrusion","id":"stock","outer":[[-40.,-30.],[40.,-30.],[40.,30.],[-40.,30.]],"height":20.}),
    ] {
        let mut legacy = document("negative");
        legacy["operations"] = json!([stock,{"kind":"bore","id":"hole","input":"stock","mode":"through","center":[0.,0.],"radius":2.}]);
        assert!(
            (typed(&legacy).rebuild().unwrap().volume().unwrap() - (96000. - PI * 4. * 20.)).abs()
                < 1e-7
        );
    }
}
