use hagane::*;
use std::sync::Arc;
fn doc(suffix: serde_json::Value, output: &str) -> NurbsFrustumWorkflowDocument {
    let mut operations = vec![
        serde_json::json!({"kind":"frustum","id":"stock","radii":[12.,12.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}),
    ];
    if !suffix.is_null() {
        operations.push(suffix);
    }
    serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":output,"display_chord_tolerance":0.5,"operations":operations})).unwrap()
}
#[test]
fn accepted_output_dispatches_actual_material_without_replay() {
    for (d, queries) in [
        (
            doc(serde_json::Value::Null, "stock"),
            vec![(0, Point3::new(0., 0., 6.), PointLocation::Inside)],
        ),
        (
            doc(
                serde_json::json!({"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.1,0.,1.]}),
                "cut",
            ),
            vec![
                (0, Point3::new(0., 0., 6.), PointLocation::Inside),
                (1, Point3::new(0., 0., 6.), PointLocation::Outside),
                (0, Point3::new(0., 0., 12.), PointLocation::Boundary),
            ],
        ),
        (
            doc(
                serde_json::json!({"kind":"coaxial_bore","id":"hole","input":"stock","radius":4.}),
                "hole",
            ),
            vec![
                (0, Point3::new(0., 0., 6.), PointLocation::Outside),
                (0, Point3::new(4., 0., 6.), PointLocation::Boundary),
                (0, Point3::new(8., 0., 6.), PointLocation::Inside),
            ],
        ),
    ] {
        let mut s = NurbsFrustumWorkflowSession::new();
        s.rebuild_with(&d, |_, _| Ok(())).unwrap();
        let accepted = s.accepted_shape().unwrap().clone();
        let prefix = s.prefix_snapshot(0).unwrap().clone();
        let history = (s.undo_count(), s.redo_count());
        for (component, point, expected) in queries {
            assert_eq!(s.classify_point(component, point).unwrap(), expected);
        }
        assert!(Arc::ptr_eq(&accepted, s.accepted_shape().unwrap()));
        assert!(Arc::ptr_eq(&prefix, s.prefix_snapshot(0).unwrap()));
        assert_eq!(history, (s.undo_count(), s.redo_count()));
        assert_eq!(s.accepted_document(), Some(&d));
    }
}
#[test]
fn explicit_index_and_failed_queries_preserve_all_accepted_state() {
    let empty = NurbsFrustumWorkflowSession::new();
    assert_eq!(
        empty
            .classify_point(0, Point3::new(0., 0., 0.))
            .unwrap_err()
            .code,
        "no_accepted_document"
    );
    let d = doc(
        serde_json::json!({"kind":"plane_split","id":"cut","input":"stock","origin":[0.,0.,12.],"normal":[0.,0.,1.]}),
        "cut",
    );
    let mut s = NurbsFrustumWorkflowSession::new();
    s.rebuild_with(&d, |_, _| Ok(())).unwrap();
    let accepted = s.accepted_shape().unwrap().clone();
    assert_eq!(
        s.classify_point(2, Point3::new(0., 0., 6.))
            .unwrap_err()
            .code,
        "invalid_component"
    );
    assert_eq!(
        s.classify_point(0, Point3::new(f64::NAN, 0., 6.))
            .unwrap_err()
            .code,
        "invalid_query"
    );
    assert_eq!(
        s.classify_point(0, Point3::new(1e100, 0., 6.))
            .unwrap_err()
            .category,
        "unsupported"
    );
    assert_eq!(
        s.classify_point(0, Point3::new(12. + 0.8e-6, 0., 12. + 0.8e-6))
            .unwrap_err()
            .category,
        "unsupported"
    );
    assert!(Arc::ptr_eq(&accepted, s.accepted_shape().unwrap()));
    assert_eq!(s.undo_count(), 0);
    assert_eq!(s.redo_count(), 0);
    assert_eq!(
        s.classify_point(1, Point3::new(0., 0., 18.)).unwrap(),
        PointLocation::Inside
    );
}
