use hagane::*;
use serde_json::json;
#[test]
fn bottom_blind_has_exact_floor_volume_closure_and_classification() {
    for scale in [1e-6, 1., 1000.] {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../docs/workflow-bottom-blind-example.json"))
                .unwrap();
        value["tolerance"]["linear"] = json!(1e-8 * scale);
        for field in ["outer", "holes"] {
            fn scale_points(v: &mut serde_json::Value, s: f64) {
                if let Some(a) = v.as_array_mut() {
                    for x in a {
                        if x.is_number() {
                            *x = json!(x.as_f64().unwrap() * s);
                        } else {
                            scale_points(x, s);
                        }
                    }
                }
            }
            scale_points(&mut value["operations"][0][field], scale);
        }
        value["operations"][0]["height"] = json!(24. * scale);
        value["operations"][0]["offset"] = json!([18. * scale, -12. * scale]);
        value["operations"][1]["center"] = json!([24. * scale, 0.]);
        value["operations"][1]["radius"] = json!(4. * scale);
        value["operations"][1]["depth"] = json!(8. * scale);
        let doc: WorkflowDocument = serde_json::from_value(value).unwrap();
        let solid = doc.rebuild().unwrap();
        let t = Tolerance::new(doc.tolerance.linear).unwrap();
        solid.validate(t).unwrap();
        let expected = (4408. * 24. - 16. * std::f64::consts::PI * 8.) * scale.powi(3);
        assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
        for (p, location) in [
            (Point3::new(24., 0., -8.), PointLocation::Outside),
            (Point3::new(24., 0., -4.), PointLocation::Boundary),
            (Point3::new(24., 0., 0.), PointLocation::Inside),
            (Point3::new(28., 0., -8.), PointLocation::Boundary),
        ] {
            assert_eq!(
                classify_point_in_solid(
                    &solid,
                    p * scale,
                    GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap()
                )
                .unwrap(),
                location
            );
        }
        let floor = solid.shell.faces.last().unwrap();
        assert_eq!(floor.orientation, -1);
        let mesh = solid.tessellate(0.01 * scale, t).unwrap();
        assert!(
            (mesh.signed_volume() - expected).abs()
                < 2. * std::f64::consts::PI * 0.01 * 4. * 8. * scale.powi(3)
        );
        let key = |p: Point3| {
            [
                (p.x / (1e-9 * scale)).round() as i64,
                (p.y / (1e-9 * scale)).round() as i64,
                (p.z / (1e-9 * scale)).round() as i64,
            ]
        };
        let mut uses = std::collections::BTreeMap::new();
        for tri in &mesh.triangles {
            for i in 0..3 {
                let a = key(mesh.positions[tri[i]]);
                let b = key(mesh.positions[tri[(i + 1) % 3]]);
                let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let e = uses.entry(edge).or_insert((0, 0));
                e.0 += 1;
                e.1 += sign;
            }
        }
        assert!(uses
            .values()
            .all(|&(count, balance)| count == 2 && balance == 0));
        let mut session = WorkflowSession::new();
        session.rebuild(&doc).unwrap();
        let mut edited = doc.clone();
        if let WorkflowOperation::Bore { entry, .. } = &mut edited.operations[1] {
            *entry = WorkflowBoreEntry::Top;
        }
        let changed = session.rebuild(&edited).unwrap();
        assert_eq!(changed.stats.reused_operations, 1);
        assert_eq!(
            changed.solid.mesh_json(0.05 * scale, t).unwrap(),
            edited
                .rebuild()
                .unwrap()
                .mesh_json(0.05 * scale, t)
                .unwrap()
        );
    }
}
#[test]
fn bottom_depth_clearance_entry_validation_and_cache_recovery() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../docs/workflow-bottom-blind-example.json")).unwrap();
    // Bottom cut remains in material below a moving opening, top cut crosses it.
    value["operations"][0]["outer"] = json!([[-40, -30], [40, -30], [40, 30], [-40, 30]]);
    value["operations"][0]["holes"] = json!([[[-2, -2], [2, -2], [2, 2], [-2, 2]]]);
    value["operations"][0]["offset"] = json!([20, 0]);
    value["operations"][1]["center"] = json!([10, 0]);
    value["operations"][1]["radius"] = json!(1);
    value["operations"][1]["depth"] = json!(4);
    let doc: WorkflowDocument = serde_json::from_value(value.clone()).unwrap();
    let mut session = WorkflowSession::new();
    session.rebuild(&doc).unwrap();
    value["operations"][1]["depth"] = json!(16);
    let failed: WorkflowDocument = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        session.rebuild(&failed).unwrap_err().code,
        "profile_hole_clearance"
    );
    assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
    value["operations"][1]["depth"] = json!(24);
    assert_eq!(
        serde_json::from_value::<WorkflowDocument>(value.clone())
            .unwrap()
            .rebuild()
            .unwrap_err()
            .code,
        "floor_thickness"
    );
    value["operations"][1]["mode"] = json!("through");
    value["operations"][1]
        .as_object_mut()
        .unwrap()
        .remove("depth");
    assert_eq!(
        serde_json::from_value::<WorkflowDocument>(value.clone())
            .unwrap()
            .rebuild()
            .unwrap_err()
            .code,
        "unexpected_entry"
    );
    value["operations"][1]["entry"] = json!("side");
    assert!(serde_json::from_value::<WorkflowDocument>(value).is_err());
    let old: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-skew-blind-example.json")).unwrap();
    assert!(serde_json::to_value(old).unwrap()["operations"][1]
        .get("entry")
        .is_none());
}
#[test]
fn mixed_box_top_bottom_and_through_nodes_preserve_all_cuts() {
    let value = json!({"schema_version":1,"units":"mm","tolerance":{"linear":1e-8,"angular":1e-10,"relative":1e-12},"operations":[
        {"kind":"box","id":"stock","size":[80,60,24]},
        {"kind":"bore","id":"lower","input":"stock","mode":"blind","entry":"bottom","center":[24,0],"radius":4,"depth":8},
        {"kind":"bore","id":"upper","input":"lower","mode":"blind","center":[-24,0],"radius":5,"depth":12},
        {"kind":"bore","id":"through","input":"upper","mode":"through","center":[0,0],"radius":3}
    ]});
    let doc: WorkflowDocument = serde_json::from_value(value).unwrap();
    let solid = doc.rebuild().unwrap();
    solid.validate(Tolerance::default()).unwrap();
    let expected = 80. * 60. * 24. - std::f64::consts::PI * (16. * 8. + 25. * 12. + 9. * 24.);
    assert!((solid.volume().unwrap() - expected).abs() < 1e-8);
    let policy = GeometryTolerance::new(1e-8, 1e-10, 0.).unwrap();
    for (p, location) in [
        (Point3::new(24., 0., -8.), PointLocation::Outside),
        (Point3::new(24., 0., 8.), PointLocation::Inside),
        (Point3::new(-24., 0., 8.), PointLocation::Outside),
        (Point3::new(-24., 0., -8.), PointLocation::Inside),
        (Point3::new(0., 0., 0.), PointLocation::Outside),
    ] {
        assert_eq!(
            classify_point_in_solid(&solid, p, policy).unwrap(),
            location
        );
    }
}
