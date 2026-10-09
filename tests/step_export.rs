use hagane::*;
fn topology(step: &str, solid: &Solid) {
    assert!(step.starts_with("ISO-10303-21;"));
    assert!(step.ends_with("END-ISO-10303-21;\n"));
    for (entity, count) in [
        ("VERTEX_POINT", solid.vertices.len()),
        ("EDGE_CURVE", solid.edges.len()),
        ("ADVANCED_FACE", solid.shell.faces.len()),
        ("CLOSED_SHELL", 1),
        ("MANIFOLD_SOLID_BREP", 1),
    ] {
        assert_eq!(
            step.lines()
                .filter(|line| line.contains(&format!("={entity}(")))
                .count(),
            count
        );
    }
    assert!(step.contains("SI_UNIT(.MILLI.,.METRE.)"));
    let records: std::collections::BTreeMap<usize, &str> = step
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            let (id, body) = line.split_once('=').unwrap();
            (id[1..].parse().unwrap(), body)
        })
        .collect();
    fn references(body: &str) -> Vec<usize> {
        body.split('#')
            .skip(1)
            .map(|tail| {
                tail.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap()
            })
            .collect()
    }
    let mut uses = std::collections::BTreeMap::new();
    for face in records
        .values()
        .filter(|body| body.starts_with("ADVANCED_FACE("))
    {
        let sign = if face.ends_with(",.T.);") { 1 } else { -1 };
        let ids = references(face);
        for bound in &ids[..ids.len() - 1] {
            let bound = records[bound];
            assert!(bound.ends_with(",.T.);"));
            let edge_loop = records[&references(bound)[0]];
            for oriented in references(edge_loop) {
                let body = records[&oriented];
                assert!(body.starts_with("ORIENTED_EDGE("));
                let edge = references(body)[0];
                let entry = uses.entry(edge).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += sign * if body.ends_with(",.T.);") { 1 } else { -1 };
            }
        }
    }
    assert_eq!(uses.len(), solid.edges.len());
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
}
#[test]
fn exact_planar_step_preserves_shared_oriented_topology_and_units() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let solid = extrude_polygon(
            &PolygonProfile {
                origin: Point3::new(0., 0., -12. * scale),
                outer: vec![
                    [-40. * scale, -30. * scale],
                    [40. * scale, -30. * scale],
                    [40. * scale, 30. * scale],
                    [-40. * scale, 30. * scale],
                ],
                holes: vec![vec![
                    [-5. * scale, -5. * scale],
                    [5. * scale, -5. * scale],
                    [5. * scale, 5. * scale],
                    [-5. * scale, 5. * scale],
                ]],
            },
            Vec3::new(18. * scale, -12. * scale, 24. * scale),
            t,
        )
        .unwrap();
        let step = export_step_planar_mm(&solid, t).unwrap();
        topology(&step, &solid);
        assert_eq!(step, export_step_planar_mm(&solid, t).unwrap());
        // Every entity reference resolves, and every emitted point round-trips f64.
        let records: Vec<_> = step.lines().filter(|line| line.starts_with('#')).collect();
        for line in &records {
            for reference in line.split('#').skip(1) {
                let digits: String = reference.chars().take_while(char::is_ascii_digit).collect();
                let id: usize = digits.parse().unwrap();
                assert!((1..=records.len()).contains(&id));
            }
        }
        for v in &solid.vertices {
            let expected = format!(
                "({},{},{})",
                decimal(v.point.x),
                decimal(v.point.y),
                decimal(v.point.z)
            );
            assert!(step.contains(&expected));
        }
    }
}
fn decimal(v: f64) -> String {
    let s = v.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.replace('e', "E")
    } else {
        format!("{s}.")
    }
}
#[test]
fn curved_invalid_and_rejected_workflows_never_export_approximation() {
    let t = Tolerance::default();
    let curved = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4.,
            height: 24.,
        },
        t,
    )
    .unwrap();
    assert!(matches!(
        export_step_planar_mm(&curved, t),
        Err(Error::Unsupported(_))
    ));
    let mut invalid = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 24.),
        },
        t,
    )
    .unwrap();
    invalid.shell.faces.pop();
    assert!(export_step_planar_mm(&invalid, t).is_err());
    assert!(export_step_planar_mm(&curved, Tolerance { linear: f64::NAN }).is_err());
    let input = include_str!("../docs/workflow-skew-extrusion-example.json");
    let report: serde_json::Value =
        serde_json::from_str(&export_workflow_step_mm_json(input).unwrap()).unwrap();
    assert_eq!(report["units"], "mm");
    assert_eq!(report["schema"], "AP214");
    assert_eq!(report["faces"], 14);
    assert!(report["step"]
        .as_str()
        .unwrap()
        .contains("MANIFOLD_SOLID_BREP"));
    assert!(
        export_workflow_step_mm_json(include_str!("../docs/workflow-skew-bores-example.json"))
            .is_err()
    );
    assert!(export_workflow_step_mm_json("{").is_err());
    assert!(export_workflow_step_mm_json(&" ".repeat(65537)).is_err());
}
