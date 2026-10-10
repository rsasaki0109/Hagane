use hagane::*;
use serde_json::{json, Value};
fn model(radii: [f64; 2], posed: bool) -> Vec<f64> {
    if posed {
        vec![radii[0], radii[1], 24., 0.3, 12., -5., 8., 1e-6, 0.1, 4.]
    } else {
        vec![radii[0], radii[1], 24., 0., 0., 0., 0., 1e-6, 0.1, 4.]
    }
}
#[test]
fn kept_brep_mesh_step_and_mass_are_actual_bored_geometry_with_analytic_removed_volume() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        for posed in [false, true] {
            let x = model(radii, posed);
            let frame = Transform::translation(Vec3::new(x[4], x[5], x[6]))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[3]).unwrap())
                .unwrap();
            let source = NurbsFrustumSolid::new(frame, radii, 24., policy).unwrap();
            let before = format!("{:?}", source.solid());
            let body = source.bore_coaxial(4., policy).unwrap();
            body.validate(policy).unwrap();
            let data: Value =
                serde_json::from_str(&nurbs_frustum_bore_demo_json(&x).unwrap()).unwrap();
            assert_eq!(data["kind"], "coaxial_bored_frustum");
            assert_eq!(data["closed"], true);
            assert_eq!(data["genus"], 1);
            assert_eq!(data["source_unchanged"], true);
            assert_eq!(
                data["placement"],
                json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)})
            );
            let expected_removed = std::f64::consts::PI * 4f64.powi(2) * 24.;
            let expected_source = std::f64::consts::PI
                * 24.
                * (radii[0].powi(2) + radii[0] * radii[1] + radii[1].powi(2))
                / 3.;
            assert!(
                (data["removed_volume"].as_f64().unwrap() - expected_removed).abs()
                    < 1e-11 * expected_removed
            );
            assert!(
                (data["volume"].as_f64().unwrap() - (expected_source - expected_removed)).abs()
                    < 1e-11 * expected_source
            );
            assert_eq!(data["source_volume"], source.volume(policy).unwrap());
            assert_eq!(data["removed_volume"], body.tool().volume(policy).unwrap());
            assert_eq!(data["volume"], body.volume(policy).unwrap());
            assert_eq!(body.tool().radii(), [4., 4.]);
            let mesh = body.tessellate(0.1, policy).unwrap();
            assert_eq!(
                data["mesh"]["positions"],
                json!(mesh.positions.into_iter().map(xyz).collect::<Vec<_>>())
            );
            assert_eq!(
                data["mesh"]["normals"],
                json!(mesh.normals.into_iter().map(xyz).collect::<Vec<_>>())
            );
            assert_eq!(data["mesh"]["triangles"], json!(mesh.triangles));
            assert_eq!(data["mesh"]["face_ids"], json!(mesh.face_ids));
            let mass = body.inertia_properties(policy).unwrap();
            assert_eq!(data["mass_properties"]["ok"], true);
            assert_eq!(
                data["mass_properties"]["centroid"],
                json!(xyz(mass.centroid))
            );
            assert_eq!(data["mass_properties"]["inertia"], json!(mass.inertia));
            assert_eq!(data["mass_properties"]["reference"], "centroid");
            assert_eq!(data["step"], body.export_step_mm(policy).unwrap());
            assert_ne!(data["step"], source.export_step_mm(policy).unwrap());
            assert!(
                import_step_nurbs_frustum_cardinal_mm(data["step"].as_str().unwrap(), policy)
                    .is_err()
            );
            assert_eq!(
                (
                    body.solid().vertices.len(),
                    body.solid().edges.len(),
                    body.solid().shell.faces.len()
                ),
                (16, 24, 10)
            );
            for i in 0..2 {
                assert_eq!(body.solid().shell.faces[i].wires.len(), 2);
                assert_eq!(data["brep"]["wire_edges"][i].as_array().unwrap().len(), 2);
            }
            for (i, face) in body.solid().shell.faces.iter().enumerate().skip(2) {
                let Surface::Nurbs(surface) = &face.surface else {
                    panic!()
                };
                assert_eq!(
                    data["brep"]["surfaces"][i]["weights"],
                    json!(surface.weights())
                );
                assert_eq!(
                    data["brep"]["surfaces"][i]["control_points"],
                    json!(surface
                        .control_points()
                        .iter()
                        .copied()
                        .map(xyz)
                        .collect::<Vec<_>>())
                );
                if i >= 6 {
                    assert_eq!(face.orientation, -1);
                }
            }
            assert_eq!(format!("{:?}", source.solid()), before);
            assert_eq!(
                format!("{:?}", &body.solid().edges[..12]),
                format!("{:?}", &source.solid().edges[..])
            );
        }
    }
}
#[test]
fn invalid_bore_and_display_requests_fail_without_phantom_reports_and_recover() {
    let x = model([16., 8.], false);
    let original = nurbs_frustum_bore_demo_json(&x).unwrap();
    for radius in [0., -1., 8., 9., f64::NAN, f64::INFINITY] {
        let mut bad = x.clone();
        bad[9] = radius;
        assert!(nurbs_frustum_bore_demo_json(&bad).is_err());
    }
    for chord in [0., -1., 1e-12] {
        let mut bad = x.clone();
        bad[8] = chord;
        assert!(nurbs_frustum_bore_demo_json(&bad).is_err());
    }
    assert!(nurbs_frustum_bore_demo_json(&x[..9]).is_err());
    assert_eq!(nurbs_frustum_bore_demo_json(&x).unwrap(), original);
}
