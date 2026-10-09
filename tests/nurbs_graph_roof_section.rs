use hagane::*;
fn check(s: &NurbsGraphSolid, section: &NurbsGraphRoofSection) {
    section.validate(GeometryTolerance::default()).unwrap();
    for span in &section.spans {
        assert_eq!(span.curve.domain(), span.parameter_range);
        for i in 0..101 {
            let t = span.parameter_range[0]
                + (span.parameter_range[1] - span.parameter_range[0]) * i as f64 / 100.;
            let uv = span.pcurve.evaluate(t);
            let [l, w, h] = s.dimensions();
            let z = h + 4. * s.bulge() * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1]);
            let want = s.placement().point(Point3::new(l * uv[0], w * uv[1], z));
            assert!((span.curve.evaluate(t).unwrap() - want).norm() < 1e-10);
        }
    }
}
#[test]
fn diagonal_degree_four_and_reverse() {
    let s = NurbsGraphSolid::new([80., 60., 20.], 30., Tolerance::default()).unwrap();
    for (a, b) in [([0.1, 0.2], [0.9, 0.8]), ([0.9, 0.8], [0.1, 0.2])] {
        let section = s.roof_section(a, b, GeometryTolerance::default()).unwrap();
        assert_eq!(section.spans.len(), 1);
        check(&s, &section);
    }
}
#[test]
fn hole_global_ranges_and_public_corruption() {
    let s = NurbsGraphSolid::new([80., 60., 20.], 30., Tolerance::default()).unwrap();
    let h =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.65], [0.35, 0.65]], Tolerance::default()).unwrap();
    let mut section = h
        .roof_section([0.1, 0.4], [0.9, 0.6], GeometryTolerance::default())
        .unwrap();
    assert_eq!(section.spans.len(), 2);
    check(&s, &section);
    assert!(section.spans[0].parameter_range[1] < section.spans[1].parameter_range[0]);
    section.spans[0].face = 0;
    assert!(section.validate(GeometryTolerance::default()).is_err());
    assert!(h
        .roof_section([0.4, 0.4], [0.6, 0.6], GeometryTolerance::default())
        .unwrap()
        .spans
        .is_empty());
    assert!(h
        .roof_section([0.1, 0.1], [0.9, 0.9], GeometryTolerance::default())
        .is_err());
}
#[test]
fn trimmed_signed_placed_and_invalid() {
    let tol = Tolerance::default();
    let s = NurbsGraphSolid::new([8., 6., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.8], [0.2, 0.9]], tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(),
            tol,
        )
        .unwrap();
    check(
        &s,
        &s.roof_section([0.2, 0.3], [0.7, 0.8], GeometryTolerance::default())
            .unwrap(),
    );
    for (a, b) in [
        ([0.2, 0.3], [0.2, 0.3]),
        ([f64::NAN, 0.3], [0.7, 0.8]),
        ([0., 0.3], [0.7, 0.8]),
    ] {
        assert!(s.roof_section(a, b, GeometryTolerance::default()).is_err());
    }
}
