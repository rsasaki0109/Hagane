use hagane::*;
use serde_json::{json, Value};
#[test]
fn opposing_coaxial_floors_retain_exact_web_volume_and_classification() {
    for scale in [1e-6, 1., 1000.] {
        let mut value: Value =
            serde_json::from_str(include_str!("../docs/workflow-opposing-blind-example.json"))
                .unwrap();
        value["tolerance"]["linear"] = json!(1e-8 * scale);
        fn scaled(v: &mut Value, s: f64) {
            if let Some(a) = v.as_array_mut() {
                for x in a {
                    if x.is_number() {
                        *x = json!(x.as_f64().unwrap() * s)
                    } else {
                        scaled(x, s)
                    }
                }
            }
        }
        for field in ["outer", "holes", "offset"] {
            scaled(&mut value["operations"][0][field], scale)
        }
        value["operations"][0]["height"] = json!(24. * scale);
        for i in [1, 2] {
            scaled(&mut value["operations"][i]["center"], scale);
            value["operations"][i]["radius"] = json!((if i == 1 { 4. } else { 6. }) * scale);
            value["operations"][i]["depth"] = json!((if i == 1 { 8. } else { 10. }) * scale);
        }
        let doc: WorkflowDocument = serde_json::from_value(value).unwrap();
        let solid = doc.rebuild().unwrap();
        let t = Tolerance::new(doc.tolerance.linear).unwrap();
        solid.validate(t).unwrap();
        let expected =
            (4408. * 24. - std::f64::consts::PI * (16. * 8. + 36. * 10.)) * scale.powi(3);
        assert!((solid.volume().unwrap() - expected).abs() < expected * 1e-12);
        assert_eq!(solid.shell.faces.len(), 18);
        let policy = GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap();
        for (z, location) in [
            (-8., PointLocation::Outside),
            (-4., PointLocation::Boundary),
            (0., PointLocation::Inside),
            (2., PointLocation::Boundary),
            (8., PointLocation::Outside),
        ] {
            assert_eq!(
                classify_point_in_solid(&solid, Point3::new(24., 0., z) * scale, policy).unwrap(),
                location
            );
        }
        let mesh = solid.tessellate(0.01 * scale, t).unwrap();
        assert!(
            (mesh.signed_volume() - expected).abs()
                < std::f64::consts::TAU * 0.01 * (4. * 8. + 6. * 10.) * scale.powi(3)
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
        assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
        let mut session = WorkflowSession::new();
        session.rebuild(&doc).unwrap();
        let mut edited = doc.clone();
        if let WorkflowOperation::Bore { depth, .. } = &mut edited.operations[2] {
            *depth = Some(9. * scale)
        }
        let result = session.rebuild(&edited).unwrap();
        assert_eq!(result.stats.reused_operations, 2);
        assert_eq!(
            result.solid.mesh_json(0.05 * scale, t).unwrap(),
            edited
                .rebuild()
                .unwrap()
                .mesh_json(0.05 * scale, t)
                .unwrap()
        );
    }
}
#[test]
fn opposing_contacts_intersections_and_same_entry_fail_without_cache_changes() {
    let doc: WorkflowDocument =
        serde_json::from_str(include_str!("../docs/workflow-opposing-blind-example.json")).unwrap();
    let mut session = WorkflowSession::new();
    session.rebuild(&doc).unwrap();
    for depth in [16., 16. - 5e-8, 17.] {
        let mut bad = doc.clone();
        if let WorkflowOperation::Bore { depth: d, .. } = &mut bad.operations[2] {
            *d = Some(depth)
        }
        let diagnostic = session.rebuild(&bad).unwrap_err();
        assert_eq!(diagnostic.code, "bore_web_thickness");
        assert_eq!(diagnostic.operation_id.as_deref(), Some("bore-2"));
        assert!(diagnostic.measured_clearance.unwrap() <= diagnostic.required_clearance.unwrap());
        assert_eq!(session.rebuild(&doc).unwrap().stats.rebuilt_operations, 0);
    }
    let mut same = doc.clone();
    if let WorkflowOperation::Bore { entry, .. } = &mut same.operations[2] {
        *entry = WorkflowBoreEntry::Bottom
    }
    assert_eq!(session.rebuild(&same).unwrap_err().code, "bore_clearance");
    // Reordering opposing cuts retains geometry and analytic volume.
    let mut reverse = doc.clone();
    reverse.operations.swap(1, 2);
    if let WorkflowOperation::Bore { input, .. } = &mut reverse.operations[1] {
        *input = "extrusion-1".into()
    }
    if let WorkflowOperation::Bore { input, .. } = &mut reverse.operations[2] {
        *input = "bore-2".into()
    }
    assert!(
        (reverse.rebuild().unwrap().volume().unwrap() - doc.rebuild().unwrap().volume().unwrap())
            .abs()
            < 1e-8
    );
}
#[test]
fn offset_footprints_and_radial_separation_use_either_valid_certificate() {
    let mut value: Value =
        serde_json::from_str(include_str!("../docs/workflow-opposing-blind-example.json")).unwrap();
    // Overlapping offset disks remain valid across a resolved axial web.
    value["operations"][2]["center"] = json!([26, 0]);
    let doc: WorkflowDocument = serde_json::from_value(value.clone()).unwrap();
    doc.rebuild().unwrap();
    // Separated disks may have overlapping depths or coplanar floors.
    value["operations"][2]["center"] = json!([0, 0]);
    value["operations"][2]["radius"] = json!(4);
    value["operations"][2]["depth"] = json!(17);
    serde_json::from_value::<WorkflowDocument>(value.clone())
        .unwrap()
        .rebuild()
        .unwrap();
    // Through tools overlapping a blind footprint remain explicitly unsupported.
    value["operations"][2]["center"] = json!([24, 0]);
    value["operations"][2]["mode"] = json!("through");
    value["operations"][2]
        .as_object_mut()
        .unwrap()
        .remove("depth");
    assert_eq!(
        serde_json::from_value::<WorkflowDocument>(value)
            .unwrap()
            .rebuild()
            .unwrap_err()
            .code,
        "bore_clearance"
    );
}
