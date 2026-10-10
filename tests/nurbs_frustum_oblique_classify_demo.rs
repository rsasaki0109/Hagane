use hagane::*;
use serde_json::{json, Value};
fn model(posed: bool) -> (Vec<f64>, Frame3) {
    let (angle, t) = if posed {
        (0.3, [12., -5., 8.])
    } else {
        (0., [0., 0., 0.])
    };
    let frame = Transform::translation(Vec3::new(t[0], t[1], t[2]))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle).unwrap())
        .unwrap();
    let o = frame.point(Point3::new(0., 0., 12.));
    let n = frame.vector(Vec3::new(0.1, 0., 1.));
    (
        vec![
            16., 8., 24., angle, t[0], t[1], t[2], 1e-6, 0.1, o.x, o.y, o.z, n.x, n.y, n.z, 0.,
        ],
        frame,
    )
}
fn query(model: &[f64], point: Point3) -> Value {
    let mut values = model.to_vec();
    values.extend([point.x, point.y, point.z]);
    serde_json::from_str(&nurbs_frustum_oblique_classify_demo_json(&values).unwrap()).unwrap()
}
#[test]
fn actual_both_child_classes_world_pose_and_existing_shape_reports_remain_consistent() {
    for posed in [false, true] {
        let (x, frame) = model(posed);
        let original: Value =
            serde_json::from_str(&nurbs_frustum_plane_split_demo_json(&x).unwrap()).unwrap();
        for (p, classes) in [
            (Point3::new(0., 0., 6.), ["inside", "outside"]),
            (Point3::new(0., 0., 20.), ["outside", "inside"]),
            (Point3::new(0., 0., 12.), ["boundary", "boundary"]),
            (Point3::new(0., 0., 0.), ["boundary", "outside"]),
            (Point3::new(0., 0., 24.), ["outside", "boundary"]),
            (Point3::new(0., 0., 30.), ["outside", "outside"]),
            (Point3::new(14., 0., 6.), ["boundary", "outside"]),
        ] {
            let world = frame.point(p);
            let mut result = query(&x, world);
            assert_eq!(result["point_query"]["classes"], json!(classes));
            assert_eq!(
                result["point_query"]["point"],
                json!([world.x, world.y, world.z])
            );
            assert_eq!(result["point_query"]["mesh_free"], true);
            result.as_object_mut().unwrap().remove("point_query");
            assert_eq!(result, original);
        }
        let p = frame.point(Point3::new(0., 0., 6.));
        let fine = query(&x, p);
        let mut coarse = x.clone();
        coarse[8] = 0.4;
        let coarse = query(&coarse, p);
        assert_eq!(fine["point_query"], coarse["point_query"]);
    }
}
#[test]
fn invalid_and_unresolved_query_requests_return_errors_and_recover_without_partial_classes() {
    let (x, frame) = model(false);
    let p = frame.point(Point3::new(0., 0., 6.));
    let original = query(&x, p);
    let mut values = x.clone();
    values.extend([p.x, p.y, p.z]);
    assert!(nurbs_frustum_oblique_classify_demo_json(&values[..18]).is_err());
    for badpoint in [[f64::NAN, 0., 0.], [f64::INFINITY, 0., 0.], [1e14, 0., 0.]] {
        let mut bad = values.clone();
        bad[16..19].copy_from_slice(&badpoint);
        assert!(nurbs_frustum_oblique_classify_demo_json(&bad).is_err());
    }
    let mut bad = values.clone();
    bad[15] = 0.5;
    assert!(matches!(
        nurbs_frustum_oblique_classify_demo_json(&bad),
        Err(Error::InvalidInput(_))
    ));
    let mut bad = values.clone();
    bad[8] = 1e-12;
    assert!(nurbs_frustum_oblique_classify_demo_json(&bad).is_err());
    assert_eq!(query(&x, p), original);
}
