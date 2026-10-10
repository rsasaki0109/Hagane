use hagane::*;
use serde_json::{json, Value};
use std::f64::consts::PI;
use std::sync::Arc;

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

fn value(root: Value, ops: Vec<Value>, scale: f64) -> Value {
    let mut operations = vec![root];
    operations.extend(ops);
    json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-6*scale,"angular":1e-10,"relative":0.},"operations":operations})
}
fn bore(id: &str, input: &str, center: [f64; 2], radius: f64) -> Value {
    json!({"kind":"bore","id":id,"input":input,"mode":"through","center":center,"radius":radius})
}
fn cut(id: &str, input: &str, offset: f64, angle: f64, side: &str) -> Value {
    json!({"kind":"plane_split","id":id,"input":input,"offset":offset,"normal_angle":angle,"side":side})
}
#[test]
fn box_history_pre_cut_bore_is_rebuilt_in_cut_mode_and_matches_direct_actual_breps() {
    let base = value(
        json!({"kind":"box","id":"stock","size":[20.,16.,5.]}),
        vec![bore("prior", "stock", [-4., 0.], 1.)],
        1.,
    );
    let mut full = base.clone();
    full["operations"]
        .as_array_mut()
        .unwrap()
        .push(cut("cut", "prior", 0., 0., "negative"));
    full["operations"]
        .as_array_mut()
        .unwrap()
        .push(bore("late", "cut", [-7., 0.], 0.5));
    let mut session = WorkflowSession::new();
    let stock = session.rebuild(&prefix(&base, 1)).unwrap();
    let old = session.rebuild(&typed(&base)).unwrap();
    assert!(old
        .solid
        .edges
        .iter()
        .any(|e| matches!(e.curve, Curve::Circle { .. })));
    let accepted = session.rebuild(&typed(&full)).unwrap();
    assert_eq!(
        (
            accepted.stats.reused_operations,
            accepted.stats.rebuilt_operations
        ),
        (1, 3)
    );
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = prefix(&base, 1).rebuild().unwrap();
    let before = bore_normal_prism(
        &source,
        Point3::new(-4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        tol,
    )
    .unwrap();
    let split = split_normal_prism_by_plane_components(
        before.kept(),
        &plane(0.),
        Vec3::new(0., 0., 1.),
        tol,
    )
    .unwrap();
    let direct = bore_normal_prism(
        &split.negative()[0],
        Point3::new(-7., 0., 0.),
        0.5,
        Vec3::new(0., 0., 1.),
        tol,
    )
    .unwrap();
    assert_eq!(signature(&accepted.solid), signature(direct.kept()));
    check(&accepted.solid, 800. - 6.25 * PI, 2);
    let mut edited = full.clone();
    edited["operations"][3]["radius"] = json!(0.4);
    let late = session.rebuild(&typed(&edited)).unwrap();
    assert_eq!(
        (late.stats.reused_operations, late.stats.rebuilt_operations),
        (3, 1)
    );
    assert_eq!(
        signature(&late.solid),
        signature(&typed(&edited).rebuild().unwrap())
    );
    let restored = session.rebuild(&typed(&base)).unwrap();
    assert_eq!(
        (
            restored.stats.reused_operations,
            restored.stats.rebuilt_operations
        ),
        (1, 1)
    );
    assert!(!Arc::ptr_eq(&restored.solid, &old.solid));
    assert_eq!(signature(&restored.solid), signature(&old.solid));
    let root = session.rebuild(&prefix(&base, 1)).unwrap();
    assert!(Arc::ptr_eq(&stock.solid, &root.solid));
    let mut firstcut = value(
        json!({"kind":"box","id":"stock","size":[20.,16.,5.]}),
        vec![
            cut("cut", "stock", 0., 0., "positive"),
            bore("hole", "cut", [5., 0.], 1.),
        ],
        1.,
    );
    let result = session.rebuild(&typed(&firstcut)).unwrap();
    check(&result.solid, 800. - 5. * PI, 1);
    firstcut["operations"][1]["offset"] = json!(2.);
    let changed = session.rebuild(&typed(&firstcut)).unwrap();
    assert_eq!(
        (
            changed.stats.reused_operations,
            changed.stats.rebuilt_operations
        ),
        (1, 2)
    );
    check(&changed.solid, 640. - 5. * PI, 1);
}
#[test]
fn noncentered_polygon_hole_ownership_concavity_and_oblique_box_cuts() {
    let root = json!({"kind":"extrusion","id":"stock","outer":[[10.,10.],[30.,10.],[30.,26.],[10.,26.]],"holes":[[[12.,16.],[14.,16.],[14.,18.],[12.,18.]]],"height":5.});
    let d = value(
        root,
        vec![
            cut("cut", "stock", 20., 0., "negative"),
            bore("hole", "cut", [17., 20.], 0.5),
        ],
        1.,
    );
    let s = typed(&d).rebuild().unwrap();
    check(&s, 780. - 1.25 * PI, 2);
    assert!((s.bounds().min.x - 10.).abs() < 1e-10);
    assert!((s.bounds().max.x - 20.).abs() < 1e-10);
    let root = json!({"kind":"extrusion","id":"stock","outer":[[-6.,-4.],[6.,-4.],[6.,6.],[2.,6.],[2.,0.],[-2.,0.],[-2.,6.],[-6.,6.]],"height":5.});
    let d = value(
        root.clone(),
        vec![cut("cut", "stock", 2., PI / 2., "negative")],
        1.,
    );
    check(&typed(&d).rebuild().unwrap(), 320., 0);
    let mut multi = d.clone();
    multi["operations"][1]["side"] = json!("positive");
    let err = typed(&multi).rebuild().unwrap_err();
    assert_eq!(err.operation_id.as_deref(), Some("cut"));
    assert_eq!(err.category, "unsupported");
    let d = value(
        json!({"kind":"box","id":"stock","size":[20.,16.,5.]}),
        vec![cut("cut", "stock", 0., 0.2, "negative")],
        1.,
    );
    check(&typed(&d).rebuild().unwrap(), 800., 0);
    for scale in [1e-4, 10.] {
        let d = value(
            json!({"kind":"box","id":"stock","size":[20.*scale,16.*scale,5.*scale]}),
            vec![
                cut("cut", "stock", 0., 0., "negative"),
                bore("hole", "cut", [-4. * scale, 0.], scale),
            ],
            scale,
        );
        let s = typed(&d).rebuild().unwrap();
        s.validate(Tolerance::new(1e-6 * scale).unwrap()).unwrap();
        assert!(
            (s.volume().unwrap() - (800. - 5. * PI) * scale.powi(3)).abs() < 1e-8 * scale.powi(3)
        );
        let encoded = export_step_bounded_analytic_mm(&s, 1e-6 * scale).unwrap();
        let imported =
            import_step_bounded_analytic_mm(&encoded, Tolerance::new(1e-6 * scale).unwrap())
                .unwrap();
        assert!((imported.volume().unwrap() - s.volume().unwrap()).abs() < 1e-8 * scale.powi(3));
    }
}
#[test]
fn invalid_blind_skew_contact_schema_and_multibody_edits_preserve_accepted_snapshots() {
    let good = value(
        json!({"kind":"box","id":"stock","size":[20.,16.,5.]}),
        vec![
            cut("cut", "stock", 0., 0., "negative"),
            bore("hole", "cut", [-4., 0.], 1.),
        ],
        1.,
    );
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&typed(&good)).unwrap();
    let exported = export_step_bounded_analytic_mm(&accepted.solid, 1e-6).unwrap();
    let mut bad = vec![];
    for x in [10., 11.] {
        let mut d = good.clone();
        d["operations"][1]["offset"] = json!(x);
        bad.push(d);
    }
    let mut contact = good.clone();
    contact["operations"][2]["center"] = json!([-1., 0.]);
    bad.push(contact);
    for before in [true, false] {
        let mut d = good.clone();
        let blind = json!({"kind":"bore","id":"blind","input":if before{"stock"}else{"cut"},"mode":"blind","center":[-4.,0.],"radius":1.,"depth":1.});
        d["operations"] = if before {
            json!([
                d["operations"][0],
                blind,
                cut("cut", "blind", 0., 0., "negative")
            ])
        } else {
            json!([d["operations"][0], d["operations"][1], blind])
        };
        bad.push(d);
    }
    let mut skew = good.clone();
    skew["operations"][0] = json!({"kind":"extrusion","id":"stock","outer":[[-10.,-8.],[10.,-8.],[10.,8.],[-10.,8.]],"height":5.,"offset":[0.1,0.]});
    bad.push(skew);
    let mut unknown = good.clone();
    unknown["operations"][1]["axis"] = json!([0., 0., 1.]);
    bad.push(unknown);
    let mut stale = good.clone();
    stale["operations"][2]["input"] = json!("stock");
    bad.push(stale);
    let concave = json!({"kind":"extrusion","id":"stock","outer":[[-6.,-4.],[6.,-4.],[6.,6.],[2.,6.],[2.,0.],[-2.,0.],[-2.,6.],[-6.,6.]],"height":5.});
    bad.push(value(
        concave,
        vec![cut("cut", "stock", 2., PI / 2., "positive")],
        1.,
    ));
    for d in bad {
        let report: Value =
            serde_json::from_str(&session.evaluate_json(&d.to_string()).unwrap()).unwrap();
        assert_eq!(report["ok"], false, "{d}");
        if d["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|op| op["id"] == "blind")
        {
            assert_eq!(report["diagnostic"]["operation_id"], "blind");
            assert_eq!(report["diagnostic"]["category"], "unsupported");
        }
        let retained = session.rebuild(&typed(&good)).unwrap();
        assert!(Arc::ptr_eq(&retained.solid, &accepted.solid));
        assert_eq!(
            (
                retained.stats.reused_operations,
                retained.stats.rebuilt_operations
            ),
            (3, 0)
        );
        assert_eq!(
            export_step_bounded_analytic_mm(&retained.solid, 1e-6).unwrap(),
            exported
        );
    }
}

#[test]
fn planar_cut_profile_and_hole_resources_are_explicit_and_atomic() {
    let good = value(
        json!({"kind":"box","id":"stock","size":[20.,16.,5.]}),
        vec![cut("cut", "stock", 0., 0., "negative")],
        1.,
    );
    let mut session = WorkflowSession::new();
    let accepted = session.rebuild(&typed(&good)).unwrap();
    let mut holes = Vec::new();
    for row in 0..4 {
        for col in 0..4 {
            let x = 10. + 5. * col as f64;
            let y = 10. + 5. * row as f64;
            holes.push(json!([
                [x - 1., y - 1.],
                [x + 1., y - 1.],
                [x + 1., y + 1.],
                [x - 1., y + 1.]
            ]));
        }
    }
    let sixteen = value(
        json!({"kind":"extrusion","id":"stock","outer":[[-40.,-40.],[40.,-40.],[40.,40.],[-40.,40.]],"holes":holes,"height":5.}),
        vec![cut("cut", "stock", 0., 0., "negative")],
        1.,
    );
    check(&typed(&sixteen).rebuild().unwrap(), 16000., 0);
    let mut extra = sixteen.clone();
    extra["operations"][0]["holes"]
        .as_array_mut()
        .unwrap()
        .push(json!([[29., 29.], [31., 29.], [31., 31.], [29., 31.]]));
    let mut bore_extra = sixteen.clone();
    bore_extra["operations"]
        .as_array_mut()
        .unwrap()
        .push(bore("hole", "cut", [-10., 0.], 1.));
    let regular = |count: usize| -> Vec<[f64; 2]> {
        (0..count)
            .map(|i| {
                let angle = 2. * PI * i as f64 / count as f64;
                [20. * angle.cos(), 20. * angle.sin()]
            })
            .collect()
    };
    let at_limit = value(
        json!({"kind":"extrusion","id":"stock","outer":regular(125),"height":5.}),
        vec![cut("cut", "stock", 0.17, 0.123, "negative")],
        1.,
    );
    let successful = typed(&at_limit).rebuild().unwrap();
    successful.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    let mut too_many = at_limit.clone();
    too_many["operations"][0]["outer"] = json!(regular(126));
    for candidate in [extra, bore_extra, too_many] {
        let error = typed(&candidate).rebuild().unwrap_err();
        assert_eq!(error.category, "unsupported");
        assert_eq!(error.code, "unsupported_history");
        let report: Value =
            serde_json::from_str(&session.evaluate_json(&candidate.to_string()).unwrap()).unwrap();
        assert_eq!(report["ok"], false);
        let retained = session.rebuild(&typed(&good)).unwrap();
        assert!(Arc::ptr_eq(&retained.solid, &accepted.solid));
    }
}
