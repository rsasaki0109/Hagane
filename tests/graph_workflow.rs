use hagane::*;
fn document() -> GraphWorkflowDocument {
    serde_json::from_str(include_str!("../docs/graph-workflow.json")).unwrap()
}
#[test]
fn exact_replay_and_json_roundtrip_retain_typed_circle_brep() {
    let doc = document();
    let shape = doc.rebuild().unwrap();
    let tol = doc.geometry_tol().unwrap();
    let expected = NurbsGraphSolid::new([80., 60., 20.], 36., tol.absolute())
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol.absolute())
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(12., -5., 8.))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(0., 1., 0.), 25_f64.to_radians()).unwrap())
                .unwrap(),
            tol.absolute(),
        )
        .unwrap()
        .through_xy_circle([40., 30.], 10., tol.absolute())
        .unwrap();
    assert!(matches!(shape, GraphWorkflowShape::Circular(_)));
    assert_eq!(
        shape.export_step_mm(tol.absolute()).unwrap(),
        expected.export_step_mm(tol.absolute()).unwrap()
    );
    let i = 0.8 * (0.25 - 0.8_f64.powi(2) / 12.);
    let stock = 80. * 60. * (20. * 0.8 * 0.8 + 144. * i * i);
    let disk = std::f64::consts::PI * 100. * (29. - 100. * 0.03125 / 8. + 10000. * 0.00005 / 192.);
    assert!((shape.volume().unwrap() - (stock - disk)).abs() < 1e-8);
    let text = serde_json::to_string(&doc).unwrap();
    let restored: GraphWorkflowDocument = serde_json::from_str(&text).unwrap();
    assert_eq!(restored, doc);
    assert_eq!(
        restored
            .rebuild()
            .unwrap()
            .export_step_mm(tol.absolute())
            .unwrap(),
        shape.export_step_mm(tol.absolute()).unwrap()
    );
    for face in &shape.brep().shell.faces[..2] {
        for c in &face.wires[1].coedges {
            assert!(matches!(c.pcurve, PCurve::Nurbs(_)));
        }
    }
}
#[test]
fn typed_display_query_metrics_and_step_dispatch_use_retained_shape() {
    let doc = document();
    let shape = doc.rebuild().unwrap();
    let tol = doc.geometry_tol().unwrap();
    let query_tol = GeometryTolerance::new(1e-6, tol.angular(), tol.relative()).unwrap();
    let display = shape
        .tessellate_bounded(
            doc.display.max_error,
            doc.display.max_triangles,
            tol.absolute(),
        )
        .unwrap();
    assert!(display.mesh.triangles.len() <= doc.display.max_triangles);
    assert!(display
        .error_bounds
        .iter()
        .all(|e| *e <= doc.display.max_error));
    assert_eq!(
        shape
            .classify_point(
                shape.source().placement().point(Point3::new(20., 15., 1.)),
                query_tol
            )
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        shape
            .classify_point(
                shape.source().placement().point(Point3::new(40., 30., 1.)),
                query_tol
            )
            .unwrap(),
        PointLocation::Outside
    );
    let mass = shape.mass_properties(tol.absolute()).unwrap();
    assert_eq!(mass.volume, shape.volume().unwrap());
    assert!(shape.inertia_properties(tol.absolute()).unwrap().inertia[0][0] > 0.);
    let mut plain = doc;
    plain.operations.truncate(1);
    let shape = plain.rebuild().unwrap();
    assert!(matches!(shape, GraphWorkflowShape::Plain(_)));
    let display = shape.tessellate_bounded(1., 33, tol.absolute());
    assert!(display.is_err());
    let display = shape.tessellate_bounded(60., 33, tol.absolute()).unwrap();
    assert!(display.mesh.triangles.len() <= 33);
}
#[test]
fn header_ids_history_unknown_fields_and_bad_geometry_are_rejected() {
    let baseline = document();
    let mut bad = baseline.clone();
    bad.schema_version = 2;
    assert_eq!(bad.rebuild().unwrap_err().code, "unsupported_schema");
    let mut bad = baseline.clone();
    bad.units = "m".into();
    assert!(bad.rebuild().is_err());
    let mut bad = baseline.clone();
    bad.display.max_error = f64::NAN;
    assert_eq!(bad.rebuild().unwrap_err().field, Some("display"));
    let mut bad = baseline.clone();
    bad.operations.push(bad.operations[1].clone());
    assert!(bad.rebuild().is_err());
    let mut bad = baseline.clone();
    if let GraphWorkflowOperation::Placement { input, .. } = &mut bad.operations[2] {
        *input = "stock".into();
    }
    assert_eq!(bad.rebuild().unwrap_err().code, "invalid_input_reference");
    let mut bad = baseline.clone();
    if let GraphWorkflowOperation::CircularThroughBore { radius, .. } = &mut bad.operations[3] {
        *radius = 30.;
    }
    let d = bad.rebuild().unwrap_err();
    assert_eq!(d.operation_id.as_deref(), Some(baseline.operations[3].id()));
    assert_eq!(d.code, "geometry_rejected");
    let mut json = serde_json::to_value(baseline).unwrap();
    json["operations"][3]["kind"] = "blind_bore".into();
    assert!(serde_json::from_value::<GraphWorkflowDocument>(json).is_err());
    let mut json = serde_json::to_value(document()).unwrap();
    json["cached_mesh"] = serde_json::json!({});
    assert!(serde_json::from_value::<GraphWorkflowDocument>(json).is_err());
}
#[test]
fn public_shape_corruption_cannot_pass_typed_dispatch() {
    let doc = document();
    let mut shape = doc.rebuild().unwrap();
    let GraphWorkflowShape::Circular(body) = &mut shape else {
        panic!()
    };
    body.solid.vertices[8].point.x = body.solid.vertices[8].point.x.next_up();
    let tol = doc.geometry_tol().unwrap();
    assert!(shape.validate(tol.absolute()).is_err());
    assert!(shape.export_step_mm(tol.absolute()).is_err());
    assert!(shape.mass_properties(tol.absolute()).is_err());
    assert!(shape
        .classify_point(Point3::new(-100., 0., 0.), tol)
        .is_err());
}
