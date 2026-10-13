use hagane::*;
use serde_json::Value;
fn parsed(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}
#[test]
fn actual_child_ray_membership_and_boundary_line_origin_preserve_accepted_source() {
    let mut session = NurbsTorusDemoSession::new();
    let origin = Point3::new(16., 0., 0.);
    let direction = Vec3::new(0., 0., 2.);
    assert!(session
        .intersect_horizontal_line(true, origin, direction)
        .is_err());
    assert!(session
        .intersect_horizontal_ray(false, origin, direction)
        .is_err());
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    assert!(session
        .intersect_horizontal_ray(true, origin, direction)
        .is_err());
    session.split_horizontal(1., 0.1).unwrap();
    let pointer = session.accepted_horizontal_split().unwrap() as *const _;
    let source_pointer = session.accepted_body().unwrap() as *const _;
    let before = format!("{:?}", session.accepted_horizontal_split().unwrap());
    let source_line = session.intersect_line(origin, direction).unwrap();
    let source_ray = session.intersect_ray(origin, direction).unwrap();
    let child_point = session.query_horizontal_point(true, origin).unwrap();
    let child_segment = session
        .intersect_horizontal_segment(true, Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
        .unwrap();
    for (lower, expected) in [(true, [-2., 0.5]), (false, [0.5, 2.])] {
        let value = parsed(
            session
                .intersect_horizontal_line(lower, origin, direction)
                .unwrap(),
        );
        assert_eq!(value["query_kind"], "line");
        assert_eq!(value["child"], if lower { "lower" } else { "upper" });
        assert_eq!(value["mesh_free"], true);
        assert_eq!(value["hits"].as_array().unwrap().len(), 2);
        for (index, parameter) in expected.into_iter().enumerate() {
            assert!((value["hits"][index]["parameter"].as_f64().unwrap() - parameter).abs() < 1e-7);
        }
        let ray = parsed(
            session
                .intersect_horizontal_ray(lower, origin, direction)
                .unwrap(),
        );
        assert_eq!(ray["query_kind"], "ray");
        let expected_ray = if lower { [0., 0.5] } else { [0.5, 2.] };
        for (index, parameter) in expected_ray.into_iter().enumerate() {
            assert!(
                (ray["material_intervals"][0][index].as_f64().unwrap() - parameter).abs() < 1e-7
            );
        }
        assert_eq!(
            ray["hits"].as_array().unwrap().len(),
            if lower { 1 } else { 2 }
        );
        assert!(ray.get("mesh").is_none() && ray.get("step").is_none());
    }
    let cap_origin = Point3::new(16., 0., 1.);
    let cap_line = parsed(
        session
            .intersect_horizontal_line(true, cap_origin, direction)
            .unwrap(),
    );
    assert!(cap_line["hits"][1]["parameter"].as_f64().unwrap().abs() < 1e-7);
    assert!(session
        .intersect_horizontal_ray(true, cap_origin, direction)
        .is_err());
    let accepted = session
        .intersect_horizontal_ray(true, origin, direction)
        .unwrap();
    for (point, vector) in [
        (origin, Vec3::new(0., 0., 0.)),
        (Point3::new(f64::NAN, 0., 0.), direction),
        (Point3::new(1e100, 0., 0.), direction),
    ] {
        assert!(session
            .intersect_horizontal_line(true, point, vector)
            .is_err());
        assert!(session
            .intersect_horizontal_ray(true, point, vector)
            .is_err());
    }
    assert!(session.split_horizontal(4., 0.1).is_err());
    assert!(session.split_horizontal(0., 1e-14).is_err());
    assert!(session.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert_eq!(
        session.accepted_horizontal_split().unwrap() as *const _,
        pointer
    );
    assert_eq!(session.accepted_body().unwrap() as *const _, source_pointer);
    assert_eq!(
        format!("{:?}", session.accepted_horizontal_split().unwrap()),
        before
    );
    assert_eq!(
        session.intersect_line(origin, direction).unwrap(),
        source_line
    );
    assert_eq!(
        session.intersect_ray(origin, direction).unwrap(),
        source_ray
    );
    assert_eq!(
        session.query_horizontal_point(true, origin).unwrap(),
        child_point
    );
    assert_eq!(
        session
            .intersect_horizontal_segment(true, Point3::new(16., 0., -8.), Point3::new(16., 0., 8.))
            .unwrap(),
        child_segment
    );
    assert_eq!(
        session
            .intersect_horizontal_ray(true, origin, direction)
            .unwrap(),
        accepted
    );
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    assert!(session.accepted_horizontal_split().is_none());
    assert!(session
        .intersect_horizontal_line(true, origin, direction)
        .is_err());
    assert!(session
        .intersect_horizontal_ray(true, origin, direction)
        .is_err());
}
