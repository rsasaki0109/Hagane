use hagane::*;
fn graph(b: f64) -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], b, Tolerance::default()).unwrap()
}
fn permute(step: &str) -> String {
    let max = step
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            line[1..]
                .split('=')
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap()
        })
        .max()
        .unwrap();
    let mut mapped = String::new();
    let chars: Vec<_> = step.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let id: usize = chars[start..i].iter().collect::<String>().parse().unwrap();
            mapped.push_str(&format!("#{}", max + 17 - id));
        } else {
            mapped.push(chars[i]);
            i += 1;
        }
    }
    let mut entities: Vec<_> = mapped
        .lines()
        .filter(|line| line.starts_with('#'))
        .collect();
    entities.reverse();
    format!(
        "{}DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;",
        mapped.split("DATA;\n").next().unwrap(),
        entities.join("\n")
    )
}
fn metres(step: &str) -> String {
    let mut output = Vec::new();
    for line in step.lines() {
        if let Some((prefix, values)) = line.split_once("CARTESIAN_POINT('',(") {
            let suffix = values.strip_suffix("));").unwrap();
            let numbers: Vec<f64> = suffix.split(',').map(|v| v.parse().unwrap()).collect();
            if numbers.len() == 3 {
                output.push(format!(
                    "{prefix}CARTESIAN_POINT('',({}));",
                    numbers
                        .iter()
                        .map(|v| (v / 1000.).to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ));
                continue;
            }
        }
        let mut line = line.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        if let Some(start) = line.find("LENGTH_MEASURE(") {
            let start = start + "LENGTH_MEASURE(".len();
            let end = start + line[start..].find(')').unwrap();
            let value = line[start..end].parse::<f64>().unwrap() / 1000.;
            line.replace_range(start..end, &value.to_string());
        }
        output.push(line);
    }
    output.join("\n")
}
#[test]
fn actual_controls_pcurves_and_mesh_roundtrip_even_after_id_record_permutation() {
    for bulge in [-2., 0., 20.] {
        let source = graph(bulge);
        let original = source.export_step_mm(Tolerance::default()).unwrap();
        for step in [
            &original,
            &permute(&original),
            &original.replace("Hagane", "Independent fixture"),
        ] {
            let imported = import_step_nurbs_graph_mm(step, Tolerance::default()).unwrap();
            assert_eq!(imported.dimensions(), [20., 12., 3.]);
            assert_eq!(imported.bulge(), bulge);
            assert_eq!(imported.source_domain(), [[0., 1.], [0., 1.]]);
            assert_eq!(imported.volume().unwrap(), source.volume().unwrap());
            assert_eq!(imported.bounds().unwrap(), source.bounds().unwrap());
            assert_eq!(
                imported.export_step_mm(Tolerance::default()).unwrap(),
                original
            );
            let mesh = imported
                .tessellate_bounded(0.2, 65536, Tolerance::default())
                .unwrap();
            let reference = source
                .tessellate_bounded(0.2, 65536, Tolerance::default())
                .unwrap();
            assert_eq!(mesh.mesh.positions, reference.mesh.positions);
            assert_eq!(mesh.mesh.triangles, reference.mesh.triangles);
            assert_eq!(mesh.vertex_nodes, reference.vertex_nodes);
        }
    }
}
#[test]
fn actual_metre_coordinates_convert_and_unsupported_geometry_is_rejected() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([1024., 512., 128.], 64., tol).unwrap();
    let imported =
        import_step_nurbs_graph_mm(&metres(&source.export_step_mm(tol).unwrap()), tol).unwrap();
    assert_eq!(imported.dimensions(), source.dimensions());
    assert_eq!(imported.bulge(), 64.);
    let source = graph(20.);
    let holed = NurbsGraphHoledSolid::new(&source, [[0.3, 0.6], [0.3, 0.6]], tol).unwrap();
    let trimmed = source.trimmed_uv([[0.2, 0.8], [0.2, 0.8]], tol).unwrap();
    let placed = source
        .transformed(Transform::translation(Vec3::new(1., 2., 3.)).unwrap(), tol)
        .unwrap();
    for step in [
        holed.export_step_mm(tol).unwrap(),
        trimmed.export_step_mm(tol).unwrap(),
        placed.export_step_mm(tol).unwrap(),
    ] {
        assert!(import_step_nurbs_graph_mm(&step, tol).is_err());
    }
}
#[test]
fn corrupt_actual_geometry_orientation_units_and_references_never_snap_to_success() {
    let tol = Tolerance::default();
    let source = graph(-2.);
    let original = source.export_step_mm(tol).unwrap();
    let crooked = original.replace(
        "CARTESIAN_POINT('',(0.,6.,3.))",
        "CARTESIAN_POINT('',(0.,6.,3.000000005))",
    );
    assert_ne!(crooked, original);
    assert!(import_step_nurbs_graph_mm(&crooked, tol).is_err());
    let pcurve = original.replacen("DIRECTION('',(1.,0.))", "DIRECTION('',(0.,1.))", 1);
    assert_ne!(pcurve, original);
    assert!(import_step_nurbs_graph_mm(&pcurve, tol).is_err());
    let orientation = original
        .lines()
        .map(|line| {
            if line.contains("EDGE_CURVE(") {
                line.replace(",.T.);", ",.F.);")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(import_step_nurbs_graph_mm(&orientation, tol).is_err());
    for bad in [
        original.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT(.CENTI.,.METRE.)"),
        original.replacen(
            "B_SPLINE_SURFACE_WITH_KNOTS",
            "RATIONAL_B_SPLINE_SURFACE",
            1,
        ),
        original.replacen("CLOSED_SHELL('',(", "CLOSED_SHELL('',(#999999,", 1),
        original.replacen(
            "CARTESIAN_POINT('',(0.,0.,0.))",
            "CARTESIAN_POINT('',(NaN,0.,0.))",
            1,
        ),
        original.replacen("END-ISO-10303-21;", "", 1),
        "x".repeat(1024 * 1024 + 1),
    ] {
        assert!(import_step_nurbs_graph_mm(&bad, tol).is_err());
    }
    assert!(import_step_nurbs_graph_mm(&original, tol).is_ok());
}
#[test]
fn entity_budget_rejects_many_records_before_geometry_recognition() {
    let source = graph(20.).export_step_mm(Tolerance::default()).unwrap();
    let marker = source.rfind("ENDSEC;").unwrap();
    let additions = (0..32769)
        .map(|i| format!("#{}=LINE('',#1,#2);\n", 100000 + i))
        .collect::<String>();
    let oversized = format!("{}{}{}", &source[..marker], additions, &source[marker..]);
    assert!(oversized.len() < 1024 * 1024);
    assert!(import_step_nurbs_graph_mm(&oversized, Tolerance::default()).is_err());
}
