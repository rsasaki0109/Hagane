use hagane::*;
use serde_json::Value;
fn parse(s: String) -> Value {
    serde_json::from_str(&s).unwrap()
}
#[test]
fn rejected_models_and_queries_preserve_actual_accepted_body() {
    let mut session = NurbsTorusDemoSession::new();
    assert!(session.query_point(Point3::new(16., 0., 0.)).is_err());
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    let report = session.apply(&model).unwrap();
    assert_eq!(report, nurbs_torus_demo_json(&model).unwrap());
    let p = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let snapshot = format!("{:?}", session.accepted_body().unwrap().solid());
    let step = session.accepted_body().unwrap().export_step_mm(p).unwrap();
    let good = session.query_point(Point3::new(16., 0., 0.)).unwrap();
    assert_eq!(parse(good.clone())["location"], "inside");
    let mut invalid = Vec::new();
    for (index, value) in [(0, 4.), (1, -1.), (7, 1e-15), (7, f64::NAN), (6, 0.)] {
        let mut x = model;
        x[index] = value;
        invalid.push(x);
    }
    for x in invalid {
        assert!(session.apply(&x).is_err());
        assert_eq!(
            format!("{:?}", session.accepted_body().unwrap().solid()),
            snapshot
        );
        assert_eq!(
            session.accepted_body().unwrap().export_step_mm(p).unwrap(),
            step
        );
        assert_eq!(session.query_point(Point3::new(16., 0., 0.)).unwrap(), good);
    }
    for point in [Point3::new(f64::NAN, 0., 0.), Point3::new(1e12, 0., 0.)] {
        assert!(session.query_point(point).is_err());
        assert_eq!(
            format!("{:?}", session.accepted_body().unwrap().solid()),
            snapshot
        );
    }
    let value: Value = serde_json::from_str(&report).unwrap();
    assert!(!value["mesh"]["triangles"].as_array().unwrap().is_empty());
    assert_eq!(value["step"], step);
}
#[test]
fn posed_queries_return_actual_world_surface_witness_without_rebuilding() {
    let mut session = NurbsTorusDemoSession::new();
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    let model = [20., 3., 0.37, 12., -5., 8., 1e-6, 0.2];
    let report = session.apply(&model).unwrap();
    let body = session.accepted_body().unwrap();
    assert_eq!(body.radii(), [20., 3.]);
    let p = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let frame = body.frame();
    let snapshot = format!("{:?}", body.solid());
    let step = body.export_step_mm(p).unwrap();
    let inside = frame.point(Point3::new(20., 0., 0.));
    let void = frame.origin();
    assert_eq!(
        parse(session.query_point(inside).unwrap())["location"],
        "inside"
    );
    assert_eq!(
        parse(session.query_point(void).unwrap())["location"],
        "outside"
    );
    let boundary = body.solid().shell.faces[5]
        .surface
        .try_evaluate(0.31, 0.63)
        .unwrap();
    let result = parse(session.query_point(boundary).unwrap());
    assert_eq!(result["location"], "boundary");
    assert!(result.get("mesh").is_none());
    assert!(result.get("step").is_none());
    let witness = &result["boundary"];
    let id = witness["face"].as_u64().unwrap() as usize;
    let uv = [
        witness["uv"][0].as_f64().unwrap(),
        witness["uv"][1].as_f64().unwrap(),
    ];
    let actual = body.solid().shell.faces[id]
        .surface
        .try_evaluate(uv[0], uv[1])
        .unwrap();
    assert!((actual - boundary).norm() < 1e-6);
    assert_eq!(
        body.point_query(boundary, p).unwrap().location,
        PointLocation::Boundary
    );
    assert_eq!(
        format!("{:?}", session.accepted_body().unwrap().solid()),
        snapshot
    );
    assert_eq!(
        session.accepted_body().unwrap().export_step_mm(p).unwrap(),
        step
    );
    assert_eq!(report, nurbs_torus_demo_json(&model).unwrap());
}
