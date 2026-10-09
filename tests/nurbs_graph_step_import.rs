use hagane::*;
fn shuffled(step: &str) -> String {
    let mut result = String::new();
    let bytes = step.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let id = step[start..end].parse::<u32>().unwrap();
            result.push_str(&format!("#{}", 100000 - id * 3));
            i = end;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    let (header, data) = result.split_once("DATA;\n").unwrap();
    let (records, end) = data.split_once("ENDSEC;\nEND-ISO").unwrap();
    let mut rows = records.lines().collect::<Vec<_>>();
    rows.reverse();
    format!("{header}DATA;\n{}\nENDSEC;\nEND-ISO{end}", rows.join("\n"))
}
#[test]
fn imports_actual_basis_without_recipe_and_accepts_renumbered_reordered_entities() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let step = source
        .export_step_mm(tol)
        .unwrap()
        .replace("Hagane", "Untrusted arbitrary product name");
    let actual = import_step_nurbs_graph_mm(&shuffled(&step), tol).unwrap();
    actual.validate(tol).unwrap();
    assert_eq!(actual.dimensions(), [8., 6., 2.]);
    assert_eq!(actual.bulge(), 1.);
    assert_eq!(actual.volume().unwrap(), source.volume().unwrap());
    for (a, b) in actual
        .solid
        .shell
        .faces
        .iter()
        .zip(&source.solid.shell.faces)
    {
        let (Surface::Nurbs(a), Surface::Nurbs(b)) = (&a.surface, &b.surface) else {
            panic!()
        };
        assert_eq!(a.control_points(), b.control_points());
        assert_eq!(a.knots(0).unwrap(), b.knots(0).unwrap());
    }
    assert!(import_step_mm(&step, tol).is_err());
}
#[test]
fn imports_metre_coordinates_to_mm_without_rescaling_uv_knots() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let step = source.export_step_mm(tol).unwrap();
    let metre = step
        .lines()
        .map(|line| {
            if line.contains("=CARTESIAN_POINT(") {
                let (prefix, coords) = line.split_once("('',(").unwrap();
                let values = coords
                    .trim_end_matches("));")
                    .split(',')
                    .map(|x| x.parse::<f64>().unwrap())
                    .collect::<Vec<_>>();
                if values.len() == 3 {
                    return format!(
                        "{prefix}('',({}));",
                        values
                            .iter()
                            .map(|x| format!("{:e}", x * 0.001))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                }
            }
            line.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let imported = import_step_nurbs_graph_mm(&metre, tol).unwrap();
    assert_eq!(imported.dimensions(), source.dimensions());
    assert_eq!(imported.volume().unwrap(), source.volume().unwrap());
}
#[test]
fn rejects_subtolerance_geometry_changes_and_initially_unsupported_scopes() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let step = source.export_step_mm(tol).unwrap();
    let changed = step.replacen(
        "CARTESIAN_POINT('',(0.,0.,0.))",
        "CARTESIAN_POINT('',(0.000000000001,0.,0.))",
        1,
    );
    assert!(import_step_nurbs_graph_mm(&changed, tol).is_err());
    for shape in [
        source
            .transformed(
                Transform::translation(Vec3::new(1e-12, 0., 0.)).unwrap(),
                tol,
            )
            .unwrap(),
        source.trimmed_uv([[0.1, 0.9], [0., 1.]], tol).unwrap(),
    ] {
        assert!(import_step_nurbs_graph_mm(&shape.export_step_mm(tol).unwrap(), tol).is_err());
    }
    let holed = NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.4, 0.6]], tol).unwrap();
    assert!(import_step_nurbs_graph_mm(&holed.export_step_mm(tol).unwrap(), tol).is_err());
    assert!(import_step_nurbs_graph_mm(&"x".repeat(STEP_IMPORT_MAX_BYTES + 1), tol).is_err());
}
