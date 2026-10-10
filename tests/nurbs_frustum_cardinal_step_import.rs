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
fn check(body: &NurbsFrustumSolid) {
    let step = body.export_step_mm(policy()).unwrap();
    let read = import_step_nurbs_frustum_cardinal_mm(&step, policy()).unwrap();
    assert_eq!(read.export_step_mm(policy()).unwrap(), step);
    assert_eq!(format!("{:?}", read.solid()), format!("{:?}", body.solid()));
    assert_eq!(read.radii(), body.radii());
    assert_eq!(read.frame().axes(), body.frame().axes());
    assert_eq!(
        read.mass_properties(policy()).unwrap().centroid,
        body.mass_properties(policy()).unwrap().centroid
    );
    assert_eq!(
        read.inertia_properties(policy()).unwrap().inertia,
        body.inertia_properties(policy()).unwrap().inertia
    );
}

#[test]
fn all_twenty_four_actual_cardinal_frames_and_partition_children() {
    for frame in frames() {
        let body = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy()).unwrap();
        check(&body);
        check(&body.split_axial(9., policy()).unwrap().upper);
        for part in body
            .split_axial_many(&[4., 10., 18.], policy())
            .unwrap()
            .parts
        {
            check(&part);
        }
        let step = body.export_step_mm(policy()).unwrap();
        assert!(import_step_nurbs_frustum_mm(&step, policy()).is_err());
        if frame.axes() != Frame3::IDENTITY.axes() {
            assert!(import_step_nurbs_frustum_translated_mm(&step, policy()).is_err());
        }
    }
}

fn refs(line: &str) -> Vec<usize> {
    let mut result = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().unwrap());
            }
            result.push(digits.parse().unwrap());
        }
    }
    result
}
#[test]
fn raw_cap_placement_axes_are_not_normalized_into_acceptance() {
    let body = NurbsFrustumSolid::new(frames().remove(0), [16., 8.], 24., policy()).unwrap();
    let step = body.export_step_mm(policy()).unwrap();
    let placement = step
        .lines()
        .find(|line| line.contains("=AXIS2_PLACEMENT_3D("))
        .unwrap();
    let ids = refs(placement);
    // Record label, origin, normal direction, and reference direction.
    for id in [ids[2], ids[3]] {
        let mut lines: Vec<_> = step.lines().map(str::to_owned).collect();
        let line = lines
            .iter_mut()
            .find(|line| line.starts_with(&format!("#{id}=")))
            .unwrap();
        let start = line.find("DIRECTION('',(").unwrap() + "DIRECTION('',(".len();
        let end = start + line[start..].find(')').unwrap();
        let mut values: Vec<f64> = line[start..end]
            .split(',')
            .map(|s| s.parse().unwrap())
            .collect();
        let value = values.iter_mut().find(|v| **v != 0.).unwrap();
        *value = f64::from_bits(value.to_bits() + 1);
        line.replace_range(
            start..end,
            &values
                .iter()
                .map(f64::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
        assert!(import_step_nurbs_frustum_cardinal_mm(&lines.join("\n"), policy()).is_err());
    }
    let top = step
        .lines()
        .filter(|line| line.contains("=AXIS2_PLACEMENT_3D("))
        .nth(1)
        .unwrap();
    let top_id = refs(top)[1];
    let mut lines: Vec<_> = step.lines().map(str::to_owned).collect();
    let line = lines
        .iter_mut()
        .find(|line| line.starts_with(&format!("#{top_id}=")))
        .unwrap();
    let start = line.find("CARTESIAN_POINT('',(").unwrap() + "CARTESIAN_POINT('',(".len();
    let end = start + line[start..].find(')').unwrap();
    let mut values: Vec<f64> = line[start..end]
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let axis = body.frame().axes()[2];
    let component = [axis.x, axis.y, axis.z]
        .iter()
        .position(|v| *v == 0.)
        .unwrap();
    values[component] = f64::from_bits(values[component].to_bits() + 1);
    line.replace_range(
        start..end,
        &values
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(","),
    );
    assert!(import_step_nurbs_frustum_cardinal_mm(&lines.join("\n"), policy()).is_err());
}

#[test]
fn mesh_equivalence_and_general_rotation_precision_refusal() {
    let body = NurbsFrustumSolid::new(frames().remove(0), [16., 8.], 24., policy()).unwrap();
    let read =
        import_step_nurbs_frustum_cardinal_mm(&body.export_step_mm(policy()).unwrap(), policy())
            .unwrap();
    assert_eq!(
        format!("{:?}", read.tessellate(0.2, policy()).unwrap()),
        format!("{:?}", body.tessellate(0.2, policy()).unwrap())
    );
    let strict = GeometryTolerance::new(1e-12, 1e-10, 0.).unwrap();
    assert!(matches!(
        import_step_nurbs_frustum_cardinal_mm(&body.export_step_mm(policy()).unwrap(), strict),
        Err(Error::Unsupported(_))
    ));
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.2).unwrap())
        .unwrap();
    let rotated = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy()).unwrap();
    assert!(import_step_nurbs_frustum_cardinal_mm(
        &rotated.export_step_mm(policy()).unwrap(),
        policy()
    )
    .is_err());
    check(&body);
}
