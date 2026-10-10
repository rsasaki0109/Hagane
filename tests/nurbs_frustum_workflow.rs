use hagane::*;
use std::sync::Arc;

fn value() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"parts","display_chord_tolerance":0.2,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-3.,5.],"axes":[[0.,1.,0.],[0.,0.,1.],[1.,0.,0.]]},{"kind":"partition","id":"parts","input":"stock","cuts":[6.,12.,18.]}]})
}
fn doc(v: serde_json::Value) -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(v).unwrap()
}
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn signature(shape: &NurbsFrustumWorkflowShape) -> Vec<String> {
    shape
        .components()
        .iter()
        .map(|body| body.export_step_mm(policy()).unwrap())
        .collect()
}

#[test]
fn actual_parts_fresh_replay_step_stock_and_prefix_identity() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let document = doc(value());
    let accepted = session.rebuild(&document).unwrap();
    assert_eq!(accepted.stats.evaluated_nodes, 2);
    assert_eq!(accepted.shape.components().len(), 4);
    assert_eq!(
        signature(&accepted.shape),
        signature(&document.rebuild().unwrap())
    );
    let stock = session.prefix_snapshot(0).unwrap().clone();
    let parent = stock.components()[0].clone();
    let direct = parent.split_axial_many(&[6., 12., 18.], policy()).unwrap();
    for (actual, expected) in accepted.shape.components().iter().zip(&direct.parts) {
        assert_eq!(
            format!("{:?}", actual.solid()),
            format!("{:?}", expected.solid())
        );
        assert_eq!(
            actual.mass_properties(policy()).unwrap().centroid,
            expected.mass_properties(policy()).unwrap().centroid
        );
        assert_eq!(
            actual.inertia_properties(policy()).unwrap().inertia,
            expected.inertia_properties(policy()).unwrap().inertia
        );
    }
    let mut sum_volume = 0.;
    let mut first_moment = Vec3::new(0., 0., 0.);
    for part in accepted.shape.components() {
        let mass = part.mass_properties(policy()).unwrap();
        sum_volume += mass.volume;
        first_moment = first_moment + mass.centroid * mass.volume;
    }
    let expected_volume = std::f64::consts::PI * 24. * (16. * 16. + 16. * 8. + 8. * 8.) / 3.;
    assert!((sum_volume - expected_volume).abs() < 1e-10 * expected_volume);
    assert!(
        (first_moment * (1. / sum_volume) - parent.mass_properties(policy()).unwrap().centroid)
            .norm()
            < 1e-10
    );
    for pair in accepted.shape.components().windows(2) {
        for i in 0..4 {
            let a = &pair[0].solid().edges[4 + i].curve;
            let b = &pair[1].solid().edges[i].curve;
            let (Curve::Nurbs(a), Curve::Nurbs(b)) = (a, b) else {
                panic!("actual rational seam");
            };
            assert_eq!(a.control_points(), b.control_points());
            assert_eq!(a.weights(), b.weights());
            assert_eq!(a.knots(), b.knots());
        }
        let Surface::Plane { u: a, v: b, .. } = pair[0].solid().shell.faces[1].surface else {
            panic!("actual section cap");
        };
        let Surface::Plane { u: c, v: d, .. } = pair[1].solid().shell.faces[0].surface else {
            panic!("actual section cap");
        };
        assert!(a.cross(b).dot(c.cross(d)) > 0.);
        assert_eq!(pair[0].solid().shell.faces[1].orientation, 1);
        assert_eq!(pair[1].solid().shell.faces[0].orientation, -1);
    }
    let mut selected = value();
    selected["operations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"select","id":"chosen","input":"parts","component":1}));
    selected["output"] = serde_json::json!("chosen");
    let selected_doc = doc(selected.clone());
    let chosen = session.rebuild(&selected_doc).unwrap();
    assert_eq!(
        (chosen.stats.reused_nodes, chosen.stats.evaluated_nodes),
        (2, 1)
    );
    assert!(Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &stock));
    assert!(Arc::ptr_eq(
        &chosen.shape.components()[0],
        &accepted.shape.components()[1]
    ));
    let mut further = selected.clone();
    further["operations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"partition","id":"refined","input":"chosen","cuts":[2.]}));
    further["output"] = serde_json::json!("refined");
    let further = doc(further);
    let refined = session.rebuild(&further).unwrap();
    assert_eq!(
        (refined.stats.reused_nodes, refined.stats.evaluated_nodes),
        (3, 1)
    );
    assert_eq!(
        signature(&refined.shape),
        signature(&further.rebuild().unwrap())
    );
    assert_eq!(refined.shape.components().len(), 2);
    selected["operations"][0] = serde_json::json!({"kind":"step_stock","id":"stock","step":parent.export_step_mm(policy()).unwrap()});
    let imported = doc(selected).rebuild().unwrap();
    assert_eq!(signature(&imported), signature(&chosen.shape));
}

#[test]
fn invalid_graph_and_display_are_atomic_and_tolerance_invalidates_cache() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let original = doc(value());
    let accepted = session.rebuild(&original).unwrap();
    let cached = session.prefix_snapshot(0).unwrap().clone();
    let mut invalids = Vec::new();
    let mut v = value();
    v["operations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"partition","id":"again","input":"parts","cuts":[1.]}));
    v["output"] = serde_json::json!("again");
    invalids.push(v);
    let mut v = value();
    v["operations"][1]["input"] = serde_json::json!("future");
    invalids.push(v);
    let mut v = value();
    v["operations"][1]["id"] = serde_json::json!("stock");
    invalids.push(v);
    let mut v = value();
    v["output"] = serde_json::json!("missing");
    invalids.push(v);
    let mut v = value();
    v["operations"][0]["axes"][0] = serde_json::json!([1., 1., 0.]);
    invalids.push(v);
    let mut v = value();
    v["operations"][1]["cuts"] = serde_json::json!([6., 6.]);
    invalids.push(v);
    let mut v = value();
    v["display_chord_tolerance"] = serde_json::json!(1e-30);
    invalids.push(v);
    let mut v = value();
    v["tolerance"]["relative"] = serde_json::json!(0.001);
    v["operations"][1]["cuts"] = serde_json::json!([0.1]);
    invalids.push(v);
    for v in invalids {
        assert!(session.rebuild(&doc(v)).is_err());
        assert_eq!(session.accepted_document(), Some(&original));
        assert!(Arc::ptr_eq(
            session.accepted_shape().unwrap(),
            &accepted.shape
        ));
        assert!(Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &cached));
        assert_eq!(session.undo_count(), 0);
    }
    let mut changed = value();
    changed["operations"][1]["cuts"] = serde_json::json!([8., 16.]);
    assert!(session
        .rebuild_with(&doc(changed), |_, _| Err::<(), _>(Error::Unsupported(
            "intentional serializer failure"
        )))
        .is_err());
    assert!(Arc::ptr_eq(
        session.accepted_shape().unwrap(),
        &accepted.shape
    ));
    let mut output = value();
    output["output"] = serde_json::json!("stock");
    output["display_chord_tolerance"] = serde_json::json!(0.3);
    let result = session.rebuild(&doc(output.clone())).unwrap();
    assert_eq!(
        (result.stats.reused_nodes, result.stats.evaluated_nodes),
        (2, 0)
    );
    output["tolerance"]["linear"] = serde_json::json!(2e-6);
    let rebuilt = session.rebuild(&doc(output)).unwrap();
    assert_eq!(
        (rebuilt.stats.reused_nodes, rebuilt.stats.evaluated_nodes),
        (0, 2)
    );
    assert!(!Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &cached));
}

#[test]
fn undo_redo_callback_rollback_and_bounded_history() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let first = doc(value());
    session.rebuild_with(&first, |_, _| Ok(())).unwrap();
    let mut next = value();
    next["operations"][1]["cuts"] = serde_json::json!([8., 16.]);
    let next = doc(next);
    let accepted = session.rebuild_with(&next, |_, _| Ok(())).unwrap().0;
    assert!(session
        .undo_with(|_, _| Err::<(), _>(Error::Unsupported("intentional serializer failure")))
        .is_err());
    assert_eq!((session.undo_count(), session.redo_count()), (1, 0));
    assert!(Arc::ptr_eq(
        session.accepted_shape().unwrap(),
        &accepted.shape
    ));
    session.undo_with(|_, _| Ok(())).unwrap();
    assert_eq!(session.accepted_document(), Some(&first));
    session.redo_with(|_, _| Ok(())).unwrap();
    assert_eq!(session.accepted_document(), Some(&next));
    session.undo_with(|_, _| Ok(())).unwrap();
    let mut branch = value();
    branch["output"] = serde_json::json!("stock");
    session.rebuild_with(&doc(branch), |_, _| Ok(())).unwrap();
    assert_eq!(session.redo_count(), 0);
    for i in 0..40 {
        let mut v = value();
        v["display_chord_tolerance"] = serde_json::json!(0.3 + i as f64 * 0.01);
        session.rebuild_with(&doc(v), |_, _| Ok(())).unwrap();
    }
    assert_eq!(session.undo_count(), 32);
}
