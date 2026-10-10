use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn check(body: &NurbsBoredFrustumSolid, p: GeometryTolerance) {
    let text = body.export_step_mm(p).unwrap();
    let read = import_step_nurbs_bored_frustum_cardinal_mm(&text, p).unwrap();
    read.validate(p).unwrap();
    assert_eq!(format!("{:?}", read.solid()), format!("{:?}", body.solid()));
    assert_eq!(read.export_step_mm(p).unwrap(), text);
    let actual = read.inertia_properties(p).unwrap();
    let expected = body.inertia_properties(p).unwrap();
    assert!((actual.volume - expected.volume).abs() < 1e-12 * expected.volume);
    assert!((read.volume(p).unwrap() - body.volume(p).unwrap()).abs() < 1e-12 * expected.volume);
    let scale = body.source().height().max(body.source().radii()[0]);
    assert!((actual.centroid - expected.centroid).norm() < 1e-12 * scale);
    let mass = read.mass_properties(p).unwrap();
    assert!((mass.volume - expected.volume).abs() < 1e-12 * expected.volume);
    assert!((mass.centroid - expected.centroid).norm() < 1e-12 * scale);
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (actual.inertia[i][j] - expected.inertia[i][j]).abs()
                    < 1e-12 * expected.volume * scale * scale
            );
        }
    }
    let source = read.source();
    let f = source.frame();
    let h = source.height();
    assert_eq!(
        read.classify_point(f.point(Point3::new(0., 0., h / 2.)), p)
            .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        read.classify_point(f.point(Point3::new(read.radius(), 0., h / 2.)), p)
            .unwrap(),
        PointLocation::Boundary
    );
    let mesh = read.tessellate(source.radii()[0] * 0.02, p).unwrap();
    assert!(mesh.signed_volume() > 0.);
    assert!(mesh.positions.iter().all(|v| v.finite()));
}
#[test]
fn all_cardinal_translations_and_scaled_cylinder_taper_partition_children_roundtrip() {
    let basis = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut count = 0;
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
                    if axes[0].cross(axes[1]).dot(axes[2]) < 0. {
                        continue;
                    }
                    count += 1;
                    let p = policy(1.);
                    let f = Frame3::new_with_tolerance(Point3::new(12., -3., 5.), axes, p).unwrap();
                    let body = NurbsFrustumSolid::new(f, [16., 8.], 24., p)
                        .unwrap()
                        .bore_coaxial(4., p)
                        .unwrap();
                    check(&body, p);
                }
            }
        }
    }
    assert_eq!(count, 24);
    for scale in [0.001, 1., 1000.] {
        for radii in [[16., 8.], [8., 16.], [12., 12.]] {
            let p = policy(scale);
            let frame =
                Frame3::new_with_tolerance(Point3::new(12., -3., 5.) * scale, basis, p).unwrap();
            let body = NurbsFrustumSolid::new(frame, radii.map(|v| v * scale), 24. * scale, p)
                .unwrap()
                .bore_coaxial(4. * scale, p)
                .unwrap();
            check(&body, p);
            for part in body
                .split_axial_many(&[6. * scale, 12. * scale, 18. * scale], p)
                .unwrap()
                .parts
            {
                check(&part, p);
            }
        }
    }
}
#[test]
fn entity_ids_and_record_order_do_not_change_actual_geometry() {
    let p = policy(1.);
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let text = body.export_step_mm(p).unwrap();
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    let indices: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.starts_with('#').then_some(i))
        .collect();
    let records: Vec<_> = indices.iter().rev().map(|&i| lines[i].clone()).collect();
    for (i, record) in indices.into_iter().zip(records) {
        lines[i] = record;
    }
    let mut remapped = String::new();
    let joined = lines.join("\n");
    let mut chars = joined.chars().peekable();
    while let Some(c) = chars.next() {
        remapped.push(c);
        if c == '#' {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().unwrap());
            }
            let id: usize = digits.parse().unwrap();
            remapped.push_str(&(id * 7 + 1000).to_string());
        }
    }
    let read = import_step_nurbs_bored_frustum_cardinal_mm(&remapped, p).unwrap();
    assert_eq!(read.export_step_mm(p).unwrap(), text);
    assert_eq!(format!("{:?}", read.solid()), format!("{:?}", body.solid()));
}

fn changed_scalar(text: &str, marker: &str, last: bool) -> String {
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    let line = lines.iter_mut().find(|line| line.contains(marker)).unwrap();
    let begin = line.find(marker).unwrap() + marker.len();
    let tail = &line[begin..];
    let mut ranges = Vec::new();
    let mut start = None;
    for (i, c) in tail
        .char_indices()
        .chain(std::iter::once((tail.len(), ',')))
    {
        if c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E') {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(a) = start.take() {
            if tail[a..i].parse::<f64>().is_ok() {
                ranges.push((a, i));
            }
        }
    }
    let &(a, b) = if last {
        ranges.last().unwrap()
    } else {
        ranges.first().unwrap()
    };
    let value: f64 = tail[a..b].parse().unwrap();
    let next = f64::from_bits(value.to_bits() + 1);
    line.replace_range(begin + a..begin + b, &next.to_string());
    lines.join("\n")
}

#[test]
fn altered_coefficients_topology_units_and_unsupported_bodies_reject_then_recover() {
    let p = policy(1.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let body = source.bore_coaxial(4., p).unwrap();
    let text = body.export_step_mm(p).unwrap();
    for (marker, last) in [
        ("RATIONAL_B_SPLINE_SURFACE((", false),
        ("RATIONAL_B_SPLINE_CURVE((", false),
        ("VECTOR('", true),
        ("CARTESIAN_POINT('',(", false),
    ] {
        assert!(
            import_step_nurbs_bored_frustum_cardinal_mm(&changed_scalar(&text, marker, last), p)
                .is_err(),
            "{marker}"
        );
    }
    for marker in [
        "=EDGE_CURVE(",
        "=ORIENTED_EDGE(",
        "=ADVANCED_FACE(",
        "=FACE_BOUND(",
    ] {
        let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
        let line = lines.iter_mut().find(|l| l.contains(marker)).unwrap();
        let end = line.rfind(",.T.)").or_else(|| line.rfind(",.F.)")).unwrap();
        let value = if &line[end..end + 5] == ",.T.)" {
            ",.F.)"
        } else {
            ",.T.)"
        };
        line.replace_range(end..end + 5, value);
        assert!(
            import_step_nurbs_bored_frustum_cardinal_mm(&lines.join("\n"), p).is_err(),
            "{marker}"
        );
    }
    let extra = text.replace(
        "ENDSEC;\nEND-ISO",
        "#999999=CARTESIAN_POINT('',(1.,2.,3.));\nENDSEC;\nEND-ISO",
    );
    assert!(import_step_nurbs_bored_frustum_cardinal_mm(&extra, p).is_err());
    assert!(import_step_nurbs_bored_frustum_cardinal_mm(
        &text.replace(".MILLI.,.METRE.", "$,.METRE."),
        p
    )
    .is_err());
    let frame = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let posed = NurbsFrustumSolid::new(frame, [16., 8.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    assert!(
        import_step_nurbs_bored_frustum_cardinal_mm(&posed.export_step_mm(p).unwrap(), p).is_err()
    );
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u,
        v: n.cross(u),
    };
    let cut = body.split_by_plane(&plane, p).unwrap();
    assert!(
        import_step_nurbs_bored_frustum_cardinal_mm(&cut.lower.export_step_mm(p).unwrap(), p)
            .is_err()
    );
    assert!(
        import_step_nurbs_bored_frustum_cardinal_mm(&source.export_step_mm(p).unwrap(), p).is_err()
    );
    check(&body, p);
}
