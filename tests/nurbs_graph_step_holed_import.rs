use hagane::*;
fn roundtrip(h: f64, b: f64) -> NurbsGraphHoledSolid {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., h], b, tol).unwrap();
    let original = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
    let step = original.export_step_mm(tol).unwrap();
    let imported = import_step_nurbs_graph_holed_mm(&step, tol).unwrap();
    imported.validate(tol).unwrap();
    assert_eq!(imported.export_step_mm(tol).unwrap(), step);
    assert_eq!(imported.hole(), original.hole());
    assert!((imported.volume().unwrap() - original.volume().unwrap()).abs() < 1e-8);
    assert!(import_step_nurbs_graph_mm(&step, tol).is_err());
    imported
}
#[test]
fn exact_default_signed_small_and_flat_refined_caps_are_recognized() {
    for (h, b) in [(20., 30.), (3., -2.), (20., 1e-3), (20., 0.)] {
        let imported = roundtrip(h, b);
        assert_eq!(imported.brep().vertices.len(), 16);
        assert_eq!(imported.brep().edges.len(), 24);
        assert_eq!(imported.brep().shell.faces.len(), 10);
    }
}
#[test]
fn retained_actual_body_supports_material_queries_and_shared_closed_mesh() {
    let tol = Tolerance::default();
    let imported = roundtrip(20., 30.);
    let geom = GeometryTolerance::new(1e-4, 1e-10, 1e-12).unwrap();
    assert_eq!(
        imported
            .classify_point(Point3::new(20., 15., 10.), geom)
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        imported
            .classify_point(Point3::new(40., 30., 10.), geom)
            .unwrap(),
        PointLocation::Outside
    );
    assert!(imported
        .vertical_section([0.5, 0.5], geom)
        .unwrap()
        .intervals
        .is_empty());
    let mesh = imported.tessellate_bounded(0.1, 65536, tol).unwrap();
    assert!(mesh.mesh.signed_volume() > 0.);
}
#[test]
fn noncanonical_actual_edits_plain_placed_and_trimmed_are_explicitly_rejected() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
    let step = body.export_step_mm(tol).unwrap();
    let altered = step.replacen(
        "CARTESIAN_POINT('',(0.,0.,0.))",
        "CARTESIAN_POINT('',(0.000000000001,0.,0.))",
        1,
    );
    assert!(import_step_nurbs_graph_holed_mm(&altered, tol).is_err());
    assert!(import_step_nurbs_graph_holed_mm(&source.export_step_mm(tol).unwrap(), tol).is_err());
    for source in [
        source
            .transformed(Transform::translation(Vec3::new(1., 0., 0.)).unwrap(), tol)
            .unwrap(),
        source.trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol).unwrap(),
    ] {
        let body = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
        assert!(import_step_nurbs_graph_holed_mm(&body.export_step_mm(tol).unwrap(), tol).is_err());
    }
}
