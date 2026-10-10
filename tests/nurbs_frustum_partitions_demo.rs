use hagane::*;

fn payload() -> Vec<f64> {
    vec![16., 8., 24., 0., 0., 0., 0., 1e-6, 0.2, 6., 12., 18.]
}

#[test]
fn serializes_actual_partition_geometry() {
    let x = payload();
    let policy = GeometryTolerance::new(x[7], GeometryTolerance::default().angular(), 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [x[0], x[1]], x[2], policy).unwrap();
    let partition = source.split_axial_many(&x[9..], policy).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&nurbs_frustum_partitions_demo_json(&x).unwrap()).unwrap();
    assert_eq!(json["cut_heights"], serde_json::json!(x[9..]));
    assert_eq!(json["parts"].as_array().unwrap().len(), 4);
    for (body, part) in partition
        .parts
        .iter()
        .zip(json["parts"].as_array().unwrap())
    {
        assert_eq!(
            part["step"].as_str().unwrap(),
            body.export_step_mm(policy).unwrap()
        );
        assert_eq!(
            part["volume"].as_f64().unwrap(),
            body.volume(policy).unwrap()
        );
        assert!(!part["mesh"]["triangles"].as_array().unwrap().is_empty());
    }
    for (curves, section) in partition
        .sections
        .iter()
        .zip(json["sections"].as_array().unwrap())
    {
        for (curve, encoded) in curves.iter().zip(section["curves"].as_array().unwrap()) {
            let Curve::Nurbs(c) = curve else {
                panic!("canonical rational rim");
            };
            assert_eq!(encoded["weights"], serde_json::json!(c.weights()));
            assert_eq!(encoded["knots"], serde_json::json!(c.knots()));
            assert_eq!(
                encoded["control_points"],
                serde_json::json!(c
                    .control_points()
                    .iter()
                    .map(|p| [p.x, p.y, p.z])
                    .collect::<Vec<_>>())
            );
        }
    }
}

#[test]
fn finite_counts_cut_order_and_display_failures_recover() {
    let good = payload();
    for bad in [good[..9].to_vec(), vec![1.; 26]] {
        assert!(nurbs_frustum_partitions_demo_json(&bad).is_err());
    }
    for (index, value) in [(0, f64::NAN), (10, 6.), (9, 0.), (8, 0.), (8, 1e-30)] {
        let mut bad = good.clone();
        bad[index] = value;
        assert!(nurbs_frustum_partitions_demo_json(&bad).is_err());
    }
    let mut maximum = good[..9].to_vec();
    maximum.extend((1..=16).map(|i| i as f64));
    let value: serde_json::Value =
        serde_json::from_str(&nurbs_frustum_partitions_demo_json(&maximum).unwrap()).unwrap();
    assert_eq!(value["parts"].as_array().unwrap().len(), 17);
    assert!(nurbs_frustum_partitions_demo_json(&good).is_ok());
}
