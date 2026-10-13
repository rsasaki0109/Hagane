use hagane::*;
use serde_json::Value;

fn parsed(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}

#[test]
fn read_only_child_segments_preserve_cached_geometry_source_queries_and_exports() {
    let mut session = NurbsTorusDemoSession::new();
    let a = Point3::new(16., 0., -8.);
    let b = Point3::new(16., 0., 8.);
    assert!(session.intersect_horizontal_segment(true, a, b).is_err());
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    assert!(session.intersect_horizontal_segment(false, a, b).is_err());
    session.split_horizontal(1., 0.1).unwrap();
    let policy = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let pointer = session.accepted_horizontal_split().unwrap() as *const _;
    let source_pointer = session.accepted_body().unwrap() as *const _;
    let source_step = session
        .accepted_body()
        .unwrap()
        .export_step_mm(policy)
        .unwrap();
    let before_source = session.intersect_segment(a, b).unwrap();
    let before_pair = format!("{:?}", session.accepted_horizontal_split().unwrap());
    for (lower, roots) in [(true, [0.25, 9. / 16.]), (false, [9. / 16., 0.75])] {
        let value = parsed(session.intersect_horizontal_segment(lower, a, b).unwrap());
        assert_eq!(value["child"], if lower { "lower" } else { "upper" });
        assert_eq!(value["cut_height"], 1.);
        assert_eq!(value["mesh_free"], true);
        assert!(value.get("mesh").is_none() && value.get("step").is_none());
        assert_eq!(value["hits"].as_array().unwrap().len(), 2);
        assert_eq!(value["material_intervals"].as_array().unwrap().len(), 1);
        for (index, parameter) in roots.into_iter().enumerate() {
            assert!(
                (value["hits"][index]["parameter"].as_f64().unwrap() - parameter).abs()
                    < 1e-6 / 16.
            );
            let hit = &value["hits"][index];
            let face = hit["boundary"]["face"].as_u64().unwrap() as usize;
            let pair = session.accepted_horizontal_split().unwrap();
            let child = if lower { &pair.lower } else { &pair.upper };
            let uv = [
                hit["boundary"]["uv"][0].as_f64().unwrap(),
                hit["boundary"]["uv"][1].as_f64().unwrap(),
            ];
            let actual = child.solid().shell.faces[face]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            assert!((actual - (a + (b - a) * parameter)).norm() < 1e-6 / 4.);
        }
    }
    let accepted = session.intersect_horizontal_segment(true, a, b).unwrap();
    for (start, end) in [
        (a, a),
        (Point3::new(f64::NAN, 0., 0.), b),
        (Point3::new(16., 0., 1.), b),
    ] {
        assert!(session
            .intersect_horizontal_segment(true, start, end)
            .is_err());
        assert_eq!(
            session.intersect_horizontal_segment(true, a, b).unwrap(),
            accepted
        );
    }
    assert!(session.split_horizontal(4., 0.1).is_err());
    assert!(session.split_horizontal(0., 1e-14).is_err());
    assert!(session
        .apply(&[16., 16., 0., 0., 0., 0., 1e-6, 0.1])
        .is_err());
    assert_eq!(
        session.accepted_horizontal_split().unwrap() as *const _,
        pointer
    );
    assert_eq!(session.accepted_body().unwrap() as *const _, source_pointer);
    assert_eq!(
        format!("{:?}", session.accepted_horizontal_split().unwrap()),
        before_pair
    );
    assert_eq!(
        session
            .accepted_body()
            .unwrap()
            .export_step_mm(policy)
            .unwrap(),
        source_step
    );
    assert_eq!(session.intersect_segment(a, b).unwrap(), before_source);
    assert_eq!(
        session.intersect_horizontal_segment(true, a, b).unwrap(),
        accepted
    );
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    assert!(session.accepted_horizontal_split().is_none());
    assert!(session.intersect_horizontal_segment(true, a, b).is_err());
}
