use hagane::*;
use serde_json::Value;

fn json(s: String) -> Value {
    serde_json::from_str(&s).unwrap()
}
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn original_nonunit_line_parameters_and_inside_ray_clipping() {
    let mut session = NurbsTorusDemoSession::new();
    let o = Point3::new(0., 0., 0.);
    let d = Vec3::new(2., 0., 0.);
    assert!(session.intersect_line(o, d).is_err());
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    let line = json(session.intersect_line(o, d).unwrap());
    assert_eq!(line["mesh_free"], true);
    assert!(line.get("mesh").is_none() && line.get("step").is_none());
    let hits = line["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 4);
    for (h, t) in hits.iter().zip([-10., -6., 6., 10.]) {
        close(h["parameter"].as_f64().unwrap(), t);
        close(h["point"][0].as_f64().unwrap(), 2. * t);
    }
    assert_eq!(line["material_intervals"].as_array().unwrap().len(), 2);
    for (v, expected) in line["material_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .zip([[-10., -6.], [6., 10.]])
    {
        for k in 0..2 {
            close(v[k].as_f64().unwrap(), expected[k]);
        }
    }
    for (direction, expected) in [(d, [[0., 2.], [0., 0.]]), (d * -1., [[0., 2.], [14., 18.]])] {
        let ray = json(
            session
                .intersect_ray(Point3::new(16., 0., 0.), direction)
                .unwrap(),
        );
        let intervals = ray["material_intervals"].as_array().unwrap();
        assert_eq!(intervals.len(), if direction.x > 0. { 1 } else { 2 });
        for (v, e) in intervals.iter().zip(expected) {
            for k in 0..2 {
                close(v[k].as_f64().unwrap(), e[k]);
            }
        }
    }
}

#[test]
fn posed_world_witnesses_and_rejections_preserve_all_accepted_queries() {
    let mut session = NurbsTorusDemoSession::new();
    let model = [16., 4., 0.37, 12., -5., 8., 1e-6, 0.1];
    session.apply(&model).unwrap();
    let body = session.accepted_body().unwrap();
    let frame = body.frame();
    let snapshot = format!("{:?}", body.solid());
    let step = body.export_step_mm(policy()).unwrap();
    let o = frame.point(Point3::new(0., 2., 0.));
    let d = frame.vector(Vec3::new(2., 0., 0.));
    let point = frame.point(Point3::new(16., 0., 0.));
    let a = frame.point(Point3::new(-24., 2., 0.));
    let b = frame.point(Point3::new(24., 2., 0.));
    let prior_point = session.query_point(point).unwrap();
    let prior_segment = session.intersect_segment(a, b).unwrap();
    let line_text = session.intersect_line(o, d).unwrap();
    let line = json(line_text.clone());
    assert_eq!(line["hits"].as_array().unwrap().len(), 4);
    for h in line["hits"].as_array().unwrap() {
        let t = h["parameter"].as_f64().unwrap();
        let p = Point3::new(
            h["point"][0].as_f64().unwrap(),
            h["point"][1].as_f64().unwrap(),
            h["point"][2].as_f64().unwrap(),
        );
        assert!((p - (o + d * t)).norm() < 1e-6);
        let w = &h["boundary"];
        let surface = &body.solid().shell.faces[w["face"].as_u64().unwrap() as usize].surface;
        let evaluated = surface
            .try_evaluate(w["uv"][0].as_f64().unwrap(), w["uv"][1].as_f64().unwrap())
            .unwrap();
        assert!((evaluated - p).norm() < 1e-6);
        let local = frame.local_point(p);
        let rho = local.x.hypot(local.y);
        let normal = frame.vector(Vec3::new(
            (rho - 16.) * local.x / (4. * rho),
            (rho - 16.) * local.y / (4. * rho),
            local.z / 4.,
        ));
        let reported = Vec3::new(
            w["normal"][0].as_f64().unwrap(),
            w["normal"][1].as_f64().unwrap(),
            w["normal"][2].as_f64().unwrap(),
        );
        assert!((normal - reported).norm() < 1e-6);
    }
    for (index, value) in [(0, 4.), (7, 1e-15), (1, f64::NAN)] {
        let mut bad = model;
        bad[index] = value;
        assert!(session.apply(&bad).is_err());
    }
    for (origin, direction) in [
        (o, Vec3::new(0., 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), d),
        (Point3::new(1e100, 0., 0.), d),
    ] {
        assert!(session.intersect_line(origin, direction).is_err());
        assert!(session.intersect_ray(origin, direction).is_err());
    }
    assert!(session
        .intersect_ray(frame.point(Point3::new(20., 0., 0.)), d)
        .is_err());
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
    assert_eq!(session.query_point(point).unwrap(), prior_point);
    assert_eq!(session.intersect_segment(a, b).unwrap(), prior_segment);
    assert_eq!(session.intersect_line(o, d).unwrap(), line_text);
}
