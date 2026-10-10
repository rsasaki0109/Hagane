use hagane::*;

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn body(radii: [f64; 2], height: f64, scale: f64) -> NurbsFrustumSolid {
    NurbsFrustumSolid::new(
        Frame3::IDENTITY,
        radii.map(|r| r * scale),
        height * scale,
        policy(scale),
    )
    .unwrap()
}
fn check(source: &NurbsFrustumSolid, p: GeometryTolerance, text: &str) {
    let actual = import_step_nurbs_frustum_mm(text, p).unwrap();
    actual.validate(p).unwrap();
    assert_eq!(actual.radii(), source.radii());
    assert_eq!(actual.height(), source.height());
    assert_eq!(
        actual.export_step_mm(p).unwrap(),
        source.export_step_mm(p).unwrap()
    );
    assert_eq!(actual.volume(p).unwrap(), source.volume(p).unwrap());
}

#[test]
fn exact_taper_cylinder_lower_child_and_scales() {
    for scale in [0.001, 1., 1000.] {
        for radii in [[16., 8.], [8., 16.], [12., 12.]] {
            let source = body(radii, 24., scale);
            let p = policy(scale);
            check(&source, p, &source.export_step_mm(p).unwrap());
            let lower = source.split_axial(9. * scale, p).unwrap().lower;
            check(&lower, p, &lower.export_step_mm(p).unwrap());
        }
    }
}

#[test]
fn entity_order_is_not_geometry() {
    let source = body([16., 8.], 24., 1.);
    let text = source.export_step_mm(policy(1.)).unwrap();
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
    check(&source, policy(1.), &lines.join("\n"));
    let mut remapped = String::new();
    let mut chars = text.chars().peekable();
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
    check(&source, policy(1.), &remapped);
}

// Change one actual serialized scalar, without independently regenerating the
// remaining geometry. Even a sub-tolerance ULP must fail canonical admission.
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
fn raw_coefficients_vectors_units_and_extra_entities_are_rejected() {
    let source = body([16., 8.], 24., 1.);
    let p = policy(1.);
    let text = source.export_step_mm(p).unwrap();
    for (marker, last) in [
        ("RATIONAL_B_SPLINE_SURFACE((", false),
        ("RATIONAL_B_SPLINE_CURVE((", false),
        ("VECTOR('", true),
        ("CARTESIAN_POINT('',(", false),
    ] {
        let bad = changed_scalar(&text, marker, last);
        assert!(import_step_nurbs_frustum_mm(&bad, p).is_err(), "{marker}");
    }
    for marker in ["=EDGE_CURVE(", "=ORIENTED_EDGE(", "=ADVANCED_FACE("] {
        let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
        let line = lines.iter_mut().find(|line| line.contains(marker)).unwrap();
        let end = line.rfind(",.T.)").or_else(|| line.rfind(",.F.)")).unwrap();
        let value = if &line[end..end + 5] == ",.T.)" {
            ",.F.)"
        } else {
            ",.T.)"
        };
        line.replace_range(end..end + 5, value);
        assert!(
            import_step_nurbs_frustum_mm(&lines.join("\n"), p).is_err(),
            "{marker}"
        );
    }
    let extra = text.replace(
        "ENDSEC;\nEND-ISO",
        "#999999=CARTESIAN_POINT('',(1.,2.,3.));\nENDSEC;\nEND-ISO",
    );
    assert!(import_step_nurbs_frustum_mm(&extra, p).is_err());
    let metres = text.replace(".MILLI.,.METRE.", "$,.METRE.");
    assert!(import_step_nurbs_frustum_mm(&metres, p).is_err());
    let posed = NurbsFrustumSolid::new(
        Transform::translation(Vec3::new(1., 0., 0.)).unwrap(),
        [16., 8.],
        24.,
        p,
    )
    .unwrap();
    assert!(import_step_nurbs_frustum_mm(&posed.export_step_mm(p).unwrap(), p).is_err());
    assert!(import_step_nurbs_frustum_mm(&(text.clone() + &text), p).is_err());
    check(&source, p, &text);
}
