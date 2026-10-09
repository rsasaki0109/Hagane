use hagane::*;
use std::collections::HashMap;
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
fn fixture(dimensions: [f64; 3], b: f64, hole: [[f64; 2]; 2]) -> NurbsGraphHoledSolid {
    let graph = NurbsGraphSolid::new(dimensions, b, Tolerance::default()).unwrap();
    NurbsGraphHoledSolid::new(&graph, hole, Tolerance::default()).unwrap()
}
fn reordered_loops(step: &str) -> String {
    let inner: Vec<usize> = step
        .lines()
        .filter_map(|line| {
            line.split_once("FACE_BOUND('',#")
                .map(|(_, tail)| tail.split(',').next().unwrap().parse().unwrap())
        })
        .collect();
    step.lines()
        .map(|line| {
            if let Some((head, tail)) = line.split_once("EDGE_LOOP('',(") {
                let id = head
                    .trim_start_matches('#')
                    .split('=')
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                if inner.contains(&id) {
                    let mut edges: Vec<_> = tail.strip_suffix("));").unwrap().split(',').collect();
                    edges.rotate_left(1);
                    return format!("{head}EDGE_LOOP('',({}));", edges.join(","));
                }
            }
            if let Some((head, tail)) = line.split_once("CLOSED_SHELL('',(") {
                let faces: Vec<_> = tail.strip_suffix("));").unwrap().split(',').rev().collect();
                return format!("{head}CLOSED_SHELL('',({}));", faces.join(","));
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn actual_holed_brep_roundtrips_signed_flat_and_small_coefficients_exactly() {
    for (dims, b, hole) in [
        ([80., 60., 20.], 30., [[0.35, 0.65], [0.35, 0.65]]),
        ([20., 12., 3.], -2., [[0.35, 0.5], [0.25, 0.4]]),
        ([80., 60., 20.], 1e-3, [[0.35, 0.65], [0.35, 0.65]]),
        ([20., 12., 3.], 0., [[0.25, 0.75], [0.25, 0.75]]),
    ] {
        let source = fixture(dims, b, hole);
        let step = source.export_step_mm(Tolerance::default()).unwrap();
        let imported = import_step_nurbs_graph_holed_mm(&step, Tolerance::default()).unwrap();
        assert_eq!(imported.export_step_mm(Tolerance::default()).unwrap(), step);
        assert_eq!(imported.hole(), hole);
        assert_eq!(imported.source().dimensions(), dims);
        assert!(
            (imported.volume().unwrap() - source.volume().unwrap()).abs()
                < 1e-10 * source.volume().unwrap().max(1.)
        );
        assert_eq!(imported.solid.vertices.len(), 16);
        assert_eq!(imported.solid.edges.len(), 24);
        assert_eq!(
            imported
                .solid
                .shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>()
                + 16
                - 24,
            0
        );
        let tolerance = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        let u = (hole[0][0] + hole[0][1]) / 2.;
        let v = (hole[1][0] + hole[1][1]) / 2.;
        assert_eq!(
            imported
                .classify_point(Point3::new(dims[0] * u, dims[1] * v, 1.), tolerance)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            imported
                .classify_point(Point3::new(dims[0] * 0.1, dims[1] * 0.1, 1.), tolerance)
                .unwrap(),
            PointLocation::Inside
        );
        let display = imported
            .tessellate_bounded(0.2, 65536, Tolerance::default())
            .unwrap();
        let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
        for ids in &display.mesh.triangles {
            for i in 0..3 {
                let a = display.vertex_nodes[ids[i]];
                let b = display.vertex_nodes[ids[(i + 1) % 3]];
                let edge = edges.entry((a.min(b), a.max(b))).or_default();
                edge.0 += 1;
                edge.1 += if a < b { 1 } else { -1 };
            }
        }
        for edge in edges.values() {
            assert_eq!(*edge, (2, 0));
        }
    }
}
#[test]
fn arbitrary_ids_record_order_cap_wire_start_face_order_and_metre_units_preserve_body() {
    let source = fixture([80., 60., 20.], 30., [[0.35, 0.65], [0.35, 0.65]]);
    let original = source.export_step_mm(Tolerance::default()).unwrap();
    for step in [
        permute(&original),
        reordered_loops(&original),
        original.replace("Hagane", "Independent file"),
    ] {
        let imported = import_step_nurbs_graph_holed_mm(&step, Tolerance::default()).unwrap();
        assert_eq!(
            imported.export_step_mm(Tolerance::default()).unwrap(),
            original
        );
    }
    let source = fixture([1024., 512., 128.], 0., [[0.25, 0.75], [0.25, 0.75]]);
    let original = source.export_step_mm(Tolerance::default()).unwrap();
    let imported =
        import_step_nurbs_graph_holed_mm(&metres(&original), Tolerance::default()).unwrap();
    assert_eq!(
        imported.export_step_mm(Tolerance::default()).unwrap(),
        original
    );
}
#[test]
fn noncanonical_subtolerance_controls_pcurves_missing_walls_and_unsupported_scope_reject() {
    let tol = Tolerance::default();
    let source = fixture([80., 60., 20.], 30., [[0.35, 0.65], [0.35, 0.65]]);
    let original = source.export_step_mm(tol).unwrap();
    let mut changed = false;
    let crooked = original
        .lines()
        .map(|line| {
            if !changed && line.contains("CARTESIAN_POINT('',(") {
                let (head, tail) = line.split_once("CARTESIAN_POINT('',(").unwrap();
                let mut xyz: Vec<f64> = tail
                    .strip_suffix("));")
                    .unwrap()
                    .split(',')
                    .map(|x| x.parse().unwrap())
                    .collect();
                if xyz.len() == 3 && xyz[0] == 0. && xyz[1] == 0. && xyz[2] == 0. {
                    changed = true;
                    xyz[0] += tol.linear / 2.;
                    return format!(
                        "{head}CARTESIAN_POINT('',({},{},{}));",
                        xyz[0], xyz[1], xyz[2]
                    );
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(changed);
    assert!(import_step_nurbs_graph_holed_mm(&crooked, tol).is_err());
    let pcurve = original.replacen("DIRECTION('',(1.,0.))", "DIRECTION('',(0.,1.))", 1);
    assert_ne!(pcurve, original);
    assert!(import_step_nurbs_graph_holed_mm(&pcurve, tol).is_err());
    let missing = original
        .lines()
        .map(|line| {
            if let Some((head, tail)) = line.split_once("CLOSED_SHELL('',(") {
                let mut faces: Vec<_> = tail.strip_suffix("));").unwrap().split(',').collect();
                faces.pop();
                return format!("{head}CLOSED_SHELL('',({}));", faces.join(","));
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(import_step_nurbs_graph_holed_mm(&missing, tol).is_err());
    let graph = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    for graph in [
        graph.trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol).unwrap(),
        graph
            .transformed(Transform::translation(Vec3::new(1., 2., 3.)).unwrap(), tol)
            .unwrap(),
    ] {
        let holed = NurbsGraphHoledSolid::new(&graph, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
        assert!(
            import_step_nurbs_graph_holed_mm(&holed.export_step_mm(tol).unwrap(), tol).is_err()
        );
    }
    for bad in [
        original.replacen(
            "B_SPLINE_SURFACE_WITH_KNOTS",
            "RATIONAL_B_SPLINE_SURFACE",
            1,
        ),
        original.replacen("CLOSED_SHELL('',(", "CLOSED_SHELL('',(#999999,", 1),
        "x".repeat(1024 * 1024 + 1),
    ] {
        assert!(import_step_nurbs_graph_holed_mm(&bad, tol).is_err());
    }
    assert!(import_step_nurbs_graph_holed_mm(&original, tol).is_ok());
}
#[test]
fn tiny_ambiguous_recipe_coefficients_never_change_encoded_geometry() {
    for b in [1e-12, 1e-14] {
        let graph = fixture([80., 60., 20.], b, [[0.35, 0.65], [0.35, 0.65]]);
        let step = graph.export_step_mm(Tolerance::default()).unwrap();
        match import_step_nurbs_graph_holed_mm(&step, Tolerance::default()) {
            Ok(imported) => {
                assert_eq!(imported.export_step_mm(Tolerance::default()).unwrap(), step)
            }
            Err(Error::Unsupported(_)) => {}
            Err(error) => panic!("unexpected {error:?}"),
        }
    }
}
