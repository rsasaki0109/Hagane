use hagane::*;
use serde_json::{json, Value};

fn xyz(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
fn cardinal_frames() -> Vec<Frame3> {
    let basis = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut frames = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }
            for a in [-1., 1.] {
                for b in [-1., 1.] {
                    let u = basis[i] * a;
                    let v = basis[j] * b;
                    frames.push(
                        Frame3::new_with_tolerance(
                            Point3::new(12., -5., 8.),
                            [u, v, u.cross(v)],
                            GeometryTolerance::default(),
                        )
                        .unwrap(),
                    );
                }
            }
        }
    }
    assert_eq!(frames.len(), 24);
    frames
}

#[test]
fn all_24_signed_cardinal_frames_serialize_actual_nets_mesh_mass_and_axes() {
    let policy = GeometryTolerance::default();
    for frame in cardinal_frames() {
        let source = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy).unwrap();
        let body = source.split_axial(10., policy).unwrap().upper;
        let step = body.export_step_mm(policy).unwrap();
        let imported = import_step_nurbs_frustum_cardinal_mm(&step, policy).unwrap();
        assert_eq!(
            format!("{:?}", imported.solid()),
            format!("{:?}", body.solid())
        );
        let actual: Value = serde_json::from_str(
            &nurbs_frustum_cardinal_step_import_demo_json(&step, 0.1).unwrap(),
        )
        .unwrap();
        let actual_frame = body.frame();
        assert_eq!(
            actual["placement"],
            json!({"translation":xyz(actual_frame.origin()), "axes":actual_frame.axes().map(xyz)})
        );
        assert!(actual["placement"].get("angle").is_none());
        assert_eq!(actual["axis"], json!(xyz(actual_frame.axes()[2])));
        assert_eq!(actual["radii"], json!(body.radii()));
        assert_eq!(actual["height"], body.height());
        let mass = body.mass_properties(policy).unwrap();
        let inertia = body.inertia_properties(policy).unwrap();
        assert_eq!(actual["volume"], mass.volume);
        assert_eq!(actual["centroid"], json!(xyz(mass.centroid)));
        assert_eq!(actual["inertia"], json!(inertia.inertia));
        let mesh = body.tessellate(0.1, policy).unwrap();
        assert_eq!(
            actual["mesh"]["positions"],
            json!(mesh.positions.into_iter().map(xyz).collect::<Vec<_>>())
        );
        assert_eq!(
            actual["mesh"]["normals"],
            json!(mesh.normals.into_iter().map(xyz).collect::<Vec<_>>())
        );
        assert_eq!(actual["mesh"]["triangles"], json!(mesh.triangles));
        assert_eq!(actual["mesh"]["face_ids"], json!(mesh.face_ids));
        assert_eq!(actual["step"], step);
        assert_eq!(actual["import"]["kind"], "nurbs_frustum_cardinal");
        assert_eq!(actual["import"]["source_geometry_preserved"], true);
        assert_eq!(actual["import"]["pcurve_representation_verified"], true);
        assert_eq!(actual["import"]["validated_geometry"], true);
        assert!(matches!(
            import_step_nurbs_frustum_mm(&step, policy),
            Err(Error::Unsupported(_))
        ));
        if actual_frame.axes() != Frame3::IDENTITY.axes() {
            assert!(matches!(
                import_step_nurbs_frustum_translated_mm(&step, policy),
                Err(Error::Unsupported(_))
            ));
        }
    }
}

#[test]
fn cardinal_demo_failures_are_explicit_and_do_not_broaden_old_api_scopes() {
    let policy = GeometryTolerance::default();
    let frame = Frame3::new_with_tolerance(
        Point3::new(12., -5., 8.),
        [
            Vec3::new(0., 1., 0.),
            Vec3::new(0., 0., 1.),
            Vec3::new(1., 0., 0.),
        ],
        policy,
    )
    .unwrap();
    let body = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy).unwrap();
    let step = body.export_step_mm(policy).unwrap();
    let good = nurbs_frustum_cardinal_step_import_demo_json(&step, 0.1).unwrap();
    for chord in [0., -1., f64::NAN, f64::INFINITY, 1e-12] {
        assert!(nurbs_frustum_cardinal_step_import_demo_json(&step, chord).is_err());
    }
    for bad in [
        String::new(),
        "not STEP".into(),
        step[..step.len() / 2].into(),
        format!("{step}garbage"),
        "x".repeat(2 * 1024 * 1024 + 1),
    ] {
        assert!(nurbs_frustum_cardinal_step_import_demo_json(&bad, 0.1).is_err());
    }
    let rotated = NurbsFrustumSolid::new(
        Transform::rotation(Vec3::new(0., 1., 0.), 0.2).unwrap(),
        [16., 8.],
        24.,
        policy,
    )
    .unwrap();
    assert!(matches!(
        nurbs_frustum_cardinal_step_import_demo_json(&rotated.export_step_mm(policy).unwrap(), 0.1),
        Err(Error::Unsupported(_))
    ));
    assert!(nurbs_frustum_translated_step_import_demo_json(&step, 0.1).is_err());
    assert!(nurbs_frustum_step_import_demo_json(&step, 0.1).is_err());
    assert!(matches!(
        import_step_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_bounded_analytic_mm(&step, policy.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        nurbs_frustum_cardinal_step_import_demo_json(&step, 0.1).unwrap(),
        good
    );
}
