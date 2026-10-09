use hagane::*;
fn fixture(b: f64) -> NurbsGraphHoledSolid {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], b, t).unwrap();
    NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], t).unwrap()
}
fn fields(input: &str) -> Vec<String> {
    let text = input.strip_prefix('(').unwrap().strip_suffix(')').unwrap();
    let mut depth = 0;
    let mut start = 0;
    let mut result = Vec::new();
    for (i, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(text[start..i].to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(text[start..].to_owned());
    result
}
fn replace_entity(step: &str, id: usize, replacement: &str) -> String {
    let prefix = format!("#{id}=");
    assert_eq!(step.lines().filter(|l| l.starts_with(&prefix)).count(), 1);
    step.lines()
        .map(|l| {
            if l.starts_with(&prefix) {
                format!("{prefix}{replacement};")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn candidate_prefilter_preserves_actual_full_brep_for_all_browser_coefficients() {
    let t = Tolerance::default();
    for b in [-12., 0.001, 30., 36., 0.] {
        let original = fixture(b);
        let step = original.export_step_mm(t).unwrap();
        let imported = import_step_nurbs_graph_holed_mm(&step, t).unwrap();
        assert_eq!(imported.export_step_mm(t).unwrap(), step, "bulge {b}");
        // Several recipe coefficients can encode the same rounded h+b. The
        // actual full B-rep, rather than the recovered recipe bits, is authoritative.
    }
}
#[test]
fn lost_recipe_rejection_and_representable_tiny_source_coefficients_stay_explicit() {
    let t = Tolerance::default();
    // An entirely lost nonzero recipe is already rejected by construction.
    assert!(NurbsGraphSolid::new([80., 60., 20.], 1e-30, t).is_err());
    for b in [f64::from_bits(20f64.to_bits() + 1) - 20., 1e-12] {
        let source = NurbsGraphSolid::new([80., 60., 20.], b, t).unwrap();
        let Surface::Nurbs(roof) = &source.brep().shell.faces[1].surface else {
            panic!()
        };
        assert_eq!(roof.control_points()[4].z.to_bits(), (20. + b).to_bits());
        assert!(roof.control_points()[4].z > 20.);
        // Import classification for these bounded-recognition edge cases is
        // covered by the existing baseline suite, not a permissive new oracle.
    }
}
#[test]
fn roof_controls_and_knots_cannot_pass_an_approximate_candidate_prefilter() {
    let t = Tolerance::default();
    let step = fixture(30.).export_step_mm(t).unwrap();
    // The upper cap is the second 7x7 standalone unit-weight surface. Read its
    // actual referenced CV instead of changing an unrelated first vertex.
    let caps: Vec<_> = step
        .lines()
        .filter(|l| l.contains("=B_SPLINE_SURFACE_WITH_KNOTS("))
        .filter(|l| {
            let args = fields(
                l.split_once("B_SPLINE_SURFACE_WITH_KNOTS")
                    .unwrap()
                    .1
                    .trim_end_matches(';'),
            );
            args[3].matches('#').count() == 49
        })
        .collect();
    assert_eq!(caps.len(), 2);
    let line = caps[1];
    let args = fields(
        line.split_once("B_SPLINE_SURFACE_WITH_KNOTS")
            .unwrap()
            .1
            .trim_end_matches(';'),
    );
    let ids: Vec<usize> = args[3]
        .split('#')
        .skip(1)
        .map(|p| {
            p.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap()
        })
        .collect();
    let id = ids[24];
    let prefix = format!("#{id}=CARTESIAN_POINT");
    let point = step.lines().find(|l| l.starts_with(&prefix)).unwrap();
    let args = fields(
        point
            .split_once("CARTESIAN_POINT")
            .unwrap()
            .1
            .trim_end_matches(';'),
    );
    let mut xyz: Vec<f64> = fields(&args[1])
        .iter()
        .map(|x| x.parse().unwrap())
        .collect();
    xyz[2] += t.linear / 100.;
    let changed = replace_entity(
        &step,
        id,
        &format!("CARTESIAN_POINT('',({},{},{}))", xyz[0], xyz[1], xyz[2]),
    );
    assert!(import_step_nurbs_graph_holed_mm(&changed, t).is_err());
    let surface_id: usize = line[1..].split('=').next().unwrap().parse().unwrap();
    let mut args = fields(
        line.split_once("B_SPLINE_SURFACE_WITH_KNOTS")
            .unwrap()
            .1
            .trim_end_matches(';'),
    );
    let mut knot = fields(&args[10]);
    let x: f64 = knot[1].parse().unwrap();
    knot[1] = f64::from_bits(x.to_bits() + 1).to_string();
    args[10] = format!("({})", knot.join(","));
    let changed = replace_entity(
        &step,
        surface_id,
        &format!("B_SPLINE_SURFACE_WITH_KNOTS({})", args.join(",")),
    );
    assert!(import_step_nurbs_graph_holed_mm(&changed, t).is_err());
}

#[test]
fn true_complex_rational_surface_with_one_ulp_weight_remains_outside_strict_import_subset() {
    let t = Tolerance::default();
    let step = fixture(30.).export_step_mm(t).unwrap();
    let line = step
        .lines()
        .filter(|l| l.contains("=B_SPLINE_SURFACE_WITH_KNOTS("))
        .find(|l| {
            fields(
                l.split_once("B_SPLINE_SURFACE_WITH_KNOTS")
                    .unwrap()
                    .1
                    .trim_end_matches(';'),
            )[3]
            .matches('#')
            .count()
                == 49
        })
        .unwrap();
    let id: usize = line[1..].split('=').next().unwrap().parse().unwrap();
    let a = fields(
        line.split_once("B_SPLINE_SURFACE_WITH_KNOTS")
            .unwrap()
            .1
            .trim_end_matches(';'),
    );
    let weights = (0..7)
        .map(|i| {
            format!(
                "({})",
                (0..7)
                    .map(|j| if i == 3 && j == 3 {
                        f64::from_bits(1f64.to_bits() + 1).to_string()
                    } else {
                        "1.".into()
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let complex=format!("(BOUNDED_SURFACE() B_SPLINE_SURFACE({}) B_SPLINE_SURFACE_WITH_KNOTS({}) GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_SURFACE(({weights})) REPRESENTATION_ITEM('') SURFACE())",a[1..8].join(","),a[8..13].join(","));
    let changed = replace_entity(&step, id, &complex);
    assert!(matches!(
        import_step_nurbs_graph_holed_mm(&changed, t),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn matching_roof_does_not_skip_final_pcurve_and_topology_validation() {
    let t = Tolerance::default();
    let step = fixture(0.001).export_step_mm(t).unwrap();
    let changed = step.replacen("DIRECTION('',(1.,0.))", "DIRECTION('',(0.,1.))", 1);
    assert_ne!(changed, step);
    assert!(import_step_nurbs_graph_holed_mm(&changed, t).is_err());
    let line = step
        .lines()
        .find(|l| l.contains("=ORIENTED_EDGE("))
        .unwrap();
    let id: usize = line[1..].split('=').next().unwrap().parse().unwrap();
    let mut args = fields(
        line.split_once("ORIENTED_EDGE")
            .unwrap()
            .1
            .trim_end_matches(';'),
    );
    args[4] = if args[4] == ".T." {
        ".F.".into()
    } else {
        ".T.".into()
    };
    let changed = replace_entity(&step, id, &format!("ORIENTED_EDGE({})", args.join(",")));
    assert!(import_step_nurbs_graph_holed_mm(&changed, t).is_err());
}
