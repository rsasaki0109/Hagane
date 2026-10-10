use hagane::*;
use std::sync::Arc;
fn document() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"parts","display_chord_tolerance":0.5,"operations":[{"kind":"posed_frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-5.,8.],"rotation_axis":[1.,2.,3.],"angle":0.7},{"kind":"partition","id":"parts","input":"stock","cuts":[6.,12.]}]})).unwrap()
}
#[test]
fn actual_pose_and_partitions_match_direct_brep_and_world_metrics() {
    let doc = document();
    let policy = doc.geometry_tol().unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(12., -5., 8.), rotation.axes(), policy).unwrap();
    let direct = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy).unwrap();
    let parts = direct.split_axial_many(&[6., 12.], policy).unwrap();
    let actual = doc.rebuild().unwrap();
    for (a, b) in actual.components().iter().zip(&parts.parts) {
        assert_eq!(
            a.export_step_mm(policy).unwrap(),
            b.export_step_mm(policy).unwrap()
        );
        assert_eq!(
            a.mass_properties(policy).unwrap().centroid,
            b.mass_properties(policy).unwrap().centroid
        );
        assert_eq!(
            a.inertia_properties(policy).unwrap().inertia,
            b.inertia_properties(policy).unwrap().inertia
        );
    }
    let local = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy).unwrap();
    let m = local.mass_properties(policy).unwrap();
    let placed = direct.mass_properties(policy).unwrap();
    assert_eq!(m.volume, placed.volume);
    assert_eq!(placed.centroid, frame.point(m.centroid));
    assert!(
        import_step_nurbs_frustum_cardinal_mm(&direct.export_step_mm(policy).unwrap(), policy)
            .is_err()
    );
}
#[test]
fn pose_edit_invalidates_prefix_and_invalid_later_node_rolls_back_atomically() {
    let doc = document();
    let mut session = NurbsFrustumWorkflowSession::new();
    session.rebuild_with(&doc, |_, _| Ok(())).unwrap();
    let original = session.prefix_snapshot(0).unwrap().clone();
    let mut edit = doc.clone();
    if let NurbsFrustumWorkflowOperation::PosedFrustum { angle, .. } = &mut edit.operations[0] {
        *angle = -0.4;
    }
    let r = session.rebuild_with(&edit, |_, _| Ok(())).unwrap().0;
    assert_eq!(r.stats.reused_nodes, 0);
    assert_eq!(r.stats.evaluated_nodes, 2);
    assert!(!Arc::ptr_eq(&original, session.prefix_snapshot(0).unwrap()));
    let accepted = session.accepted_shape().unwrap().clone();
    let history = session.undo_count();
    let mut bad = edit.clone();
    bad.operations
        .push(NurbsFrustumWorkflowOperation::PosedFrustum {
            id: "invalid".into(),
            radii: [1., 1.],
            height: 2.,
            origin: [0.; 3],
            rotation_axis: [0.; 3],
            angle: 0.,
        });
    assert_eq!(
        session.rebuild_with(&bad, |_, _| Ok(())).unwrap_err().code,
        "invalid_posed_frustum"
    );
    assert_eq!(session.accepted_document(), Some(&edit));
    assert_eq!(history, session.undo_count());
    assert!(Arc::ptr_eq(&accepted, session.accepted_shape().unwrap()));
    assert!(session
        .rebuild_with(&doc, |_, _| Err::<(), _>(Error::Unsupported(
            "candidate display rejected"
        )))
        .is_err());
    assert_eq!(session.accepted_document(), Some(&edit));
}
#[test]
fn invalid_angles_axes_and_legacy_noncardinal_frames_are_refused() {
    for (axis, angle) in [
        ([0.; 3], 0.),
        ([1., 0., 0.], std::f64::consts::PI + 0.01),
        ([f64::INFINITY, 0., 0.], 0.),
        ([1., 0., 0.], f64::NAN),
    ] {
        let mut d = document();
        if let NurbsFrustumWorkflowOperation::PosedFrustum {
            rotation_axis,
            angle: a,
            ..
        } = &mut d.operations[0]
        {
            *rotation_axis = axis;
            *a = angle;
        }
        assert!(d.rebuild().is_err());
    }
    let mut d = document();
    d.operations[0] = NurbsFrustumWorkflowOperation::Frustum {
        id: "stock".into(),
        radii: [16., 8.],
        height: 24.,
        origin: [0.; 3],
        axes: [[0.8, 0.6, 0.], [-0.6, 0.8, 0.], [0., 0., 1.]],
    };
    assert!(d.rebuild().is_err());
}
