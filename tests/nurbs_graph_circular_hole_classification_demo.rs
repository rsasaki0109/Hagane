use hagane::*;
use serde_json::Value;
fn input(point: [f64; 3]) -> Vec<f64> {
    let mut x = vec![
        80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 40., 30., 12.,
    ];
    x.extend(point);
    x.push(1e-8);
    x
}
fn location(x: &[f64]) -> Value {
    serde_json::from_str(&nurbs_graph_circular_hole_point_demo_json(x).unwrap()).unwrap()
}
#[test]
fn material_bore_wall_caps_and_outer_boundary_are_distinguished() {
    for (p, expected) in [
        ([10., 30., 10.], "Inside"),
        ([40., 30., 10.], "Outside"),
        ([52., 30., 10.], "Boundary"),
        ([10., 30., 0.], "Boundary"),
        ([0., 0., 10.], "Boundary"),
        ([40., 30., 0.], "Outside"),
        ([10., 30., -1.], "Outside"),
        ([10., 30., 30.], "Outside"),
    ] {
        let d = location(&input(p));
        assert_eq!(d["location"], expected);
        assert_eq!(d["point"], serde_json::json!(p));
        assert_eq!(d["units"], "mm");
    }
}
#[test]
fn checked_query_is_model_only_and_preserves_world_pose() {
    let mut x = input([10., 30., 10.]);
    x[4] = 1e-12;
    assert!(nurbs_graph_circular_hole_demo_json(&x[..16]).is_err());
    assert_eq!(location(&x)["location"], "Inside");
    x[19] = 1e-6;
    x[5] = 0.3;
    x[6] = 100.;
    x[7] = -30.;
    x[8] = 12.;
    let world = Transform::translation(Vec3::new(100., -30., 12.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.3).unwrap())
        .unwrap()
        .point(Point3::new(10., 30., 10.));
    x[16] = world.x;
    x[17] = world.y;
    x[18] = world.z;
    let d = location(&x);
    assert_eq!(d["location"], "Inside");
    for (k, v) in [10., 30., 10.].iter().enumerate() {
        assert!((d["source_point"][k].as_f64().unwrap() - v).abs() < 1e-10);
    }
}
#[test]
fn malformed_input_and_unresolved_tolerance_reject_then_recover() {
    let good = input([10., 30., 10.]);
    for (i, v) in [
        (19, 0.),
        (19, -1.),
        (19, f64::NAN),
        (19, f64::INFINITY),
        (19, 1e-14),
        (16, f64::NAN),
        (15, 0.),
        (15, 30.),
    ] {
        let mut x = good.clone();
        x[i] = v;
        assert!(nurbs_graph_circular_hole_point_demo_json(&x).is_err());
    }
    assert!(nurbs_graph_circular_hole_point_demo_json(&good[..19]).is_err());
    let mut long = good.clone();
    long.push(0.);
    assert!(nurbs_graph_circular_hole_point_demo_json(&long).is_err());
    assert_eq!(location(&good)["location"], "Inside");
}
