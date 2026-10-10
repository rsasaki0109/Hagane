use hagane::*;
use serde_json::{json, Value};
fn values(origin: [f64; 3], direction: [f64; 3], mode: f64) -> Vec<f64> {
    let mut x = vec![16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1];
    x.extend(origin);
    x.extend(direction);
    x.push(mode);
    x
}
fn report(x: &[f64]) -> Value {
    serde_json::from_str(&nurbs_frustum_line_demo_json(x).unwrap()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9, "{a} != {b}");
}
#[test]
fn line_and_ray_parameters_witnesses_and_display_independence_use_actual_geometry() {
    let x = values([-25., 3., 12.], [1., 0., 0.], 0.);
    let line = report(&x);
    let hits = line["line"]["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    let root = (12f64.powi(2) - 3f64.powi(2)).sqrt();
    let body = NurbsFrustumSolid::new(
        Frame3::IDENTITY,
        [16., 8.],
        24.,
        GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap(),
    )
    .unwrap();
    for (hit, t) in hits.iter().zip([25. - root, 25. + root]) {
        close(hit["parameter"].as_f64().unwrap(), t);
        let p = &hit["point"];
        let p = Point3::new(
            p[0].as_f64().unwrap(),
            p[1].as_f64().unwrap(),
            p[2].as_f64().unwrap(),
        );
        assert!((p - Point3::new(-25. + t, 3., 12.)).norm() < 1e-9);
        for face in hit["faces"].as_array().unwrap() {
            let id = face["face_id"].as_u64().unwrap() as usize;
            let uv = &face["uv"];
            let actual = body.solid().shell.faces[id]
                .surface
                .try_evaluate(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap())
                .unwrap();
            assert!((actual - p).norm() < 1e-9);
        }
    }
    let doubled = report(&values([-25., 3., 12.], [2., 0., 0.], 0.));
    assert_eq!(doubled["line"]["direction"], json!([2., 0., 0.]));
    for (a, b) in doubled["line"]["hits"].as_array().unwrap().iter().zip(hits) {
        close(
            a["parameter"].as_f64().unwrap() * 2.,
            b["parameter"].as_f64().unwrap(),
        );
        assert_eq!(a["point"], b["point"]);
    }
    let ray = report(&values([-25., 3., 12.], [1., 0., 0.], 1.));
    assert_eq!(ray["line"]["hits"], line["line"]["hits"]);
    assert_eq!(ray["line"]["kind"], "ray");
    let inside = report(&values([0., 3., 12.], [1., 0., 0.], 1.));
    assert_eq!(inside["line"]["hits"].as_array().unwrap().len(), 1);
    assert_eq!(inside["line"]["material_interval"][0], 0.);
    close(
        inside["line"]["material_interval"][1].as_f64().unwrap(),
        root,
    );
    let behind = report(&values([25., 3., 12.], [1., 0., 0.], 1.));
    assert_eq!(behind["line"]["hits"], json!([]));
    assert!(behind["line"]["material_interval"].is_null());
    let mut coarse = x.clone();
    coarse[8] = 0.4;
    let coarse = report(&coarse);
    assert_eq!(coarse["line"], line["line"]);
    assert_eq!(coarse["step"], line["step"]);
}
#[test]
fn strict_mode_invalid_query_and_display_errors_return_no_partial_report() {
    let x = values([-25., 3., 12.], [1., 0., 0.], 0.);
    let original = nurbs_frustum_line_demo_json(&x).unwrap();
    for mode in [-1., 0.5, 2., f64::NAN, f64::INFINITY] {
        let mut bad = x.clone();
        bad[15] = mode;
        assert!(matches!(
            nurbs_frustum_line_demo_json(&bad),
            Err(Error::InvalidInput(_))
        ));
    }
    assert!(nurbs_frustum_line_demo_json(&x[..15]).is_err());
    assert!(nurbs_frustum_line_demo_json(&values([-25., 3., 12.], [0., 0., 0.], 0.)).is_err());
    assert!(nurbs_frustum_line_demo_json(&values([-25., 12., 12.], [1., 0., 0.], 0.)).is_err());
    let mut bad = x.clone();
    bad[9] = f64::NAN;
    assert!(nurbs_frustum_line_demo_json(&bad).is_err());
    let mut bad = x.clone();
    bad[8] = 1e-12;
    assert!(nurbs_frustum_line_demo_json(&bad).is_err());
    assert_eq!(nurbs_frustum_line_demo_json(&x).unwrap(), original);
}
