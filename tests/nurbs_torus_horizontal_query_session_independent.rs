use hagane::*;
use serde_json::Value;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap()
}
fn json(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}
fn point(v: &Value) -> Point3 {
    Point3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}
fn witness(body: &NurbsTorusHorizontalSolid, report: &Value, expected: Point3) {
    assert_eq!(report["location"], "boundary");
    assert!(report.get("mesh").is_none() && report.get("step").is_none());
    let w = &report["boundary"];
    let face = &body.solid().shell.faces[w["face"].as_u64().unwrap() as usize];
    let uv = [w["uv"][0].as_f64().unwrap(), w["uv"][1].as_f64().unwrap()];
    let evaluated = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
    assert!((evaluated - expected).norm() < 1e-7);
    assert!((point(&w["point"]) - expected).norm() < 1e-7);
    assert!(w["distance"].as_f64().unwrap() < 1e-7);
    let reported = point(&w["normal"]) - Point3::new(0., 0., 0.);
    let local = body.source().frame().local_point(expected);
    let normal = if (local.z - body.cut_height()).abs() < 1e-8 {
        body.source()
            .frame()
            .vector(Vec3::new(0., 0., if body.is_lower() { 1. } else { -1. }))
    } else {
        let rho = local.x.hypot(local.y);
        body.source().frame().vector(Vec3::new(
            (rho - 16.) * local.x / (4. * rho),
            (rho - 16.) * local.y / (4. * rho),
            local.z / 4.,
        ))
    };
    assert!((reported - normal).norm() < 1e-7);
}

#[test]
fn both_cached_children_report_actual_posed_cap_and_curved_face_witnesses() {
    let mut session = NurbsTorusDemoSession::new();
    assert!(session
        .query_horizontal_point(true, Point3::new(16., 0., 0.))
        .is_err());
    session
        .apply(&[16., 4., 0.37, 12., -5., 8., 1e-6, 0.1])
        .unwrap();
    assert!(session
        .query_horizontal_point(false, Point3::new(16., 0., 0.))
        .is_err());
    session.split_horizontal(0.7, 0.1).unwrap();
    let pair = session.accepted_horizontal_split().unwrap();
    let frame = session.accepted_body().unwrap().frame();
    let cap = frame.point(Point3::new(16., 0., 0.7));
    for (lower, child) in [(true, &pair.lower), (false, &pair.upper)] {
        let q = json(session.query_horizontal_point(lower, cap).unwrap());
        assert_eq!(q["child"], if lower { "lower" } else { "upper" });
        assert_eq!(q["cut_height"], 0.7);
        witness(child, &q, cap);
        let surface = child
            .solid()
            .shell
            .faces
            .iter()
            .find_map(|face| {
                if let Surface::Nurbs(surface) = &face.surface {
                    Some(surface)
                } else {
                    None
                }
            })
            .unwrap();
        let domain = surface.domain();
        let p = surface
            .evaluate(
                domain[0][0] + 0.35 * (domain[0][1] - domain[0][0]),
                domain[1][0] + 0.6 * (domain[1][1] - domain[1][0]),
            )
            .unwrap();
        witness(
            child,
            &json(session.query_horizontal_point(lower, p).unwrap()),
            p,
        );
        let inside = frame.point(Point3::new(16., 0., if lower { -1. } else { 2. }));
        let other = frame.point(Point3::new(16., 0., if lower { 2. } else { -1. }));
        assert_eq!(
            json(session.query_horizontal_point(lower, inside).unwrap())["location"],
            "inside"
        );
        assert_eq!(
            json(session.query_horizontal_point(lower, other).unwrap())["location"],
            "outside"
        );
        assert_eq!(
            json(
                session
                    .query_horizontal_point(lower, frame.origin())
                    .unwrap()
            )["location"],
            "outside"
        );
    }
}

#[test]
fn failed_requests_keep_cached_pair_and_source_but_new_source_invalidates_children() {
    let mut session = NurbsTorusDemoSession::new();
    let model = [16., 4., 0., 0., 0., 0., 1e-6, 0.1];
    session.apply(&model).unwrap();
    session.split_horizontal(0.7, 0.1).unwrap();
    let pair = session.accepted_horizontal_split().unwrap();
    let pair_id = pair as *const _ as usize;
    let source_id = session.accepted_body().unwrap() as *const _ as usize;
    let snapshot = [
        format!("{:?}", pair.lower.solid()),
        format!("{:?}", pair.upper.solid()),
        format!("{:?}", session.accepted_body().unwrap().solid()),
    ];
    let steps = [
        pair.lower.export_step_mm(policy()).unwrap(),
        pair.upper.export_step_mm(policy()).unwrap(),
        session
            .accepted_body()
            .unwrap()
            .export_step_mm(policy())
            .unwrap(),
    ];
    let cap = Point3::new(16., 0., 0.7);
    let prior = [
        session.query_horizontal_point(true, cap).unwrap(),
        session.query_horizontal_point(false, cap).unwrap(),
        session.query_point(Point3::new(16., 0., 0.)).unwrap(),
    ];
    for (height, chord) in [(4., 0.1), (f64::NAN, 0.1), (0.7, 1e-15)] {
        assert!(session.split_horizontal(height, chord).is_err());
    }
    for (index, value) in [(1, 16.), (0, f64::NAN), (7, 1e-15)] {
        let mut bad = model;
        bad[index] = value;
        assert!(session.apply(&bad).is_err());
    }
    for lower in [true, false] {
        assert!(session
            .query_horizontal_point(lower, Point3::new(f64::NAN, 0., 0.))
            .is_err());
        assert!(session
            .query_horizontal_point(lower, Point3::new(1e100, 0., 0.))
            .is_err());
    }
    let pair = session.accepted_horizontal_split().unwrap();
    assert_eq!(pair as *const _ as usize, pair_id);
    assert_eq!(
        session.accepted_body().unwrap() as *const _ as usize,
        source_id
    );
    assert_eq!(
        [
            format!("{:?}", pair.lower.solid()),
            format!("{:?}", pair.upper.solid()),
            format!("{:?}", session.accepted_body().unwrap().solid())
        ],
        snapshot
    );
    assert_eq!(
        [
            pair.lower.export_step_mm(policy()).unwrap(),
            pair.upper.export_step_mm(policy()).unwrap(),
            session
                .accepted_body()
                .unwrap()
                .export_step_mm(policy())
                .unwrap()
        ],
        steps
    );
    assert_eq!(
        [
            session.query_horizontal_point(true, cap).unwrap(),
            session.query_horizontal_point(false, cap).unwrap(),
            session.query_point(Point3::new(16., 0., 0.)).unwrap()
        ],
        prior
    );
    let mut changed = model;
    changed[0] = 20.;
    session.apply(&changed).unwrap();
    assert!(session.accepted_horizontal_split().is_none());
    assert!(session.query_horizontal_point(true, cap).is_err());
    assert!(session.query_horizontal_point(false, cap).is_err());
    session.split_horizontal(-0.7, 0.1).unwrap();
    assert_eq!(
        json(
            session
                .query_horizontal_point(true, Point3::new(20., 0., -1.))
                .unwrap()
        )["location"],
        "inside"
    );
    assert_eq!(
        json(
            session
                .query_horizontal_point(false, Point3::new(20., 0., 1.))
                .unwrap()
        )["location"],
        "inside"
    );
}
