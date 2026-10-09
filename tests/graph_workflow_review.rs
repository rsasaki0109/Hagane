use hagane::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
fn fixture() -> Value {
    serde_json::from_str(include_str!("../docs/graph-workflow.json")).unwrap()
}
fn curve_signature(curve: &Curve) -> Value {
    let Curve::Nurbs(c) = curve else {
        panic!("replayed graph edges must remain rational NURBS")
    };
    json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"points":c.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()})
}
fn pcurve_signature(pcurve: &PCurve) -> Value {
    match pcurve {
        PCurve::Affine { origin, direction } => {
            json!({"kind":"affine","origin":origin,"direction":direction})
        }
        PCurve::Nurbs(c) => {
            json!({"kind":"nurbs","degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"points":c.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()})
        }
        _ => panic!("graph replay introduced an unrelated pcurve representation"),
    }
}
fn signature(s: &Solid) -> Value {
    json!({"vertices":s.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),
        "edges":s.edges.iter().map(|e|json!({"vertices":e.vertices,"curve":curve_signature(&e.curve)})).collect::<Vec<_>>(),
        "faces":s.shell.faces.iter().map(|f|{let Surface::Nurbs(surface)=&f.surface else{panic!("graph surfaces must be retained NURBS")};
            json!({"orientation":f.orientation,"degrees":surface.degrees(),"counts":surface.control_counts(),"knots":[surface.knots(0).unwrap(),surface.knots(1).unwrap()],"weights":surface.weights(),"points":surface.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"wires":f.wires.iter().map(|w|w.coedges.iter().map(|c|json!({"edge":c.edge,"forward":c.forward,"pcurve":pcurve_signature(&c.pcurve)})).collect::<Vec<_>>()).collect::<Vec<_>>()})
        }).collect::<Vec<_>>()})
}
fn assert_closed_geometry(s: &Solid) {
    let mut incidence: BTreeMap<usize, (usize, i32)> = BTreeMap::new();
    for f in &s.shell.faces {
        for w in &f.wires {
            for (i, c) in w.coedges.iter().enumerate() {
                let e = &s.edges[c.edge];
                let next = &w.coedges[(i + 1) % w.coedges.len()];
                assert_eq!(
                    e.vertices[usize::from(c.forward)],
                    s.edges[next.edge].vertices[usize::from(!next.forward)]
                );
                let usage = incidence.entry(c.edge).or_default();
                usage.0 += 1;
                usage.1 += f.orientation as i32 * if c.forward { 1 } else { -1 };
                let [a, b] = e.curve.range();
                for j in 0..=32 {
                    let t = a + (b - a) * j as f64 / 32.;
                    let uv = c.pcurve.try_evaluate(t).unwrap();
                    let p = e.curve.try_evaluate(t).unwrap();
                    assert!((p - f.surface.try_evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-8);
                }
                for k in 0..2 {
                    assert!(
                        (e.curve.try_evaluate([a, b][k]).unwrap()
                            - s.vertices[e.vertices[k]].point)
                            .norm()
                            < 1e-8
                    );
                }
            }
        }
    }
    assert_eq!(incidence.len(), s.edges.len());
    assert!(incidence
        .values()
        .all(|&(count, sign)| count == 2 && sign == 0));
}
fn independent_volume(source: &NurbsGraphSolid, center: [f64; 2], r: f64) -> f64 {
    let [w, d, h] = source.dimensions();
    let b = source.bulge();
    let [ur, vr] = source.source_domain();
    let integral = |[a, z]: [f64; 2]| (z - a) * ((a + z) / 2. - (a * a + a * z + z * z) / 3.);
    let stock =
        w * d * (h * (ur[1] - ur[0]) * (vr[1] - vr[0]) + 4. * b * integral(ur) * integral(vr));
    let u = center[0] / w;
    let v = center[1] / d;
    let disk = std::f64::consts::PI
        * r
        * r
        * (h + 4. * b * u * (1. - u) * v * (1. - v)
            - b * r * r * (v * (1. - v) / (w * w) + u * (1. - u) / (d * d))
            + b * r.powi(4) / (6. * w * w * d * d));
    stock - disk
}
fn typed(v: &Value) -> GraphWorkflowDocument {
    serde_json::from_value(v.clone()).unwrap()
}
#[test]
fn replay_retains_actual_rational_brep_and_matches_independent_column_volume() {
    let t = Tolerance::default();
    for bulge in [-12., 0., 36.] {
        let mut v = fixture();
        v["operations"][0]["bulge"] = json!(bulge);
        let d = typed(&v);
        let shape = d.rebuild().unwrap();
        let pose = Transform::translation(Vec3::new(12., -5., 8.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.4363323129985824).unwrap())
            .unwrap();
        let source = NurbsGraphSolid::new([80., 60., 20.], bulge, t)
            .unwrap()
            .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], t)
            .unwrap()
            .transformed(pose, t)
            .unwrap();
        let expected = source.through_xy_circle([40., 30.], 10., t).unwrap();
        assert_eq!(signature(shape.brep()), signature(expected.brep()));
        assert_closed_geometry(shape.brep());
        assert_eq!(
            (
                shape.brep().vertices.len(),
                shape.brep().edges.len(),
                shape.brep().shell.faces.len()
            ),
            (16, 24, 10)
        );
        shape.validate(t).unwrap();
        assert!(matches!(
            shape.brep().validate(t),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(shape.brep().volume(), Err(Error::Unsupported(_))));
        assert!(matches!(
            export_step_mm(shape.brep(), t),
            Err(Error::Unsupported(_))
        ));
        assert!(shape
            .export_step_mm(t)
            .unwrap()
            .contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(
            (shape.volume().unwrap() - independent_volume(&source, [40., 30.], 10.)).abs() < 1e-8
        );
        let GraphWorkflowShape::Circular(body) = &shape else {
            panic!()
        };
        assert_eq!(body.center(), [40., 30.]);
        assert_eq!(body.radius(), 10.);
        for edge in &shape.brep().edges[12..20] {
            let Curve::Nurbs(c) = &edge.curve else {
                panic!()
            };
            assert_eq!(c.degree(), 8);
            assert!(c.weights().iter().any(|w| *w != 1.));
        }
        for cap in &shape.brep().shell.faces[..2] {
            for c in &cap.wires[1].coedges {
                let PCurve::Nurbs(uv) = &c.pcurve else {
                    panic!()
                };
                assert_eq!(uv.degree(), 2);
                assert_eq!(uv.weights(), &[1., std::f64::consts::FRAC_1_SQRT_2, 1.]);
            }
        }
        let band = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        for (local, expected) in [
            (Point3::new(40., 30., 10.), PointLocation::Outside),
            (Point3::new(20., 20., 5.), PointLocation::Inside),
        ] {
            assert_eq!(
                shape.classify_point(pose.point(local), band).unwrap(),
                expected
            );
        }
        let properties = shape.mass_properties(t).unwrap();
        assert!((properties.volume - shape.volume().unwrap()).abs() < 1e-8);
        shape.inertia_properties(t).unwrap();
        shape.bounds().unwrap();
    }
}
#[test]
fn serializer_reports_retained_snapshot_geometry_and_real_changed_suffix() {
    let mut session = GraphWorkflowSession::new();
    let baseline = fixture();
    let d = typed(&baseline);
    let first = session.rebuild(&d).unwrap();
    let prefixes: Vec<_> = (0..4)
        .map(|i| Arc::clone(session.prefix_snapshot(i).unwrap()))
        .collect();
    let first_signature = signature(first.shape.brep());
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&baseline.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["shape"]["kind"], "circular");
    assert_eq!(report["rebuild"]["reused_operations"], 4);
    assert_eq!(report["rebuild"]["rebuilt_operations"], 0);
    assert!(Arc::ptr_eq(&first.shape, session.accepted_shape().unwrap()));
    let brep = &report["shape"]["brep"];
    assert_eq!(brep["vertices"], first_signature["vertices"]);
    for (i, e) in first.shape.brep().edges.iter().enumerate() {
        let actual = &brep["curves"][i];
        let retained = curve_signature(&e.curve);
        for key in ["degree", "knots", "weights"] {
            assert_eq!(actual[key], retained[key]);
        }
        assert_eq!(actual["control_points"], retained["points"]);
    }
    for (i, f) in first.shape.brep().shell.faces.iter().enumerate() {
        let actual = &brep["surfaces"][i];
        let retained = &first_signature["faces"][i];
        for key in ["degrees", "knots", "weights", "counts"] {
            assert_eq!(actual[key], retained[key]);
        }
        assert_eq!(actual["control_points"], retained["points"]);
        for (wi, w) in f.wires.iter().enumerate() {
            for (ci, c) in w.coedges.iter().enumerate() {
                let actual = &brep["wire_pcurves"][i][wi][ci];
                match &c.pcurve {
                    PCurve::Affine { origin, direction } => {
                        assert_eq!(actual["origin"], json!(origin));
                        assert_eq!(actual["direction"], json!(direction));
                    }
                    PCurve::Nurbs(uv) => {
                        assert_eq!(actual["weights"], json!(uv.weights()));
                        assert_eq!(actual["knots"], json!(uv.knots()));
                        assert_eq!(
                            actual["control_points"],
                            json!(uv
                                .control_points()
                                .iter()
                                .map(|p| [p.x, p.y, p.z])
                                .collect::<Vec<_>>())
                        );
                    }
                    _ => panic!(),
                }
            }
        }
    }
    let mut changed = baseline;
    changed["operations"][3]["radius"] = json!(8.);
    let rebuilt = session.rebuild(&typed(&changed)).unwrap();
    assert_eq!(rebuilt.stats.reused_operations, 3);
    assert_eq!(rebuilt.stats.rebuilt_operations, 1);
    assert_eq!(rebuilt.stats.rebuilt_operation_ids, ["bore"]);
    for (i, prefix) in prefixes.iter().take(3).enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
    }
    assert!(!Arc::ptr_eq(&prefixes[3], &rebuilt.shape));
    assert_ne!(signature(rebuilt.shape.brep()), first_signature);
    assert_eq!(
        signature(rebuilt.shape.brep()),
        signature(typed(&changed).rebuild().unwrap().brep())
    );
    assert_eq!(signature(prefixes[3].brep()), first_signature);
    assert!(session.prefix_snapshot(usize::MAX).is_none());
}
#[test]
fn invalid_scope_fields_limits_and_display_failure_preserve_all_real_snapshots() {
    let baseline = fixture();
    let accepted = typed(&baseline);
    let mut session = GraphWorkflowSession::new();
    let result = session.rebuild(&accepted).unwrap();
    let geometry = signature(result.shape.brep());
    let prefixes: Vec<_> = (0..4)
        .map(|i| Arc::clone(session.prefix_snapshot(i).unwrap()))
        .collect();
    let mut invalid = Vec::new();
    for (key, value) in [
        ("schema_version", json!(2)),
        ("units", json!("m")),
        ("unexpected", json!(true)),
    ] {
        let mut v = baseline.clone();
        v[key] = value;
        invalid.push(v);
    }
    for (index, key, value) in [
        (0, "id", json!("bad id")),
        (1, "input", json!(0)),
        (2, "input", json!("stock")),
        (3, "id", json!("stock")),
        (3, "depth", json!(2.)),
        (3, "mode", json!("blind")),
        (3, "kind", json!("circular_blind_bore")),
        (3, "radius", json!(100.)),
        (3, "center", json!([1., 1.])),
        (2, "axis", json!([0., 0., 0.])),
        (1, "ranges", json!([[0., 1.1], [0., 1.]])),
        (0, "dimensions", json!([80., 60., 0.])),
    ] {
        let mut v = baseline.clone();
        v["operations"][index][key] = value;
        invalid.push(v);
    }
    let mut multiple = baseline.clone();
    multiple["operations"].as_array_mut().unwrap().push(json!({"kind":"circular_through_bore","id":"second","input":"bore","center":[20.,20.],"radius":2.}));
    invalid.push(multiple);
    let mut post = baseline.clone();
    post["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"uv_trim","id":"after","input":"bore","ranges":[[0.2,0.8],[0.2,0.8]]}));
    invalid.push(post);
    let mut empty = baseline.clone();
    empty["operations"] = json!([]);
    invalid.push(empty);
    let mut many = baseline.clone();
    many["operations"]=json!((0..17).map(|i|json!({"kind":"graph_stock","id":format!("stock{i}"),"dimensions":[80.,60.,20.],"bulge":30.})).collect::<Vec<_>>());
    invalid.push(many);
    for (key, value) in [
        ("max_error", json!(0.)),
        ("max_triangles", json!(31)),
        ("max_triangles", json!(65537)),
        ("unknown", json!(0)),
    ] {
        let mut v = baseline.clone();
        v["display"][key] = value;
        invalid.push(v);
    }
    for v in invalid {
        let report: Value = serde_json::from_str(
            &session
                .evaluate_json_with(&v.to_string(), |_, _| {
                    panic!("invalid replay must not reach serializer")
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(report["ok"], false, "{v}");
        assert!(report.get("shape").is_none());
        assert_eq!(session.accepted_document(), Some(&accepted));
        assert_eq!(
            signature(session.accepted_shape().unwrap().brep()),
            geometry
        );
        for (i, prefix) in prefixes.iter().enumerate() {
            assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
        }
    }
    let mut display = baseline.clone();
    display["operations"][3]["radius"] = json!(8.);
    display["display"]["max_triangles"] = json!(32);
    let report: Value =
        serde_json::from_str(&session.evaluate_json(&display.to_string()).unwrap()).unwrap();
    assert_eq!(report["ok"], false);
    assert!(report.get("shape").is_none());
    assert!(Arc::ptr_eq(
        &result.shape,
        session.accepted_shape().unwrap()
    ));
    assert_eq!(session.accepted_document(), Some(&accepted));
    let report: Value = serde_json::from_str(
        &session
            .evaluate_json_with(&display.to_string(), |_, _| {
                Err(Error::InvalidInput(
                    "independent deliberate serialization failure",
                ))
            })
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["ok"], false);
    assert!(Arc::ptr_eq(
        &result.shape,
        session.accepted_shape().unwrap()
    ));
    let oversized = " ".repeat(65537);
    let report: Value = serde_json::from_str(&session.evaluate_json(&oversized).unwrap()).unwrap();
    assert_eq!(report["ok"], false);
    assert!(Arc::ptr_eq(
        &result.shape,
        session.accepted_shape().unwrap()
    ));
}
