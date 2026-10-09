use serde_json::Value;
fn values() -> Vec<f64> {
    let mut result = vec![
        0.,
        -0.,
        1.,
        -1.,
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        f64::from_bits(1),
        -f64::from_bits(1),
        1e-200,
        1e200,
        1e-8,
        -1e-8,
        0.9999999999999999,
        1.0000000000000002,
    ];
    for i in 0..16 {
        let a = i as f64 * std::f64::consts::TAU / 16.;
        result.extend([
            0.5 + 0.4 * a.cos(),
            0.5 + 0.4 * a.sin(),
            a.cos() * 1e-12,
            a.sin() * 1e-12,
        ]);
    }
    let mut state = 0x37e2b90cfad12561u64;
    for _ in 0..2048 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let value = f64::from_bits(state);
        if value.is_finite() {
            result.push(value);
        }
    }
    result
}
#[test]
fn finite_ieee_values_preserve_bits_in_typed_and_value_json_roundtrips() {
    for original in values() {
        let text = serde_json::to_string(&original).unwrap();
        let standard = text.parse::<f64>().unwrap();
        assert_eq!(original.to_bits(), standard.to_bits(), "serializer {text}");
        let typed: f64 = serde_json::from_str(&text).unwrap();
        let dynamic: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            typed.to_bits(),
            standard.to_bits(),
            "typed decimal={text} old_bits={:016x} expected_bits={:016x}",
            typed.to_bits(),
            standard.to_bits()
        );
        assert_eq!(
            dynamic.as_f64().unwrap().to_bits(),
            standard.to_bits(),
            "Value decimal={text}"
        );
    }
}
#[test]
fn vectors_and_nested_user_input_preserve_decimal_bits_and_signed_zero() {
    let original = values();
    let text = serde_json::to_string(&original).unwrap();
    let typed: Vec<f64> = serde_json::from_str(&text).unwrap();
    let dynamic: Value = serde_json::from_str(&text).unwrap();
    for ((a, b), c) in original.iter().zip(typed).zip(dynamic.as_array().unwrap()) {
        assert_eq!(a.to_bits(), b.to_bits());
        assert_eq!(a.to_bits(), c.as_f64().unwrap().to_bits());
    }
    let nested = serde_json::json!({"polygon":original.chunks(2).map(|p|p.to_vec()).collect::<Vec<_>>(),"normal":[-0.,1e-200,-1e-12]});
    let text = serde_json::to_string(&nested).unwrap();
    let recovered: Value = serde_json::from_str(&text).unwrap();
    fn compare(a: &Value, b: &Value) {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                assert_eq!(a.as_f64().unwrap().to_bits(), b.as_f64().unwrap().to_bits())
            }
            (Value::Array(a), Value::Array(b)) => {
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b) {
                    compare(a, b);
                }
            }
            (Value::Object(a), Value::Object(b)) => {
                assert_eq!(a.len(), b.len());
                for (k, a) in a {
                    compare(a, &b[k]);
                }
            }
            _ => assert_eq!(a, b),
        }
    }
    compare(&nested, &recovered);
}
