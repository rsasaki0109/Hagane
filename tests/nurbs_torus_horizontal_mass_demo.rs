use hagane::*;
use serde_json::Value;
#[test]
fn child_metric_reports_match_actual_core_and_keep_accepted_pair_after_failure() {
    let mut session = NurbsTorusDemoSession::new();
    session
        .apply(&[16., 4., 0.37, 12., -5., 8., 1e-6, 0.1])
        .unwrap();
    let value: Value = serde_json::from_str(&session.split_horizontal(1., 0.1).unwrap()).unwrap();
    let pair = session.accepted_horizontal_split().unwrap();
    let pointer = pair as *const _;
    let policy = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    for (name, body) in [("lower", &pair.lower), ("upper", &pair.upper)] {
        let actual = body.inertia_properties(policy).unwrap();
        let m = &value[name]["mass_properties"];
        assert_eq!(m["ok"], true);
        assert_eq!(m["volume"], actual.volume);
        assert_eq!(
            m["centroid"],
            serde_json::json!([actual.centroid.x, actual.centroid.y, actual.centroid.z])
        );
        assert_eq!(m["inertia"], serde_json::json!(actual.inertia));
        assert_eq!(m["reference"], "centroid");
        assert_eq!(m["tensor_axes"], "world");
        assert_eq!(m["inertia_units"], "mm^5");
        assert_eq!(value[name]["step"], body.export_step_mm(policy).unwrap());
    }
    let expected_volume = 512. * std::f64::consts::PI.powi(2);
    let total = value["lower"]["mass_properties"]["volume"]
        .as_f64()
        .unwrap()
        + value["upper"]["mass_properties"]["volume"]
            .as_f64()
            .unwrap();
    assert!((total - expected_volume).abs() < 1e-8);
    for axis in 0..3 {
        let first = ["lower", "upper"]
            .iter()
            .map(|name| {
                value[name]["mass_properties"]["volume"].as_f64().unwrap()
                    * value[name]["mass_properties"]["centroid"][axis]
                        .as_f64()
                        .unwrap()
            })
            .sum::<f64>();
        assert!((first / total - [12., -5., 8.][axis]).abs() < 1e-10);
    }
    let frame = session.accepted_body().unwrap().frame();
    let axes = frame.axes();
    let diagonal = [
        expected_volume * (16_f64.powi(2) / 2. + 5. * 4_f64.powi(2) / 8.),
        expected_volume * (16_f64.powi(2) / 2. + 5. * 4_f64.powi(2) / 8.),
        expected_volume * (16_f64.powi(2) + 3. * 4_f64.powi(2) / 4.),
    ];
    for (i, _) in [12., -5., 8.].iter().enumerate() {
        for (j, _) in [12., -5., 8.].iter().enumerate() {
            let expected = diagonal
                .iter()
                .enumerate()
                .map(|(k, v)| {
                    let a = [axes[k].x, axes[k].y, axes[k].z];
                    v * a[i] * a[j]
                })
                .sum::<f64>();
            let actual = ["lower", "upper"]
                .iter()
                .map(|name| {
                    let m = &value[name]["mass_properties"];
                    let v = m["volume"].as_f64().unwrap();
                    let d: Vec<_> = (0..3)
                        .map(|k| m["centroid"][k].as_f64().unwrap() - [12., -5., 8.][k])
                        .collect();
                    m["inertia"][i][j].as_f64().unwrap()
                        + v * (if i == j {
                            d.iter().map(|x| x * x).sum::<f64>()
                        } else {
                            0.
                        } - d[i] * d[j])
                })
                .sum::<f64>();
            assert!((actual - expected).abs() <= 1e-9 * expected.abs().max(1.));
        }
    }
    assert!(session.split_horizontal(4., 0.1).is_err());
    assert!(session.split_horizontal(0., 1e-14).is_err());
    assert_eq!(
        session.accepted_horizontal_split().unwrap() as *const _,
        pointer
    );
    assert!(session
        .query_horizontal_point(true, Point3::new(12., -5., 8.))
        .unwrap()
        .contains("outside"));
    assert_eq!(
        serde_json::from_str::<Value>(&session.split_horizontal(1., 0.1).unwrap()).unwrap(),
        value
    );
}
