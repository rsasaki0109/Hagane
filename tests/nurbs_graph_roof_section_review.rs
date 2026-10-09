use hagane::*;
fn tol() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn check(s: &NurbsGraphSolid, r: &NurbsGraphRoofSection) {
    r.validate(tol()).unwrap();
    let [l, w, h] = s.dimensions();
    for span in &r.spans {
        assert_eq!(span.face, 1);
        assert_eq!(span.curve.degree(), 4);
        assert_eq!(span.curve.domain(), span.parameter_range);
        for i in 0..=100 {
            let t = span.parameter_range[0]
                + (span.parameter_range[1] - span.parameter_range[0]) * i as f64 / 100.;
            let uv = std::array::from_fn::<_, 2, _>(|a| r.start[a] + (r.end[a] - r.start[a]) * t);
            let actual_uv = span.pcurve.evaluate(t);
            for a in 0..2 {
                assert!((actual_uv[a] - uv[a]).abs() < 1e-14);
            }
            let p = s.placement().point(Point3::new(
                l * uv[0],
                w * uv[1],
                h + 4. * s.bulge() * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1]),
            ));
            assert!((span.curve.evaluate(t).unwrap() - p).norm() < 1e-10);
        }
    }
}
#[test]
fn original_global_parameters_dense_surface_geometry_and_reversal() {
    let tr = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    for b in [-2., 0., 20.] {
        let s = NurbsGraphSolid::new([20., 12., 3.], b, Tolerance::default())
            .unwrap()
            .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], Tolerance::default())
            .unwrap()
            .transformed(tr, Tolerance::default())
            .unwrap();
        let a = [0.2, 0.15];
        let z = [0.8, 0.65];
        let r = s.roof_section(a, z, tol()).unwrap();
        let rev = s.roof_section(z, a, tol()).unwrap();
        assert_eq!(r.spans.len(), 1);
        check(&s, &r);
        check(&s, &rev);
        for i in 0..=20 {
            let t = i as f64 / 20.;
            assert!(
                (r.spans[0].curve.evaluate(t).unwrap()
                    - rev.spans[0].curve.evaluate(1. - t).unwrap())
                .norm()
                    < 1e-10
            );
        }
    }
}
#[test]
fn hole_clipping_preserves_global_gaps_and_empty_or_single_material_pieces() {
    let s = NurbsGraphSolid::new([20., 12., 3.], 20., Tolerance::default()).unwrap();
    let h =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.65], [0.3, 0.7]], Tolerance::default()).unwrap();
    let r = h.roof_section([0.1, 0.45], [0.9, 0.55], tol()).unwrap();
    assert_eq!(r.spans.len(), 2);
    check(&s, &r);
    let ranges = [[0., 0.3125], [0.6875, 1.]];
    for (span, range) in r.spans.iter().zip(ranges) {
        for (a, expected) in range.iter().enumerate() {
            assert!((span.parameter_range[a] - expected).abs() < 1e-14);
        }
        for i in 0..=100 {
            let t = range[0] + (range[1] - range[0]) * i as f64 / 100.;
            let uv = span.pcurve.evaluate(t);
            assert!(!(uv[0] > 0.35 + 1e-14 && uv[0] < 0.65 - 1e-14 && uv[1] > 0.3 && uv[1] < 0.7));
        }
    }
    let empty = h.roof_section([0.4, 0.4], [0.6, 0.6], tol()).unwrap();
    assert!(empty.spans.is_empty());
    empty.validate(tol()).unwrap();
    let single = h.roof_section([0.5, 0.5], [0.9, 0.55], tol()).unwrap();
    assert_eq!(single.spans.len(), 1);
    assert!((single.spans[0].parameter_range[0] - 0.375).abs() < 1e-14);
    check(&s, &single);
}
#[test]
fn contact_invalid_and_mutated_public_certificates_reject() {
    let s = NurbsGraphSolid::new([20., 12., 3.], 20., Tolerance::default()).unwrap();
    let h =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.65], [0.3, 0.7]], Tolerance::default()).unwrap();
    for (a, b) in [
        ([f64::NAN, 0.5], [0.9, 0.5]),
        ([-0.1, 0.5], [0.9, 0.5]),
        ([0.5, 0.5], [0.5, 0.5]),
        ([0., 0.], [1., 0.]),
    ] {
        assert!(s.roof_section(a, b, tol()).is_err());
    }
    for (a, b) in [
        ([0.1, 0.3], [0.9, 0.3]),
        ([0.1, 0.05], [0.9, 0.85]),
        ([0.35, 0.5], [0.9, 0.5]),
        ([0.35 - 1e-8, 0.5], [0.9, 0.5]),
    ] {
        assert!(h.roof_section(a, b, tol()).is_err());
    }
    let original = h.roof_section([0.1, 0.45], [0.9, 0.55], tol()).unwrap();
    let mut r = original.clone();
    r.start[0] += 1e-12;
    assert!(r.validate(tol()).is_err());
    let mut r = original.clone();
    r.spans[0].parameter_range[1] += 1e-12;
    assert!(r.validate(tol()).is_err());
    let mut r = original.clone();
    r.spans[0].face = 0;
    assert!(r.validate(tol()).is_err());
    let mut r = original;
    r.spans[0].pcurve = PCurve::Affine {
        origin: [0., 0.],
        direction: [1., 1.],
    };
    assert!(r.validate(tol()).is_err());
    let mut broken = s;
    broken.solid.vertices[0].point.x += 0.01;
    assert!(broken.roof_section([0.1, 0.1], [0.9, 0.9], tol()).is_err());
}
