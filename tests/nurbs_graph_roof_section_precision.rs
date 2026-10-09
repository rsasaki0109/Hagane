use hagane::*;

#[test]
fn world_roundoff_is_separate_from_local_relative_tolerance() {
    let construction = Tolerance::new(1e-3).unwrap();
    let body = NurbsGraphSolid::new([80., 60., 20.], 30., construction)
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(1e6, -1e6, 1e6)).unwrap(),
            construction,
        )
        .unwrap();
    body.validate(construction).unwrap();
    for relative in [0., 1e-4] {
        let fine = GeometryTolerance::new(1e-6, 1e-10, relative).unwrap();
        let error = body.roof_section([0.1, 0.2], [0.9, 0.8], fine).unwrap_err();
        assert!(
            matches!(error, Error::InvalidInput(message) if message.contains("cannot resolve validation tolerance"))
        );
    }
    let resolved = GeometryTolerance::new(1e-3, 1e-10, 0.).unwrap();
    let section = body.roof_section([0.1, 0.2], [0.9, 0.8], resolved).unwrap();
    section.validate(resolved).unwrap();
    assert_eq!(section.spans.len(), 1);
    let Surface::Nurbs(roof) = &body.brep().shell.faces[1].surface else {
        panic!("expected actual retained NURBS roof");
    };
    for i in 0..=32 {
        let t = i as f64 / 32.;
        let uv = section.spans[0].pcurve.evaluate(t);
        assert!(
            (section.spans[0].curve.evaluate(t).unwrap() - roof.evaluate(uv[0], uv[1]).unwrap())
                .norm()
                < 1e-8
        );
    }
}
