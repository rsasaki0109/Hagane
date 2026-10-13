use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
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

fn check(body: &NurbsTorusSolid, p: GeometryTolerance, s: f64) {
    let original = format!("{:?}", body.solid());
    let step = body.export_step_mm(p).unwrap();
    let read = import_step_nurbs_torus_cardinal_mm(&step, p)
        .unwrap_or_else(|e| panic!("radii {:?} frame {:?}: {e:?}", body.radii(), body.frame()));
    read.validate(p).unwrap();
    assert_eq!(format!("{:?}", read.solid()), original);
    assert_eq!(read.export_step_mm(p).unwrap(), step);
    let mass = read.mass_properties(p).unwrap();
    let expected = body.inertia_properties(p).unwrap();
    let inertia = read.inertia_properties(p).unwrap();
    assert!((mass.volume - expected.volume).abs() < expected.volume * 1e-12);
    assert!((mass.centroid - expected.centroid).norm() < 1e-10 * s);
    let envelope = expected
        .inertia
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0., f64::max);
    for (a, b) in inertia
        .inertia
        .iter()
        .flatten()
        .zip(expected.inertia.iter().flatten())
    {
        assert!((a - b).abs() < envelope * 1e-12);
    }
    let frame = read.frame();
    let [major, minor] = read.radii();
    assert_eq!(
        read.classify_point(frame.point(Point3::new(0., 0., 0.)), p)
            .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        read.classify_point(frame.point(Point3::new(major, 0., 0.)), p)
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        read.classify_point(frame.point(Point3::new(major + minor, 0., 0.)), p)
            .unwrap(),
        PointLocation::Boundary
    );
    for face in &read.solid().shell.faces {
        let Surface::Nurbs(net) = &face.surface else {
            panic!("not rational")
        };
        for u in [0., 0.31, 1.] {
            for v in [0., 0.67, 1.] {
                let x = frame.local_point(net.evaluate(u, v).unwrap());
                assert!(((x.x.hypot(x.y) - major).hypot(x.z) - minor).abs() < 1e-8 * s);
            }
        }
    }
    let mesh = read.tessellate(0.5 * s, p).unwrap();
    assert!(mesh.signed_volume() > 0.);
    assert!(mesh.positions.iter().all(|p| p.finite()));
    assert_eq!(original, format!("{:?}", body.solid()));
    assert_eq!(step, body.export_step_mm(p).unwrap());
}
#[test]
fn all_signed_cardinal_translations_and_scaled_rational_torus_breps_roundtrip_exactly() {
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
                    let frame =
                        Frame3::new_with_tolerance(Point3::new(12., -3., 5.), axes, p).unwrap();
                    check(&NurbsTorusSolid::new(frame, [16., 4.], p).unwrap(), p, 1.);
                }
            }
        }
    }
    assert_eq!(count, 24);
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let frame = Frame3::new_with_tolerance(Point3::new(12., -3., 5.) * s, basis, p).unwrap();
        check(
            &NurbsTorusSolid::new(frame, [16. * s, 4. * s], p).unwrap(),
            p,
            s,
        );
    }
}
#[test]
fn arbitrary_entity_identifiers_and_record_order_do_not_change_actual_geometry() {
    let p = policy(1.);
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let text = body.export_step_mm(p).unwrap();
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    for line in &mut lines {
        if let Some(begin) = line.find("EDGE_LOOP('',(") {
            let a = begin + "EDGE_LOOP('',(".len();
            let b = line[a..].find(')').unwrap() + a;
            let mut refs: Vec<_> = line[a..b].split(',').map(str::to_owned).collect();
            refs.rotate_left(1);
            line.replace_range(a..b, &refs.join(","));
        }
    }

    let indices: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| l.starts_with('#').then_some(i))
        .collect();
    let reversed: Vec<_> = indices.iter().rev().map(|&i| lines[i].clone()).collect();
    for (i, line) in indices.into_iter().zip(reversed) {
        lines[i] = line;
    }
    let joined = lines.join("\n");
    let mut chars = joined.chars().peekable();
    let mut remapped = String::new();
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
    let read = import_step_nurbs_torus_cardinal_mm(&remapped, p).unwrap();
    assert_eq!(read.export_step_mm(p).unwrap(), text);
    assert_eq!(format!("{:?}", read.solid()), format!("{:?}", body.solid()));
}
#[test]
fn changed_raw_coefficients_topology_units_and_other_shapes_refuse_without_repair() {
    let p = policy(1.);
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let text = body.export_step_mm(p).unwrap();
    for (marker, last) in [
        ("RATIONAL_B_SPLINE_SURFACE((", false),
        ("RATIONAL_B_SPLINE_CURVE((", false),
        ("B_SPLINE_CURVE_WITH_KNOTS(", true),
        ("CARTESIAN_POINT('',(", false),
        ("VECTOR('", true),
    ] {
        let changed = changed_scalar(&text, marker, last);
        assert_ne!(changed, text);
        assert!(
            import_step_nurbs_torus_cardinal_mm(&changed, p).is_err(),
            "accepted {marker}"
        );
    }
    for marker in [
        "=EDGE_CURVE(",
        "=ORIENTED_EDGE(",
        "=ADVANCED_FACE(",
        "=FACE_OUTER_BOUND(",
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
            import_step_nurbs_torus_cardinal_mm(&lines.join("\n"), p).is_err(),
            "accepted {marker}"
        );
    }
    let extra = text.replace(
        "ENDSEC;\nEND-ISO",
        "#999999=CARTESIAN_POINT('',(1.,2.,3.));\nENDSEC;\nEND-ISO",
    );
    assert!(import_step_nurbs_torus_cardinal_mm(&extra, p).is_err());
    assert!(
        import_step_nurbs_torus_cardinal_mm(&text.replace(".MILLI.,.METRE.", "$,.METRE."), p)
            .is_err()
    );
    for bad in ["", "ISO-10303-21;", "#1=NAN;"] {
        assert!(import_step_nurbs_torus_cardinal_mm(bad, p).is_err());
    }
    let rotated = NurbsTorusSolid::new(
        Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap(),
        [16., 4.],
        p,
    )
    .unwrap();
    assert!(import_step_nurbs_torus_cardinal_mm(&rotated.export_step_mm(p).unwrap(), p).is_err());
    let child = body.split_by_local_z(1.25, p).unwrap().lower;
    assert!(import_step_nurbs_torus_cardinal_mm(&child.export_step_mm(p).unwrap(), p).is_err());
    let unresolved = GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap();
    assert!(import_step_nurbs_torus_cardinal_mm(&text, unresolved).is_err());
    check(&body, p, 1.);
}

#[test]
fn ambiguous_nominal_radius_preimages_preserve_actual_step_geometry_deterministically() {
    let p = policy(1.);
    let axes = [
        Vec3::new(0., -1., 0.),
        Vec3::new(0., 0., -1.),
        Vec3::new(1., 0., 0.),
    ];
    let frame = Frame3::new_with_tolerance(Point3::new(12., -3., 5.), axes, p).unwrap();
    let body = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
    let text = body.export_step_mm(p).unwrap();
    let a = import_step_nurbs_torus_cardinal_mm(&text, p).unwrap();
    let b = import_step_nurbs_torus_cardinal_mm(&text, p).unwrap();
    assert_eq!(format!("{:?}", a), format!("{:?}", b));
    assert_eq!(format!("{:?}", a.solid()), format!("{:?}", body.solid()));
    assert_eq!(a.export_step_mm(p).unwrap(), text);
    check(&body, p, 1.);
}
