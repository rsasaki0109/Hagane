use hagane::*;
use serde_json::Value;
fn curve(data: &Value) -> NurbsCurve {
    NurbsCurve::new(
        data["degree"].as_u64().unwrap() as usize,
        data["knots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect(),
        data["control_points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                Point3::new(
                    v[0].as_f64().unwrap(),
                    v[1].as_f64().unwrap(),
                    v[2].as_f64().unwrap(),
                )
            })
            .collect(),
        data["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect(),
    )
    .unwrap()
}
fn point(data: &Value) -> Point3 {
    Point3::new(
        data[0].as_f64().unwrap(),
        data[1].as_f64().unwrap(),
        data[2].as_f64().unwrap(),
    )
}
fn values(cylinder: bool, posed: bool) -> (Vec<f64>, NurbsFrustumSolid) {
    let (angle, translation) = if posed {
        (0.3, [12., -5., 8.])
    } else {
        (0., [0., 0., 0.])
    };
    let frame = Transform::translation(Vec3::new(translation[0], translation[1], translation[2]))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle).unwrap())
        .unwrap();
    let radii = if cylinder { [12., 12.] } else { [16., 8.] };
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(frame, radii, 24., tol).unwrap();
    let origin = frame.point(Point3::new(0., 0., 12.));
    let normal = frame.vector(Vec3::new(0.1, 0., 1.));
    (
        vec![
            radii[0],
            radii[1],
            24.,
            angle,
            translation[0],
            translation[1],
            translation[2],
            1e-6,
            0.1,
            origin.x,
            origin.y,
            origin.z,
            normal.x,
            normal.y,
            normal.z,
        ],
        body,
    )
}
#[test]
fn actual_quadratic_sections_and_cubic_uv_uses_match_plane_surfaces_and_bounded_display() {
    for (cylinder, posed) in [(false, false), (true, false), (false, true)] {
        let (x, body) = values(cylinder, posed);
        let data: Value =
            serde_json::from_str(&nurbs_frustum_plane_section_demo_json(&x).unwrap()).unwrap();
        let section = &data["section"];
        assert_eq!(section["closed"], true);
        assert_eq!(section["mesh_free"], true);
        assert_eq!(section["cut_created"], false);
        assert_eq!(
            data["step"],
            body.export_step_mm(GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap())
                .unwrap()
        );
        let origin = point(&section["plane_origin"]);
        let normal = point(&section["plane_normal"]);
        let edges = section["curves"].as_array().unwrap();
        assert_eq!(edges.len(), 4);
        for (i, edge) in edges.iter().enumerate() {
            let c = curve(&edge["curve"]);
            let uv = curve(&edge["pcurve"]);
            assert_eq!(c.degree(), 2);
            assert_eq!(uv.degree(), 3);
            assert!(uv.control_points().iter().all(|p| p.z == 0.));
            let face = edge["face_id"].as_u64().unwrap() as usize;
            assert_eq!(face, 2 + i);
            for j in 0..=64 {
                let t = j as f64 / 64.;
                let p = c.evaluate(t).unwrap();
                let q = uv.evaluate(t).unwrap();
                assert!((p - origin).dot(normal).abs() < 1e-9);
                assert!(
                    (p - body.solid().shell.faces[face]
                        .surface
                        .try_evaluate(q.x, q.y)
                        .unwrap())
                    .norm()
                        < 1e-9
                );
            }
            assert!(
                (c.evaluate(1.).unwrap()
                    - curve(&edges[(i + 1) % 4]["curve"]).evaluate(0.).unwrap())
                .norm()
                    < 1e-9
            );
            let points = edge["points"].as_array().unwrap();
            let params = edge["parameters"].as_array().unwrap();
            let bounds = edge["error_bounds"].as_array().unwrap();
            assert_eq!(points.len(), params.len());
            assert_eq!(bounds.len() + 1, points.len());
            for k in 0..bounds.len() {
                let bound = bounds[k].as_f64().unwrap();
                assert!(bound <= x[8]);
                let a = params[k].as_f64().unwrap();
                let b = params[k + 1].as_f64().unwrap();
                for j in 0..=8 {
                    let s = j as f64 / 8.;
                    let actual = c.evaluate(a * (1. - s) + b * s).unwrap();
                    let chord = point(&points[k]) * (1. - s) + point(&points[k + 1]) * s;
                    assert!((actual - chord).norm() <= bound + 1e-9);
                }
            }
        }
        let mut coarse = x.clone();
        coarse[8] = 0.4;
        let coarse: Value =
            serde_json::from_str(&nurbs_frustum_plane_section_demo_json(&coarse).unwrap()).unwrap();
        for i in 0..4 {
            assert_eq!(coarse["section"]["curves"][i]["curve"], edges[i]["curve"]);
            assert_eq!(coarse["section"]["curves"][i]["pcurve"], edges[i]["pcurve"]);
        }
        assert_eq!(coarse["step"], data["step"]);
    }
}
#[test]
fn out_of_scope_planes_bad_inputs_and_display_errors_return_no_partial_results() {
    let (x, _) = values(false, false);
    let original = nurbs_frustum_plane_section_demo_json(&x).unwrap();
    for (origin, normal) in [
        ([0., 0., 30.], [0., 0., 1.]),
        ([0., 0., 12.], [1., 0., 0.]),
        ([0., 0., 12.], [5., 0., 1.]),
        ([0., 0., 12.], [0., 0., 0.]),
    ] {
        let mut bad = x.clone();
        bad[9..12].copy_from_slice(&origin);
        bad[12..15].copy_from_slice(&normal);
        assert!(nurbs_frustum_plane_section_demo_json(&bad).is_err());
    }
    assert!(nurbs_frustum_plane_section_demo_json(&x[..14]).is_err());
    let mut bad = x.clone();
    bad[12] = f64::NAN;
    assert!(nurbs_frustum_plane_section_demo_json(&bad).is_err());
    let mut bad = x.clone();
    bad[8] = 1e-12;
    assert!(nurbs_frustum_plane_section_demo_json(&bad).is_err());
    assert_eq!(nurbs_frustum_plane_section_demo_json(&x).unwrap(), original);
}
