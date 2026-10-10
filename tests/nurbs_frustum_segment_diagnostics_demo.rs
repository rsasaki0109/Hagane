use hagane::*;
use serde_json::Value;

fn values(start: [f64; 3], end: [f64; 3]) -> Vec<f64> {
    let mut values = vec![16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1];
    values.extend(start);
    values.extend(end);
    values
}

#[test]
fn actionable_segment_failure_reasons_reach_demo_without_changing_valid_reports() {
    let valid = values([-25., 3., 12.], [25., 3., 12.]);
    let before = nurbs_frustum_segment_demo_json(&valid).unwrap();
    let report: Value = serde_json::from_str(&before).unwrap();
    assert_eq!(report["segment"]["hits"].as_array().unwrap().len(), 2);
    for (start, end, phrase) in [
        ([12., 0., 12.], [25., 0., 12.], "endpoint"),
        ([-25., 12., 12.], [25., 12., 12.], "tangen"),
        ([20., 20., 0.], [21., 20., 0.], "cap-plane"),
        ([-1e14, 3., 12.], [1e14, 3., 12.], "precision"),
    ] {
        let error = nurbs_frustum_segment_demo_json(&values(start, end)).unwrap_err();
        let Error::Unsupported(reason) = error else {
            panic!("expected scoped Unsupported, got {error:?}");
        };
        assert!(
            reason.to_lowercase().contains(phrase),
            "expected {phrase:?}, got {reason:?}"
        );
        assert_eq!(nurbs_frustum_segment_demo_json(&valid).unwrap(), before);
    }
}
