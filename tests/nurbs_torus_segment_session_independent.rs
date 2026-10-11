use hagane::*;
use serde_json::Value;
fn json(s: String) -> Value {
    serde_json::from_str(&s).unwrap()
}
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap()
}
#[test]
fn analytical_crossings_and_failed_requests_retain_accepted_geometry() {
    let mut session = NurbsTorusDemoSession::new();
    let start = Point3::new(-24., 0., 0.);
    let end = Point3::new(24., 0., 0.);
    assert!(session.intersect_segment(start, end).is_err());
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    let report = session.apply(&model).unwrap();
    let body = session.accepted_body().unwrap();
    let snapshot = format!("{:?}", body.solid());
    let step = body.export_step_mm(policy()).unwrap();
    let point_result = session.query_point(Point3::new(16., 0., 0.)).unwrap();
    let result = json(session.intersect_segment(start, end).unwrap());
    assert_eq!(result["mesh_free"], true);
    assert!(result.get("mesh").is_none());
    assert!(result.get("step").is_none());
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 4);
    for (hit, x) in hits.iter().zip([-20., -12., 12., 20.]) {
        assert!((hit["parameter"].as_f64().unwrap() - (x + 24.) / 48.).abs() < 1e-7);
        assert!((hit["point"][0].as_f64().unwrap() - x).abs() < 1e-6);
    }
    let intervals = result["material_intervals"].as_array().unwrap();
    assert_eq!(intervals.len(), 2);
    for (v, expected) in intervals.iter().zip([[1. / 12., 0.25], [0.75, 11. / 12.]]) {
        for k in 0..2 {
            assert!((v[k].as_f64().unwrap() - expected[k]).abs() < 1e-7);
        }
    }
    let clipped = json(
        session
            .intersect_segment(Point3::new(16., 0., 0.), end)
            .unwrap(),
    );
    assert_eq!(clipped["material_intervals"][0][0], 0.);
    assert!((clipped["material_intervals"][0][1].as_f64().unwrap() - 0.5).abs() < 1e-7);
    for (i, v) in [(0, 4.), (1, -1.), (7, 1e-15), (7, f64::NAN)] {
        let mut bad = model;
        bad[i] = v;
        assert!(session.apply(&bad).is_err());
    }
    for (a, b) in [
        (Point3::new(-24., 20., 0.), Point3::new(24., 20., 0.)),
        (Point3::new(20., 0., 0.), end),
        (Point3::new(1e12, 0., 0.), Point3::new(1e12 + 1., 0., 0.)),
        (start, start),
        (Point3::new(f64::NAN, 0., 0.), end),
    ] {
        assert!(session.intersect_segment(a, b).is_err());
    }
    assert_eq!(
        format!("{:?}", session.accepted_body().unwrap().solid()),
        snapshot
    );
    assert_eq!(
        session
            .accepted_body()
            .unwrap()
            .export_step_mm(policy())
            .unwrap(),
        step
    );
    assert_eq!(
        session.query_point(Point3::new(16., 0., 0.)).unwrap(),
        point_result
    );
    assert_eq!(
        serde_json::from_str::<Value>(&report).unwrap()["step"],
        step
    );
}
#[test]
fn posed_hit_witnesses_evaluate_retained_surfaces_without_replay() {
    let mut session = NurbsTorusDemoSession::new();
    session
        .apply(&[16., 4., 0.37, 12., -5., 8., 1e-6, 0.2])
        .unwrap();
    let body = session.accepted_body().unwrap();
    let frame = body.frame();
    let a = frame.point(Point3::new(-24., 2., 0.));
    let b = frame.point(Point3::new(24., 2., 0.));
    let snapshot = format!("{:?}", body.solid());
    let report = json(session.intersect_segment(a, b).unwrap());
    let direct = body.intersect_segment(a, b, policy()).unwrap();
    assert_eq!(report["hits"].as_array().unwrap().len(), direct.hits.len());
    for hit in report["hits"].as_array().unwrap() {
        let t = hit["parameter"].as_f64().unwrap();
        let point = Point3::new(
            hit["point"][0].as_f64().unwrap(),
            hit["point"][1].as_f64().unwrap(),
            hit["point"][2].as_f64().unwrap(),
        );
        assert!((point - (a + (b - a) * t)).norm() < 1e-6);
        let w = &hit["boundary"];
        let face = w["face"].as_u64().unwrap() as usize;
        let actual = body.solid().shell.faces[face]
            .surface
            .try_evaluate(w["uv"][0].as_f64().unwrap(), w["uv"][1].as_f64().unwrap())
            .unwrap();
        assert!((actual - point).norm() < 1e-6);
        let local = frame.local_point(point);
        assert!(((local.x.hypot(local.y) - 16.).hypot(local.z) - 4.).abs() < 1e-6);
    }
    session
        .query_point(frame.point(Point3::new(16., 0., 0.)))
        .unwrap();
    assert_eq!(
        format!("{:?}", session.accepted_body().unwrap().solid()),
        snapshot
    );
}
