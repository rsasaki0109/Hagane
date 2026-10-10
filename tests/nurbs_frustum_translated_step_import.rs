use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn source(origin: Vec3, radii: [f64; 2], height: f64) -> NurbsFrustumSolid {
    NurbsFrustumSolid::new(
        Transform::translation(origin).unwrap(),
        radii,
        height,
        policy(),
    )
    .unwrap()
}
fn check(body: &NurbsFrustumSolid) {
    let text = body.export_step_mm(policy()).unwrap();
    let read = import_step_nurbs_frustum_translated_mm(&text, policy()).unwrap();
    read.validate(policy()).unwrap();
    assert_eq!(
        read.radii(),
        body.radii(),
        "radius must come from cap UV, not rounded world offsets"
    );
    assert_eq!(read.frame().origin(), body.frame().origin());
    assert_eq!(
        read.export_step_mm(policy()).unwrap(),
        text,
        "actual nets/weights/topology must remain unchanged"
    );
    for (a, b) in read
        .solid()
        .shell
        .faces
        .iter()
        .zip(&body.solid().shell.faces)
    {
        if let (Surface::Nurbs(a), Surface::Nurbs(b)) = (&a.surface, &b.surface) {
            assert_eq!(a.control_points(), b.control_points());
            assert_eq!(a.weights(), b.weights());
            for axis in 0..2 {
                assert_eq!(a.knots(axis).unwrap(), b.knots(axis).unwrap());
            }
        }
    }
}

#[test]
fn translated_cap_uv_radii_and_actual_children_roundtrip() {
    // Height metadata is inferred from represented world planes; verify actual
    // retained geometry rather than requiring the original decimal recipe.
    check(&source(Vec3::new(1000., -2000., 1000.), [0.1, 0.2], 0.1));
    for origin in [
        Vec3::new(12., -3., 5.),
        Vec3::new(1000., -2000., 1000.),
        Vec3::new(-0.125, 0.25, -0.5),
    ] {
        for radii in [[16., 8.], [8., 16.], [12., 12.], [0.1, 0.2]] {
            let body = source(origin, radii, 24.);
            check(&body);
            let split = body.split_axial(9., policy()).unwrap();
            check(&split.upper);
            for child in body
                .split_axial_many(&[4., 10., 18.], policy())
                .unwrap()
                .parts
            {
                check(&child);
            }
            assert!(import_step_nurbs_frustum_mm(
                &body.export_step_mm(policy()).unwrap(),
                policy()
            )
            .is_err());
        }
    }
}

#[test]
fn unplaced_compatibility_rotation_and_precision_refusal() {
    let plain = source(Vec3::new(0., 0., 0.), [16., 8.], 24.);
    let text = plain.export_step_mm(policy()).unwrap();
    assert_eq!(
        import_step_nurbs_frustum_mm(&text, policy())
            .unwrap()
            .export_step_mm(policy())
            .unwrap(),
        import_step_nurbs_frustum_translated_mm(&text, policy())
            .unwrap()
            .export_step_mm(policy())
            .unwrap()
    );
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.2).unwrap())
        .unwrap();
    let rotated = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy()).unwrap();
    assert!(import_step_nurbs_frustum_translated_mm(
        &rotated.export_step_mm(policy()).unwrap(),
        policy()
    )
    .is_err());
    let translated = source(Vec3::new(1000., -2000., 1000.), [16., 8.], 24.)
        .export_step_mm(policy())
        .unwrap();
    let strict = GeometryTolerance::new(1e-12, 1e-10, 0.).unwrap();
    assert!(matches!(
        import_step_nurbs_frustum_translated_mm(&translated, strict),
        Err(Error::Unsupported(_))
    ));
    check(&plain);
    let represented = source(Vec3::new(12., -3., 5.), [16., 8.], 24.);
    let read = import_step_nurbs_frustum_translated_mm(
        &represented.export_step_mm(policy()).unwrap(),
        policy(),
    )
    .unwrap();
    let a = read.mass_properties(policy()).unwrap();
    let b = represented.mass_properties(policy()).unwrap();
    assert_eq!(a.volume, b.volume);
    assert_eq!(a.centroid, b.centroid);
    assert_eq!(
        read.inertia_properties(policy()).unwrap().inertia,
        represented.inertia_properties(policy()).unwrap().inertia
    );
    assert_eq!(
        format!("{:?}", read.tessellate(0.2, policy()).unwrap()),
        format!("{:?}", represented.tessellate(0.2, policy()).unwrap())
    );
}

#[test]
fn one_ulp_retained_weight_and_world_control_corruption_rejects() {
    let text = source(Vec3::new(12., -3., 5.), [16., 8.], 24.)
        .export_step_mm(policy())
        .unwrap();
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    let line = lines.iter_mut().find(|s| s.contains("=VECTOR(")).unwrap();
    let start = line.rfind(',').unwrap() + 1;
    let end = start + line[start..].find(')').unwrap();
    let value: f64 = line[start..end].parse().unwrap();
    line.replace_range(start..end, &f64::from_bits(value.to_bits() + 1).to_string());
    assert!(import_step_nurbs_frustum_translated_mm(&lines.join("\n"), policy()).is_err());
    for marker in ["RATIONAL_B_SPLINE_SURFACE((", "CARTESIAN_POINT('',("] {
        let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
        let line = lines.iter_mut().find(|s| s.contains(marker)).unwrap();
        let mut start = line.find(marker).unwrap() + marker.len();
        while line.as_bytes()[start] == b'(' {
            start += 1;
        }
        let end = start + line[start..].find([',', ')']).unwrap();
        let value: f64 = line[start..end].parse().unwrap();
        let changed = f64::from_bits(value.to_bits() + 1);
        line.replace_range(start..end, &changed.to_string());
        assert!(
            import_step_nurbs_frustum_translated_mm(&lines.join("\n"), policy()).is_err(),
            "{marker}"
        );
    }
}
