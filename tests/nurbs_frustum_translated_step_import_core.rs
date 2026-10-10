use hagane::*;

#[test]
fn actual_translated_coefficients_and_upper_partition_roundtrip() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let frame = Frame3::new(
        Point3::new(13., -27., 1000.),
        Frame3::IDENTITY.axes(),
        p.absolute(),
    )
    .unwrap();
    for h in [24., 0.1] {
        let source = NurbsFrustumSolid::new(frame, [16., 8.], h, p).unwrap();
        let text = source.export_step_mm(p).unwrap();
        let actual = import_step_nurbs_frustum_translated_mm(&text, p).unwrap();
        assert_eq!(actual.export_step_mm(p).unwrap(), text);
        assert_eq!(
            format!("{:?}", actual.solid()),
            format!("{:?}", source.solid())
        );
        assert!(import_step_nurbs_frustum_mm(&text, p).is_err());
    }
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let upper = source.split_axial(6., p).unwrap().upper;
    let text = upper.export_step_mm(p).unwrap();
    assert_eq!(
        import_step_nurbs_frustum_translated_mm(&text, p)
            .unwrap()
            .export_step_mm(p)
            .unwrap(),
        text
    );
}

#[test]
fn translated_reader_rejects_rotated_actual_body() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let frame = Frame3::new(
        Point3::new(1., 2., 3.),
        [
            Vec3::new(0., 1., 0.),
            Vec3::new(-1., 0., 0.),
            Vec3::new(0., 0., 1.),
        ],
        p.absolute(),
    )
    .unwrap();
    let source = NurbsFrustumSolid::new(frame, [16., 8.], 24., p).unwrap();
    assert!(
        import_step_nurbs_frustum_translated_mm(&source.export_step_mm(p).unwrap(), p).is_err()
    );
}
