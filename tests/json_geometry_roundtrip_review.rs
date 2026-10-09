use hagane::*;

fn polygon() -> Vec<[f64; 2]> {
    (0..16)
        .map(|i| {
            let a = std::f64::consts::TAU * (i as f64 + 0.01) / 16.;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect()
}
fn values() -> Vec<f64> {
    let mut values = vec![20., 12., 3., -2., 0.05, 0., 0., 0., 0., 0., 1., 0., 1.];
    values.extend(polygon().iter().flatten());
    values
}
fn typed() -> NurbsGraphPolygonSolid {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], -2., t).unwrap();
    NurbsGraphPolygonSolid::new(&source, polygon(), t).unwrap()
}

#[test]
fn serialized_editable_model_preserves_original_trigonometric_float_bits() {
    let original = values();
    let text = serde_json::to_string(&original).unwrap();
    let parsed: Vec<f64> = serde_json::from_str(&text).unwrap();
    for (i, (a, b)) in original.iter().zip(&parsed).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "model field {i}: {a:?} became {b:?}"
        );
    }
    let expected = typed().export_step_mm(Tolerance::default()).unwrap();
    let report: serde_json::Value =
        serde_json::from_str(&nurbs_graph_polygon_step_demo_json(&parsed).unwrap()).unwrap();
    assert_eq!(report["step"].as_str().unwrap(), expected);
}

#[test]
fn actual_display_polygon_and_centroid_reparse_as_original_binary64_geometry() {
    let body = typed();
    let text = nurbs_graph_polygon_numeric_demo_json(&values()).unwrap();
    let report: serde_json::Value = serde_json::from_str(&text).unwrap();
    let parsed: Vec<[f64; 2]> = serde_json::from_value(report["polygon"].clone()).unwrap();
    for (a, b) in body.polygon().iter().flatten().zip(parsed.iter().flatten()) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
    let centroid = body.mass_properties(Tolerance::default()).unwrap().centroid;
    let parsed: Vec<f64> =
        serde_json::from_value(report["mass_properties"]["centroid"].clone()).unwrap();
    for (a, b) in [centroid.x, centroid.y, centroid.z].iter().zip(parsed) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

#[test]
fn finite_serializer_outputs_reparse_against_independent_binary64_bits() {
    // This UV coordinate from the 16-corner fixture was rounded one ULP upward
    // by serde_json's default parser. The reference is independent of JSON.
    let decimal = "0.21827018629739742";
    let reference = decimal.parse::<f64>().unwrap();
    assert_eq!(reference.to_bits(), 0x3fcbf04707eb693e);
    let parsed: f64 = serde_json::from_str(decimal).unwrap();
    assert_eq!(parsed.to_bits(), reference.to_bits());
    let mut values = vec![
        0.,
        -0.,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
        -f64::MAX,
        0.1,
        std::f64::consts::PI,
    ];
    let mut state = 0x71dabe327919e515u64;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let value = f64::from_bits(state);
        if value.is_finite() {
            values.push(value);
        }
    }
    let text = serde_json::to_string(&values).unwrap();
    let parsed: Vec<f64> = serde_json::from_str(&text).unwrap();
    for (i, (a, b)) in values.iter().zip(parsed).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "finite sample {i}: {a:?} became {b:?}"
        );
    }
}
