use hagane::*;
use serde_json::json;
use std::sync::Arc;

fn document() -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"stock","display_chord_tolerance":0.2,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-3.,5.],"axes":[[0.,1.,0.],[0.,0.,1.],[1.,0.,0.]]},{"kind":"coaxial_bore","id":"hole","input":"stock","radius":4.},{"kind":"plane_split","id":"cut","input":"stock","origin":[24.,-3.,5.],"normal":[1.,0.1,0.]}]})).unwrap()
}
fn world(r: f64, z: f64) -> Point3 {
    Point3::new(12. + z, -3. + r, 5.)
}
fn assert_read_only(session: &NurbsFrustumWorkflowSession, points: &[Point3]) {
    let doc = session.accepted_document().unwrap().clone();
    let shape = session.accepted_shape().unwrap().clone();
    let prefixes: Vec<_> = (0..doc.operations.len())
        .map(|i| session.prefix_snapshot(i).unwrap().clone())
        .collect();
    let history = (session.undo_count(), session.redo_count());
    let policy = GeometryTolerance::new(
        doc.tolerance.linear,
        doc.tolerance.angular,
        doc.tolerance.relative,
    )
    .unwrap();
    let steps: Vec<_> = shape
        .components()
        .iter()
        .map(|b| b.export_step_mm(policy).unwrap())
        .collect();
    for (i, body) in shape.components().iter().enumerate() {
        for &point in points {
            let expected = body.classify_point(point, policy);
            let actual = session.classify_point(i, point);
            match expected {
                Ok(location) => assert_eq!(actual.unwrap(), location),
                Err(_) => assert_eq!(actual.unwrap_err().code, "geometry_rejected"),
            }
        }
    }
    assert_eq!(
        session
            .classify_point(usize::MAX, world(5., 12.))
            .unwrap_err()
            .code,
        "invalid_component"
    );
    assert_eq!(
        session
            .classify_point(0, Point3::new(f64::NAN, 0., 0.))
            .unwrap_err()
            .code,
        "invalid_query"
    );
    assert!(session
        .classify_point(0, Point3::new(1e12, 0., 0.))
        .is_err());
    assert_eq!(session.accepted_document(), Some(&doc));
    assert!(Arc::ptr_eq(&shape, session.accepted_shape().unwrap()));
    for (i, prefix) in prefixes.iter().enumerate() {
        assert!(Arc::ptr_eq(prefix, session.prefix_snapshot(i).unwrap()));
    }
    assert_eq!(history, (session.undo_count(), session.redo_count()));
    assert_eq!(
        steps,
        shape
            .components()
            .iter()
            .map(|b| b.export_step_mm(policy).unwrap())
            .collect::<Vec<_>>()
    );
}
#[test]
fn actual_typed_outputs_query_without_replay_and_preserve_undo_redo() {
    let mut session = NurbsFrustumWorkflowSession::new();
    assert_eq!(
        session.classify_point(0, world(0., 0.)).unwrap_err().code,
        "no_accepted_document"
    );
    let mut doc = document();
    session.rebuild(&doc).unwrap();
    assert_eq!(
        session.classify_point(0, world(0., 12.)).unwrap(),
        PointLocation::Inside
    );
    assert_read_only(
        &session,
        &[world(0., 12.), world(12., 12.), world(20., 12.)],
    );
    doc.output = "hole".into();
    let update = session.rebuild(&doc).unwrap();
    assert_eq!(update.stats.evaluated_nodes, 0);
    assert_eq!(
        session.classify_point(0, world(0., 12.)).unwrap(),
        PointLocation::Outside
    );
    assert_read_only(&session, &[world(0., 12.), world(4., 12.), world(5., 12.)]);
    doc.output = "cut".into();
    session.rebuild(&doc).unwrap();
    assert_eq!(session.accepted_shape().unwrap().components().len(), 2);
    assert_read_only(
        &session,
        &[
            world(0., 8.),
            world(0., 12.),
            world(0., 16.),
            world(20., 12.),
        ],
    );
    session.undo().unwrap();
    assert_eq!(session.accepted_document().unwrap().output, "hole");
    assert_read_only(&session, &[world(0., 12.), world(5., 12.)]);
    session.redo().unwrap();
    assert_eq!(session.accepted_document().unwrap().output, "cut");
    assert_read_only(&session, &[world(0., 8.), world(0., 16.)]);
}
#[test]
fn finite_annular_corner_distance_and_accepted_relative_policy_are_not_naive_cap_tests() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let mut doc = document();
    doc.output = "hole".into();
    doc.tolerance.relative = 0.001;
    session.rebuild(&doc).unwrap();
    let band = 0.001 * 32_f64.hypot(24.);
    // Both coordinate gaps are below the band, but their Euclidean combination is above it.
    assert_eq!(
        session
            .classify_point(0, world(4. - 0.6 * band, -0.6 * band))
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        session
            .classify_point(0, world(4. - 0.8 * band, -0.8 * band))
            .unwrap(),
        PointLocation::Outside
    );
    let point = world(4. - band, 12.);
    let error = session.classify_point(0, point).unwrap_err();
    assert_eq!(error.code, "geometry_rejected");
    assert_eq!(error.operation_id.as_deref(), Some("hole"));
    assert_read_only(&session, &[point, world(4. - 0.8 * band, -0.8 * band)]);
}
