use hagane::*;
use std::collections::BTreeMap;
fn fields(input: &str) -> Vec<String> {
    let text = input
        .trim()
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut start = 0;
    for (i, ch) in text.char_indices() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                parts.push(text[start..i].trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim().to_owned());
    parts
}
fn components(input: &str) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::new();
    let mut rest = input
        .trim()
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .trim();
    while !rest.is_empty() {
        let open = rest.find('(').unwrap();
        let kind = rest[..open].trim().to_owned();
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in rest[open..].char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
        }
        result.insert(kind, fields(&rest[open..end]));
        rest = rest[end..].trim();
    }
    result
}
fn records(step: &str) -> BTreeMap<usize, (String, Vec<String>)> {
    step.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('#') {
                return None;
            }
            let (id, body) = line[1..].split_once('=').unwrap();
            let body = body.trim_end_matches(';');
            let decoded = if body.starts_with('(') {
                let c = components(body);
                if let Some(base) = c.get("B_SPLINE_CURVE") {
                    let mut args = vec!["''".into()];
                    args.extend(base.clone());
                    args.extend(c["B_SPLINE_CURVE_WITH_KNOTS"].clone());
                    args.push(c["RATIONAL_B_SPLINE_CURVE"][0].clone());
                    ("B_SPLINE_CURVE_WITH_KNOTS".into(), args)
                } else if c.contains_key("B_SPLINE_SURFACE") {
                    let mut args = vec!["''".into()];
                    args.extend(c["B_SPLINE_SURFACE"].clone());
                    args.extend(c["B_SPLINE_SURFACE_WITH_KNOTS"].clone());
                    args.push(c["RATIONAL_B_SPLINE_SURFACE"][0].clone());
                    ("B_SPLINE_SURFACE_WITH_KNOTS".into(), args)
                } else {
                    ("COMPLEX".into(), c.values().flatten().cloned().collect())
                }
            } else {
                let (kind, args) = body.split_once('(').unwrap();
                (kind.into(), fields(&format!("({args}")))
            };
            Some((id.parse().unwrap(), decoded))
        })
        .collect()
}
fn reference(s: &str) -> usize {
    s.trim_start_matches('#')
        .parse()
        .unwrap_or_else(|_| panic!("bad reference {s:?}"))
}
fn references(s: &str) -> Vec<usize> {
    fields(s).iter().map(|s| reference(s)).collect()
}
fn numbers(s: &str) -> Vec<f64> {
    fields(s).iter().map(|s| s.parse().unwrap()).collect()
}
fn verify(s: &Solid) {
    let t = Tolerance::default();
    let text = export_step_bounded_analytic_mm(s, t.linear).unwrap();
    let r = records(&text);
    let edges: Vec<_> = r.iter().filter(|(_, v)| v.0 == "EDGE_CURVE").collect();
    assert_eq!(edges.len(), s.edges.len());
    for ((_, (_, args)), edge) in edges.iter().zip(&s.edges) {
        assert_eq!(
            args[4] == ".T.",
            !matches!(edge.curve,Curve::Arc{sweep,..} if sweep<0.)
        );
        let sc = &r[&reference(&args[3])];
        let expected_uses: Vec<_> = s
            .shell
            .faces
            .iter()
            .enumerate()
            .flat_map(|(i, f)| {
                f.wires
                    .iter()
                    .flat_map(move |w| w.coedges.iter().map(move |c| (i, c)))
            })
            .filter(|(_, c)| std::ptr::eq(&s.edges[c.edge], edge))
            .collect();
        assert_eq!(
            sc.0,
            if expected_uses[0].0 == expected_uses[1].0 {
                "SEAM_CURVE"
            } else {
                "SURFACE_CURVE"
            }
        );
        let pc = references(&sc.1[2]);
        assert_eq!(pc.len(), 2);
        for (id, (_, c)) in pc.iter().zip(expected_uses) {
            let p = &r[id];
            assert_eq!(p.0, "PCURVE");
            let definition = &r[&reference(&p.1[2])];
            let g = &r[&references(&definition.1[1])[0]];
            for q in [0., 0.17, 0.61, 1.] {
                let domain = edge.curve.range();
                let param = domain[0] + q * (domain[1] - domain[0]);
                let actual = c.pcurve.try_evaluate(param).unwrap();
                let uv = if g.0 == "CIRCLE" {
                    let placement = &r[&reference(&g.1[1])];
                    let center = numbers(&r[&reference(&placement.1[1])].1[1]);
                    let dir = numbers(&r[&reference(&placement.1[2])].1[1]);
                    let phase = dir[1].atan2(dir[0]);
                    let radius: f64 = g.1[2].parse().unwrap();
                    [
                        center[0] + radius * (phase + param).cos(),
                        center[1] + radius * (phase + param).sin(),
                    ]
                } else {
                    assert_eq!(g.0, "LINE");
                    let origin = numbers(&r[&reference(&g.1[1])].1[1]);
                    let vector = &r[&reference(&g.1[2])];
                    let dir = numbers(&r[&reference(&vector.1[1])].1[1]);
                    let len: f64 = vector.1[2].parse().unwrap();
                    let norm = dir[0].hypot(dir[1]);
                    [
                        origin[0] + param * dir[0] / norm * len,
                        origin[1] + param * dir[1] / norm * len,
                    ]
                };
                assert!((uv[0] - actual[0]).hypot(uv[1] - actual[1]) < 1e-10);
            }
        }
    }
    assert!(!text.contains("TRIANGULATED_FACE_SET"));
}
fn ring(center: [f64; 2], radius: f64, start: f64, sign: f64) -> Vec<PlanarSegment> {
    (0..4)
        .map(|i| {
            let phase = start + sign * i as f64 * std::f64::consts::FRAC_PI_2;
            PlanarSegment::Arc {
                center,
                radius,
                start_angle: phase.sin().atan2(phase.cos()),
                sweep: sign * std::f64::consts::FRAC_PI_2,
            }
        })
        .collect()
}
#[test]
fn signed_profile_wrapped_arcs_and_opposed_coedges_preserve_actual_pcurves() {
    let t = Tolerance::default();
    for start in [0., 5.5] {
        let outer = ring([0., 0.], 4., start, 1.);
        let holes = vec![ring([0., 0.], 1., start, -1.)];
        let s = extrude_arc_line_region(
            &ArcLineRegion {
                origin: Point3::new(0., 0., 0.),
                outer,
                holes,
            },
            2.,
            t,
        )
        .unwrap();
        verify(&s);
        assert!(export_step_mm(&s, t).is_err());
        assert!(
            import_step_mm(&export_step_bounded_analytic_mm(&s, t.linear).unwrap(), t).is_err()
        );
        let placed = s
            .transformed(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(), t)
            .unwrap();
        verify(&placed);
    }
}
#[test]
fn old_writer_bytes_and_periodic_seam_remain_unchanged() {
    let t = Tolerance::default();
    let hash = |s: String| {
        s.bytes().fold(14695981039346656037u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(1099511628211)
        })
    };
    let b = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 6., 4.),
        },
        t,
    )
    .unwrap();
    assert_eq!(hash(export_step_mm(&b, t).unwrap()), 7670638184179138552);
    assert_eq!(
        export_step_mm(&b, t).unwrap(),
        export_step_planar_mm(&b, t).unwrap()
    );
    verify(&b);
    let c = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t,
    )
    .unwrap();
    assert_eq!(hash(export_step_mm(&c, t).unwrap()), 11644595882696959320);
    verify(&c);
}
#[test]
fn invalid_negative_long_arc_tampering_and_unresolved_placement_are_rejected() {
    let t = Tolerance::default();
    let source = extrude_arc_line(
        &rounded_rectangle_profile(Point3::new(0., 0., 0.), 12., 10., 1., t).unwrap(),
        2.,
        t,
    )
    .unwrap();
    let index = source
        .edges
        .iter()
        .position(|e| matches!(e.curve, Curve::Arc { .. }))
        .unwrap();
    for sweep in [-1., 4., std::f64::consts::TAU, 0., 1e-20] {
        let mut s = source.clone();
        let Curve::Arc { sweep: actual, .. } = &mut s.edges[index].curve else {
            panic!()
        };
        *actual = sweep;
        assert!(export_step_bounded_analytic_mm(&s, t.linear).is_err());
    }
    let far = source
        .transformed(
            Transform::translation(Vec3::new(1e12, 1e12, 1e12)).unwrap(),
            t,
        )
        .unwrap();
    assert!(export_step_bounded_analytic_mm(&far, t.linear).is_err());
    for linear in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(export_step_bounded_analytic_mm(&source, linear).is_err());
    }
}
