use hagane::*;
use serde_json::{json, Value};
fn values(cylinder: bool, posed: bool, selected: f64) -> (Vec<f64>, NurbsFrustumSolid, Surface) {
    let (angle, t) = if posed {
        (0.3, [12., -5., 8.])
    } else {
        (0., [0., 0., 0.])
    };
    let frame = Transform::translation(Vec3::new(t[0], t[1], t[2]))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle).unwrap())
        .unwrap();
    let r = if cylinder { [12., 12.] } else { [16., 8.] };
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(frame, r, 24., policy).unwrap();
    let origin = frame.point(Point3::new(0., 0., 12.));
    let n = frame.vector(Vec3::new(0.1, 0., 1.));
    let normal = n.normalized().unwrap();
    let reference = if normal.x.abs() < 0.9 {
        Vec3::new(1., 0., 0.)
    } else {
        Vec3::new(0., 1., 0.)
    };
    let u = reference.cross(normal).normalized().unwrap();
    let v = normal.cross(u).normalized().unwrap();
    (
        vec![
            r[0], r[1], 24., angle, t[0], t[1], t[2], 1e-6, 0.1, origin.x, origin.y, origin.z, n.x,
            n.y, n.z, selected,
        ],
        source,
        Surface::Plane { origin, u, v },
    )
}
#[test]
fn reports_both_actual_closed_children_and_selected_child_step_without_fake_metrics() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let xyz = |p: Vec3| [p.x, p.y, p.z];
    for (cylinder, posed) in [(false, false), (true, false), (false, true)] {
        let (x, source, plane) = values(cylinder, posed, 0.);
        let before = format!("{:?}", source.solid());
        let split = source.split_by_plane(&plane, tol).unwrap();
        let data: Value =
            serde_json::from_str(&nurbs_frustum_plane_split_demo_json(&x).unwrap()).unwrap();
        assert_eq!(data["parts"].as_array().unwrap().len(), 2);
        assert_eq!(data["selected"], 0);
        assert_eq!(data["step"], data["parts"][0]["step"]);
        assert_ne!(data["step"], source.export_step_mm(tol).unwrap());
        let mut sum = 0.;
        for (i, body) in [&split.lower, &split.upper].iter().enumerate() {
            let part = &data["parts"][i];
            let volume = body.volume(tol).unwrap();
            sum += volume;
            assert_eq!(part["volume"], volume);
            assert_eq!(part["is_lower"], i == 0);
            assert!(part.get("centroid").is_none());
            assert!(part.get("inertia").is_none());
            assert_eq!(part["bounds_kind"], "conservative rational control hull");
            let bounds = body.bounds(tol).unwrap();
            assert_eq!(
                part["bounds"],
                json!({"min":xyz(bounds.min),"max":xyz(bounds.max)})
            );
            let mesh = body.tessellate(0.1, tol).unwrap();
            assert_eq!(
                part["mesh"]["positions"],
                json!(mesh.positions.into_iter().map(xyz).collect::<Vec<_>>())
            );
            assert_eq!(
                part["mesh"]["normals"],
                json!(mesh.normals.into_iter().map(xyz).collect::<Vec<_>>())
            );
            assert_eq!(part["mesh"]["triangles"], json!(mesh.triangles));
            assert_eq!(part["mesh"]["face_ids"], json!(mesh.face_ids));
            assert_eq!(part["step"], body.export_step_mm(tol).unwrap());
            for q in 0..4 {
                let Surface::Nurbs(surface) = &body.solid().shell.faces[2 + q].surface else {
                    panic!()
                };
                let report = &part["brep"]["surfaces"][2 + q];
                assert_eq!(report["weights"], json!(surface.weights()));
                assert_eq!(
                    report["control_points"],
                    json!(surface
                        .control_points()
                        .iter()
                        .copied()
                        .map(xyz)
                        .collect::<Vec<_>>())
                );
            }
            assert!(matches!(
                import_step_nurbs_frustum_cardinal_mm(part["step"].as_str().unwrap(), tol),
                Err(Error::Unsupported(_))
            ));
        }
        assert!((sum - source.volume(tol).unwrap()).abs() < 1e-10 * sum);
        assert_eq!(
            data["part_volumes"],
            json!([data["parts"][0]["volume"], data["parts"][1]["volume"]])
        );
        let mut upper = x.clone();
        upper[15] = 1.;
        let upper: Value =
            serde_json::from_str(&nurbs_frustum_plane_split_demo_json(&upper).unwrap()).unwrap();
        assert_eq!(upper["parts"], data["parts"]);
        assert_eq!(upper["step"], data["parts"][1]["step"]);
        assert_eq!(upper["section"], data["section"]);
        assert_eq!(format!("{:?}", source.solid()), before);
    }
}
#[test]
fn invalid_selection_planes_and_unresolved_display_do_not_return_partial_children() {
    let (x, _, _) = values(false, false, 0.);
    let original = nurbs_frustum_plane_split_demo_json(&x).unwrap();
    for selection in [-1., 0.5, 2., f64::NAN] {
        let mut bad = x.clone();
        bad[15] = selection;
        assert!(matches!(
            nurbs_frustum_plane_split_demo_json(&bad),
            Err(Error::InvalidInput(_))
        ));
    }
    for (origin, normal) in [
        ([0., 0., 30.], [0., 0., 1.]),
        ([0., 0., 12.], [1., 0., 0.]),
        ([0., 0., 12.], [0., 0., 0.]),
    ] {
        let mut bad = x.clone();
        bad[9..12].copy_from_slice(&origin);
        bad[12..15].copy_from_slice(&normal);
        assert!(nurbs_frustum_plane_split_demo_json(&bad).is_err());
    }
    let mut bad = x.clone();
    bad[8] = 1e-12;
    assert!(nurbs_frustum_plane_split_demo_json(&bad).is_err());
    assert!(nurbs_frustum_plane_split_demo_json(&x[..15]).is_err());
    assert_eq!(nurbs_frustum_plane_split_demo_json(&x).unwrap(), original);
}
