use hagane::*;

#[test]
fn four_rectangular_restrictions_conserve_volume_and_exact_section_geometry() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 12., tol).unwrap();
    let mut parts = Vec::new();
    for u in [[0., 0.37], [0.37, 1.]] {
        for v in [[0., 0.63], [0.63, 1.]] {
            let part = source.trimmed_uv([u, v], tol).unwrap();
            part.validate(tol).unwrap();
            parts.push(part);
        }
    }
    let total: f64 = parts.iter().map(|p| p.volume().unwrap()).sum();
    assert!((total - source.volume().unwrap()).abs() < 1e-12);
    // The first and third parts meet at the same curved roof section.
    let section = |part: &NurbsGraphSolid| {
        part.solid
            .edges
            .iter()
            .find(|edge| {
                let range = edge.curve.range();
                let a = edge.curve.try_evaluate(range[0]).unwrap();
                let b = edge.curve.try_evaluate(range[1]).unwrap();
                let m = edge.curve.try_evaluate((range[0] + range[1]) / 2.).unwrap();
                (a.x - 8. * 0.37).abs() < 1e-12
                    && (b.x - a.x).abs() < 1e-12
                    && (a.y - b.y).abs() > 1.
                    && m.z > 2.
            })
            .unwrap()
            .curve
            .clone()
    };
    let left = section(&parts[0]);
    let right = section(&parts[2]);
    assert_eq!(left.range(), right.range());
    for i in 0..=20 {
        let v = if i == 20 { 0.63 } else { 0.63 * i as f64 / 20. };
        let exact = Point3::new(
            8. * 0.37,
            6. * v,
            2. + 48. * 0.37 * (1. - 0.37) * v * (1. - v),
        );
        assert!((left.try_evaluate(v).unwrap() - exact).norm() < 1e-12);
        assert!((right.try_evaluate(v).unwrap() - exact).norm() < 1e-12);
    }
}
