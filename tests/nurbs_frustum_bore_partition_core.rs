use hagane::*;
#[test]
fn actual_annular_parts_and_inner_restrictions_preserve_volume() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let result = source.split_axial_many(&[6., 12., 18.], p).unwrap();
    assert_eq!(result.parts.len(), 4);
    let sum = result
        .parts
        .iter()
        .map(|v| v.volume(p).unwrap())
        .sum::<f64>();
    assert!((sum - source.volume(p).unwrap()).abs() < 1e-9);
    for child in &result.parts {
        assert_eq!(child.radius(), 4.);
        assert_eq!(
            (
                child.solid().vertices.len(),
                child.solid().edges.len(),
                child.solid().shell.faces.len()
            ),
            (16, 24, 10)
        );
        assert_eq!(child.solid().shell.faces[0].wires.len(), 2);
        assert_eq!(child.solid().shell.faces[1].wires.len(), 2);
        let center =
            child
                .source()
                .frame()
                .point(Point3::new(0., 0., child.source().height() / 2.));
        assert_eq!(
            child.classify_point(center, p).unwrap(),
            PointLocation::Outside
        );
        child.export_step_mm(p).unwrap();
    }
    for pair in result.parts.windows(2) {
        for q in 0..4 {
            let a = &pair[0].solid().edges[16 + q].curve;
            let b = &pair[1].solid().edges[12 + q].curve;
            for t in [0., 0.3, 1.] {
                assert!((a.try_evaluate(t).unwrap() - b.try_evaluate(t).unwrap()).norm() < 1e-12);
            }
        }
    }
}
#[test]
fn workflow_select_then_repartition_keeps_the_opening_and_rollback() {
    let d:NurbsFrustumWorkflowDocument=serde_json::from_value(serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"again","display_chord_tolerance":0.5,"operations":[{"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},{"kind":"coaxial_bore","id":"hole","input":"stock","radius":4.},{"kind":"partition","id":"parts","input":"hole","cuts":[6.,12.]},{"kind":"select","id":"pick","input":"parts","component":1},{"kind":"partition","id":"again","input":"pick","cuts":[3.]}]})).unwrap();
    let mut s = NurbsFrustumWorkflowSession::new();
    let report = s.rebuild_with(&d, |_, _| Ok(())).unwrap().0;
    assert_eq!(report.shape.components().len(), 2);
    assert!(report
        .shape
        .components()
        .iter()
        .all(|v| v.bored().unwrap().radius() == 4.));
    let mut bad = d.clone();
    bad.output = "parts".into();
    bad.operations[4] = NurbsFrustumWorkflowOperation::Partition {
        id: "again".into(),
        input: "parts".into(),
        cuts: vec![3.],
    };
    assert_eq!(
        s.rebuild_with(&bad, |_, _| Ok(())).unwrap_err().code,
        "ambiguous_components"
    );
    assert_eq!(s.accepted_document(), Some(&d));
}
