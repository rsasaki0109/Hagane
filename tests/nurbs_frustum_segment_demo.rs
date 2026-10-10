use hagane::*;
use serde_json::{json, Value};
fn model() -> Vec<f64> {
    vec![16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1]
}
fn query(mut x: Vec<f64>, start: Point3, end: Point3) -> Value {
    x.extend([start.x, start.y, start.z, end.x, end.y, end.z]);
    serde_json::from_str(&nurbs_frustum_segment_demo_json(&x).unwrap()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn verify_witnesses(report: &Value, body: &NurbsFrustumSolid, start: Point3, end: Point3) {
    for hit in report["segment"]["hits"].as_array().unwrap() {
        let t = hit["parameter"].as_f64().unwrap();
        let p = &hit["point"];
        let point = Point3::new(
            p[0].as_f64().unwrap(),
            p[1].as_f64().unwrap(),
            p[2].as_f64().unwrap(),
        );
        assert!((point - (start + (end - start) * t)).norm() < 1e-9);
        let faces = hit["faces"].as_array().unwrap();
        assert!(!faces.is_empty());
        for face in faces {
            let index = face["face_id"].as_u64().unwrap() as usize;
            let uv = &face["uv"];
            let actual = body.solid().shell.faces[index]
                .surface
                .try_evaluate(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap())
                .unwrap();
            assert!((actual - point).norm() < 1e-9);
        }
    }
}
#[test]
fn analytic_side_and_cap_hits_have_actual_face_witnesses_and_display_independence() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., tol).unwrap();
    let start = Point3::new(-25., 3., 12.);
    let end = Point3::new(25., 3., 12.);
    let report = query(model(), start, end);
    let hits = report["segment"]["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    let dx = (12f64.powi(2) - 3f64.powi(2)).sqrt();
    for (hit, expected) in hits.iter().zip([(25. - dx) / 50., (25. + dx) / 50.]) {
        close(hit["parameter"].as_f64().unwrap(), expected);
    }
    close(
        report["segment"]["material_interval"][0].as_f64().unwrap(),
        (25. - dx) / 50.,
    );
    close(
        report["segment"]["material_interval"][1].as_f64().unwrap(),
        (25. + dx) / 50.,
    );
    verify_witnesses(&report, &body, start, end);
    assert_eq!(report["step"], body.export_step_mm(tol).unwrap());
    let mut coarse = model();
    coarse[8] = 0.4;
    let coarse = query(coarse, start, end);
    assert_eq!(coarse["segment"], report["segment"]);
    assert!(
        coarse["mesh"]["triangles"].as_array().unwrap().len()
            < report["mesh"]["triangles"].as_array().unwrap().len()
    );
    let start = Point3::new(2., 3., -5.);
    let end = Point3::new(2., 3., 30.);
    let cap = query(model(), start, end);
    let hits = cap["segment"]["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    for (hit, (t, face)) in hits.iter().zip([(5. / 35., 0), (29. / 35., 1)]) {
        close(hit["parameter"].as_f64().unwrap(), t);
        assert_eq!(hit["faces"][0]["face_id"], face);
    }
    verify_witnesses(&cap, &body, start, end);
    let miss = query(
        model(),
        Point3::new(-25., 20., 12.),
        Point3::new(25., 20., 12.),
    );
    assert_eq!(miss["segment"]["hits"], json!([]));
    assert!(miss["segment"]["material_interval"].is_null());
}
#[test]
fn world_pose_is_retained_and_failed_queries_return_no_partial_report() {
    let mut x = model();
    x[3] = 0.3;
    x[4] = 12.;
    x[5] = -5.;
    x[6] = 8.;
    let frame = Transform::translation(Vec3::new(12., -5., 8.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.3).unwrap())
        .unwrap();
    let start = frame.point(Point3::new(-25., 3., 12.));
    let end = frame.point(Point3::new(25., 3., 12.));
    let report = query(x.clone(), start, end);
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    assert_eq!(
        report["placement"],
        json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)})
    );
    let body = NurbsFrustumSolid::new(
        frame,
        [16., 8.],
        24.,
        GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap(),
    )
    .unwrap();
    verify_witnesses(&report, &body, start, end);
    assert_eq!(report["segment"]["hits"].as_array().unwrap().len(), 2);
    for (start, end) in [
        (Point3::new(-25., 12., 12.), Point3::new(25., 12., 12.)),
        (Point3::new(-25., 3., 0.), Point3::new(25., 3., 0.)),
        (Point3::new(16., 0., 0.), Point3::new(25., 0., 0.)),
        (Point3::new(0., 0., 10.), Point3::new(0., 0., 10.)),
    ] {
        let mut values = model();
        values.extend([start.x, start.y, start.z, end.x, end.y, end.z]);
        assert!(nurbs_frustum_segment_demo_json(&values).is_err());
    }
    assert!(nurbs_frustum_segment_demo_json(&[]).is_err());
    let mut values = model();
    values.extend([-25., 3., 12., 25., 3., 12.]);
    values[9] = f64::NAN;
    assert!(nurbs_frustum_segment_demo_json(&values).is_err());
    assert_eq!(query(x, start, end), report);
}
