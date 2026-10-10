use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn frames() -> Vec<Frame3> {
    let basis = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut frames = Vec::new();
    for permutation in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    let axes = [
                        basis[permutation[0]] * x,
                        basis[permutation[1]] * y,
                        basis[permutation[2]] * z,
                    ];
                    if axes[0].cross(axes[1]).dot(axes[2]) > 0. {
                        frames.push(
                            Frame3::new_with_tolerance(Point3::new(12., -3., 5.), axes, policy())
                                .unwrap(),
                        );
                    }
                }
            }
        }
    }
    assert_eq!(frames.len(), 24);
    frames
}
#[test]
fn exact_cardinal_bored_coefficients_and_partial_children_roundtrip() {
    for frame in frames() {
        for radii in [[16., 8.], [8., 16.], [12., 12.]] {
            let source = NurbsFrustumSolid::new(frame, radii, 24., policy()).unwrap();
            let bore = source.bore_coaxial(4., policy()).unwrap();
            let parts = bore.split_axial_many(&[6., 12., 18.], policy()).unwrap();
            for original in std::iter::once(&bore).chain(parts.parts.iter()) {
                let text = original.export_step_mm(policy()).unwrap();
                let imported =
                    import_step_nurbs_bored_frustum_cardinal_mm(&text, policy()).unwrap();
                assert_eq!(
                    format!("{:?}", imported.solid()),
                    format!("{:?}", original.solid())
                );
                assert_eq!(imported.export_step_mm(policy()).unwrap(), text);
                assert_eq!(
                    imported.mass_properties(policy()).unwrap().centroid,
                    original.mass_properties(policy()).unwrap().centroid
                );
                assert!(import_step_nurbs_frustum_cardinal_mm(&text, policy()).is_err());
            }
        }
    }
}
#[test]
fn cyclic_annular_wires_reordered_shell_preserve_actual_body() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy()).unwrap();
    let original = source.bore_coaxial(4., policy()).unwrap();
    let text = original.export_step_mm(policy()).unwrap();
    let changed = text
        .lines()
        .map(|line| {
            for kind in ["EDGE_LOOP", "CLOSED_SHELL", "ADVANCED_FACE"] {
                let prefix = if kind == "ADVANCED_FACE" {
                    "ADVANCED_FACE('',(".to_string()
                } else {
                    format!("{kind}('',(")
                };
                if let Some(begin) = line.find(&prefix) {
                    let begin = begin + prefix.len();
                    let end = begin + line[begin..].find(')').unwrap();
                    let mut ids: Vec<_> = line[begin..end].split(',').collect();
                    if kind == "EDGE_LOOP" {
                        ids.rotate_left(1);
                    } else {
                        ids.reverse();
                    }
                    return format!("{}{}{}", &line[..begin], ids.join(","), &line[end..]);
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let imported = import_step_nurbs_bored_frustum_cardinal_mm(&changed, policy()).unwrap();
    assert_eq!(imported.export_step_mm(policy()).unwrap(), text);
}
#[test]
fn actual_serialized_parameter_and_cap_role_corruption_reject() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy()).unwrap();
    let text = source
        .bore_coaxial(4., policy())
        .unwrap()
        .export_step_mm(policy())
        .unwrap();
    let vector = text.lines().find(|line| line.contains("=VECTOR(")).unwrap();
    let begin = vector.rfind(',').unwrap() + 1;
    let end = vector.rfind(')').unwrap();
    let value: f64 = vector[begin..end].parse().unwrap();
    let altered = format!(
        "{}{}{}",
        &vector[..begin],
        f64::from_bits(value.to_bits() + 1),
        &vector[end..]
    );
    assert!(
        import_step_nurbs_bored_frustum_cardinal_mm(&text.replace(vector, &altered), policy())
            .is_err()
    );
    let bound = text
        .lines()
        .find(|line| line.contains("=FACE_BOUND("))
        .unwrap();
    let altered = bound.replace(".T.", ".F.");
    assert!(
        import_step_nurbs_bored_frustum_cardinal_mm(&text.replace(bound, &altered), policy())
            .is_err()
    );
    assert!(import_step_nurbs_bored_frustum_cardinal_mm(
        &text.replace(".MILLI.,.METRE.", "$,.METRE."),
        policy()
    )
    .is_err());
    let rotated = NurbsFrustumSolid::new(
        Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(),
        [16., 8.],
        24.,
        policy(),
    )
    .unwrap()
    .bore_coaxial(4., policy())
    .unwrap();
    assert!(import_step_nurbs_bored_frustum_cardinal_mm(
        &rotated.export_step_mm(policy()).unwrap(),
        policy()
    )
    .is_err());
}
