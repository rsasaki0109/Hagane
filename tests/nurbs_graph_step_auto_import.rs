use hagane::*;
fn fixtures() -> (NurbsGraphSolid, NurbsGraphHoledSolid) {
    let tol = Tolerance::default();
    let plain = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let holed = NurbsGraphHoledSolid::new(&plain, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
    (plain, holed)
}
#[test]
fn structural_auto_import_retains_plain_and_holed_bodies_with_common_queries() {
    let (plain, holed) = fixtures();
    let tol = Tolerance::default();
    for (hole, step, volume) in [
        (
            false,
            plain.export_step_mm(tol).unwrap(),
            plain.volume().unwrap(),
        ),
        (
            true,
            holed.export_step_mm(tol).unwrap(),
            holed.volume().unwrap(),
        ),
    ] {
        let imported = import_step_nurbs_graph_auto_mm(&step, tol).unwrap();
        assert_eq!(matches!(&imported, ImportedNurbsGraph::Holed(_)), hole);
        assert_eq!(matches!(&imported, ImportedNurbsGraph::Plain(_)), !hole);
        imported.validate(tol).unwrap();
        assert_eq!(imported.export_step_mm(tol).unwrap(), step);
        assert_eq!(imported.volume().unwrap(), volume);
        let bounds = imported.bounds().unwrap();
        assert_eq!(bounds.min, Point3::new(0., 0., 0.));
        assert_eq!(bounds.max.x, 80.);
        assert_eq!(bounds.max.y, 60.);
        let brep = imported.brep();
        let euler = brep.vertices.len() as isize - brep.edges.len() as isize
            + brep
                .shell
                .faces
                .iter()
                .map(|face| 2 - face.wires.len() as isize)
                .sum::<isize>();
        assert_eq!(euler, if hole { 0 } else { 2 });
        let t = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        assert_eq!(
            imported
                .classify_point(Point3::new(40., 30., 10.), t)
                .unwrap(),
            if hole {
                PointLocation::Outside
            } else {
                PointLocation::Inside
            }
        );
        assert_eq!(
            imported
                .classify_point(Point3::new(8., 6., 10.), t)
                .unwrap(),
            PointLocation::Inside
        );
        let section = imported.vertical_section([0.5, 0.5], t).unwrap();
        assert_eq!(section.intervals.len(), usize::from(!hole));
        if !hole {
            assert_eq!(section.intervals[0], [0., 27.5]);
        }
        let material = imported.vertical_section([0.1, 0.1], t).unwrap();
        assert_eq!(material.intervals.len(), 1);
        assert!((material.intervals[0][1] - 20.972).abs() < 1e-12);
        let display = imported.tessellate_bounded(0.2, 65536, tol).unwrap();
        assert!(display.mesh.signed_volume() > 0.);
        assert_eq!(display.error_bounds.len(), display.mesh.triangles.len());
    }
}
#[test]
fn auto_import_does_not_broaden_existing_typed_or_generic_importers() {
    let (plain, holed) = fixtures();
    let t = Tolerance::default();
    let plain = plain.export_step_mm(t).unwrap();
    let holed = holed.export_step_mm(t).unwrap();
    assert!(import_step_nurbs_graph_mm(&holed, t).is_err());
    assert!(import_step_nurbs_graph_holed_mm(&plain, t).is_err());
    assert!(import_step_mm(&plain, t).is_err());
    assert!(import_step_mm(&holed, t).is_err());
    assert!(import_step_nurbs_graph_auto_mm(&plain, t).is_ok());
    assert!(import_step_nurbs_graph_auto_mm(&holed, t).is_ok());
}
#[test]
fn auto_import_rejects_structural_corruption_and_unsupported_placement_or_trim() {
    let (plain, holed) = fixtures();
    let t = Tolerance::default();
    for step in [
        plain.export_step_mm(t).unwrap(),
        holed.export_step_mm(t).unwrap(),
    ] {
        let bad = step.replacen("CLOSED_SHELL('',(", "CLOSED_SHELL('',(#999999,", 1);
        assert!(import_step_nurbs_graph_auto_mm(&bad, t).is_err());
        let bad = step.replacen("DIRECTION('',(1.,0.))", "DIRECTION('',(0.,1.))", 1);
        assert_ne!(bad, step);
        assert!(import_step_nurbs_graph_auto_mm(&bad, t).is_err());
    }
    for unsupported in [
        plain.trimmed_uv([[0.2, 0.8], [0.2, 0.8]], t).unwrap(),
        plain
            .transformed(Transform::translation(Vec3::new(1., 2., 3.)).unwrap(), t)
            .unwrap(),
    ] {
        assert!(
            import_step_nurbs_graph_auto_mm(&unsupported.export_step_mm(t).unwrap(), t).is_err()
        );
    }
    assert!(import_step_nurbs_graph_auto_mm("not a STEP file", t).is_err());
}
