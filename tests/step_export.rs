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
fn planar_only_api_and_invalid_workflows_never_export_approximation() {
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
            .is_ok()
    );
    assert!(export_workflow_step_mm_json("{").is_err());
    assert!(export_workflow_step_mm_json(&" ".repeat(65537)).is_err());
}

#[test]
fn cylindrical_step_preserves_periodic_seams_and_rigid_placement() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let cylinder = make_cylinder(
            CylinderSpec {
                base: Point3::new(0., 0., -12. * scale),
                radius: 4. * scale,
                height: 24. * scale,
            },
            t,
        )
        .unwrap();
        let tube = make_tube(
            TubeSpec {
                base: Point3::new(0., 0., -12. * scale),
                outer_radius: 8. * scale,
                inner_radius: 4. * scale,
                height: 24. * scale,
            },
            t,
        )
        .unwrap();
        let placed = tube
            .transformed(
                Transform::translation(Vec3::new(20. * scale, -7. * scale, 12. * scale))
                    .unwrap()
                    .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
                    .unwrap(),
                t,
            )
            .unwrap();
        for solid in [cylinder, tube, placed] {
            let step = export_step_mm(&solid, t).unwrap();
            topology(&step, &solid);
            let cylinders = solid
                .shell
                .faces
                .iter()
                .filter(|f| {
                    matches!(
                        f.surface,
                        Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
                    )
                })
                .count();
            assert_eq!(
                step.lines()
                    .filter(|s| s.contains("=CYLINDRICAL_SURFACE("))
                    .count(),
                cylinders
            );
            assert_eq!(
                step.lines().filter(|s| s.contains("=SEAM_CURVE(")).count(),
                cylinders
            );
            assert_eq!(
                step.lines().filter(|s| s.contains("=PCURVE(")).count(),
                2 * cylinders
            );
            assert_eq!(
                step.lines().filter(|s| s.contains("=CIRCLE(")).count(),
                2 * cylinders
            );
            assert_eq!(step, export_step_mm(&solid, t).unwrap());
            assert!(export_step_planar_mm(&solid, t).is_err());
            // Each seam refers to the shared 3D line and two separate UV curves.
            for record in step.lines().filter(|s| s.contains("=SEAM_CURVE(")) {
                let ids: Vec<usize> = record
                    .split_once('=')
                    .unwrap()
                    .1
                    .split('#')
                    .skip(1)
                    .map(|s| {
                        s.chars()
                            .take_while(char::is_ascii_digit)
                            .collect::<String>()
                            .parse()
                            .unwrap()
                    })
                    .collect();
                assert_eq!(ids.len(), 3);
                assert_ne!(ids[1], ids[2]);
                for id in &ids[1..] {
                    assert!(step
                        .lines()
                        .any(|s| s.starts_with(&format!("#{id}=PCURVE("))));
                }
            }
        }
    }
    let t = Tolerance::default();
    for face in [
        BoxFace::MaxX,
        BoxFace::MinX,
        BoxFace::MaxY,
        BoxFace::MinY,
        BoxFace::MaxZ,
        BoxFace::MinZ,
    ] {
        let solid = box_face_blind_bore_demo_solid(face, 4., 8.).unwrap();
        topology(&export_step_mm(&solid, t).unwrap(), &solid);
    }
}

#[test]
fn unsupported_analytic_trims_and_failed_documents_do_not_export() {
    let t = Tolerance::default();
    for solid in [
        framed_arc_extrusion_demo(4.).unwrap(),
        skew_arc_extrusion_demo(4., 8., 24.).unwrap(),
        ellipse_planar_demo_solid(4., 24., 0.3, GeometryTolerance::default()).unwrap(),
    ] {
        assert!(matches!(
            export_step_mm(&solid, t),
            Err(Error::Unsupported(_))
        ));
    }
    let mut doc: serde_json::Value =
        serde_json::from_str(include_str!("../docs/workflow-opposing-blind-example.json")).unwrap();
    let solid_report: serde_json::Value =
        serde_json::from_str(&export_workflow_step_mm_json(&doc.to_string()).unwrap()).unwrap();
    assert_eq!(solid_report["faces"], 18);
    assert_eq!(
        solid_report["step"]
            .as_str()
            .unwrap()
            .matches("=SEAM_CURVE(")
            .count(),
        2
    );
    doc["operations"][2]["depth"] = 16.into();
    assert!(export_workflow_step_mm_json(&doc.to_string()).is_err());
}
