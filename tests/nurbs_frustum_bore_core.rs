use hagane::*;
#[test]
fn actual_annulus_keeps_source_coefficients_and_cylinder_metrics() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [8., 8.], 20., p).unwrap();
    let bore = source.bore_coaxial(3., p).unwrap();
    assert_eq!(
        (
            bore.solid().vertices.len(),
            bore.solid().edges.len(),
            bore.solid().shell.faces.len()
        ),
        (16, 24, 10)
    );
    assert_eq!(
        format!("{:?}", &bore.solid().vertices[..8]),
        format!("{:?}", source.solid().vertices)
    );
    assert_eq!(
        format!("{:?}", &bore.solid().edges[..12]),
        format!("{:?}", source.solid().edges)
    );
    let volume = std::f64::consts::PI * 20. * 55.;
    let m = bore.inertia_properties(p).unwrap();
    assert!((m.volume - volume).abs() < 1e-9);
    assert!((m.centroid.z - 10.).abs() < 1e-12);
    assert!((m.inertia[2][2] - volume * 73. / 2.).abs() < 1e-8);
    assert!((m.inertia[0][0] - volume * (73. / 4. + 400. / 12.)).abs() < 1e-8);
    assert_eq!(bore.solid().shell.faces[0].wires.len(), 2);
    assert!(bore.solid().shell.faces[6..]
        .iter()
        .all(|f| f.orientation == -1));
    assert!(bore.export_step_mm(p).unwrap().contains("FACE_BOUND"));
    assert!(matches!(
        bore.solid().validate(p.absolute()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn unresolved_contact_rejects_and_taper_volume_is_conserved() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    for radius in [0., -1., 8., 9., 8. - 1e-6] {
        assert!(source.bore_coaxial(radius, p).is_err());
    }
    let bore = source.bore_coaxial(4., p).unwrap();
    assert!(
        (bore.volume(p).unwrap() + bore.tool().volume(p).unwrap() - source.volume(p).unwrap())
            .abs()
            < 1e-9
    );
    assert!(bore.mass_properties(p).unwrap().centroid.z < 12.);
}

#[test]
fn admitted_thin_annular_cylinder_uses_physical_gaps_before_scaling() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), rotation.axes(), p).unwrap();
    let source = NurbsFrustumSolid::new(frame, [16., 16.], 24., p).unwrap();
    let radius = 16. - 2e-5;
    let body = source.bore_coaxial(radius, p).unwrap();
    let m = body.inertia_properties(p).unwrap();
    assert_eq!(m.volume, body.volume(p).unwrap());
    assert_eq!(m.volume, body.mass_properties(p).unwrap().volume);
    let expected_volume = std::f64::consts::PI * 24. * (16. - radius) * (16. + radius);
    assert!((m.volume / expected_volume - 1.).abs() < 2e-15);
    let radial = (256. + radius * radius) / 4.;
    let diagonal = [radial + 48., radial + 48., 2. * radial];
    let axes = frame.axes();
    let rows = [
        [axes[0].x, axes[1].x, axes[2].x],
        [axes[0].y, axes[1].y, axes[2].y],
        [axes[0].z, axes[1].z, axes[2].z],
    ];
    for i in 0..3 {
        for j in 0..3 {
            let expected = expected_volume
                * (0..3)
                    .map(|k| rows[i][k] * diagonal[k] * rows[j][k])
                    .sum::<f64>();
            assert!((m.inertia[i][j] - expected).abs() < expected_volume * 512. * 2e-14);
        }
    }
    let expected = frame.point(Point3::new(0., 0., 12.));
    assert!((m.centroid - expected).norm() < 1e-13);
}
