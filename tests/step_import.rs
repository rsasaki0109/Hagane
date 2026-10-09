use hagane::*;
const TETRA: &str = include_str!("../docs/step-tetrahedron-metres.step");
fn box_solid(scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-4. * scale, -3. * scale, -scale),
            size: Vec3::new(8. * scale, 6. * scale, 2. * scale),
        },
        Tolerance::new(1e-8 * scale).unwrap(),
    )
    .unwrap()
}
fn same_geometry(a: &Solid, b: &Solid, t: Tolerance) {
    b.validate(t).unwrap();
    assert_eq!(a.vertices.len(), b.vertices.len());
    assert_eq!(a.edges.len(), b.edges.len());
    assert_eq!(a.shell.faces.len(), b.shell.faces.len());
    assert!(
        (a.volume().unwrap() - b.volume().unwrap()).abs()
            < t.linear * 100. * (a.bounds().max - a.bounds().min).norm().powi(2)
    );
    for vertex in &a.vertices {
        assert!(b
            .vertices
            .iter()
            .any(|v| (v.point - vertex.point).norm() < t.linear));
    }
    let ba = a.bounds();
    let bb = b.bounds();
    assert!((ba.min - bb.min).norm() < t.linear);
    assert!((ba.max - bb.max).norm() < t.linear);
}
#[test]
fn independent_metre_tetrahedron_preserves_edge_bound_and_face_senses() {
    let t = Tolerance::default();
    let solid = import_step_planar_mm(TETRA, t).unwrap();
    assert_eq!(
        (
            solid.vertices.len(),
            solid.edges.len(),
            solid.shell.faces.len()
        ),
        (4, 6, 4)
    );
    assert!((solid.volume().unwrap() - 4.).abs() < 1e-12);
    assert_eq!(solid.bounds().min, Point3::new(0., 0., 0.));
    assert_eq!(solid.bounds().max, Point3::new(2., 3., 4.));
    assert_eq!(solid.shell.faces[0].orientation, -1);
    let again = import_step_planar_mm(&export_step_planar_mm(&solid, t).unwrap(), t).unwrap();
    same_geometry(&solid, &again, t);
    let report: serde_json::Value =
        serde_json::from_str(&import_step_planar_json(TETRA).unwrap()).unwrap();
    assert_eq!(report["units"], "mm");
    assert_eq!(report["mesh"]["faces"], 4);
    assert!((report["mesh"]["volume"].as_f64().unwrap() - 4.).abs() < 1e-12);
    // Layout, forward references and labels cannot change interpretation.
    let mut lines: Vec<_> = TETRA.lines().filter(|s| s.starts_with('#')).collect();
    lines.reverse();
    let prefix = TETRA.split("DATA;\n").next().unwrap();
    let reordered = format!(
        "{prefix}DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;",
        lines.join("\n")
    );
    same_geometry(&solid, &import_step_planar_mm(&reordered, t).unwrap(), t);
    let quoted = TETRA.replace(
        "Hand authored tetrahedron",
        "DATA; ''quoted'' #999999=PLANE; ENDSEC;",
    );
    assert!(import_step_planar_mm(&quoted, t).is_ok());
}
#[test]
fn convex_box_skew_prism_and_rotated_geometry_round_trip_at_three_scales() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let skew = extrude_polygon(
            &PolygonProfile {
                origin: Point3::new(0., 0., -scale),
                outer: vec![
                    [-4. * scale, -3. * scale],
                    [4. * scale, -3. * scale],
                    [4. * scale, 3. * scale],
                    [-4. * scale, 3. * scale],
                ],
                holes: vec![],
            },
            Vec3::new(2. * scale, -scale, 2. * scale),
            t,
        )
        .unwrap();
        let placed = skew
            .transformed(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(), t)
            .unwrap();
        for source in [box_solid(scale), skew, placed] {
            let step = export_step_planar_mm(&source, t).unwrap();
            let restored = import_step_planar_mm(&step, t).unwrap();
            same_geometry(&source, &restored, t);
            assert!(restored.tessellate(0.01 * scale, t).is_ok());
        }
    }
}
#[test]
fn syntax_geometry_units_and_resource_errors_never_succeed() {
    let t = Tolerance::default();
    for invalid in [
        TETRA.replace("#11=VERTEX_POINT('',#1);", "#11=VERTEX_POINT('',#999);"),
        TETRA.replace(
            "#11=VERTEX_POINT('',#1);",
            "#11=VERTEX_POINT('',#1);#11=VERTEX_POINT('',#2);",
        ),
        TETRA.replace("(0.002,0.,0.)", "(1.E999,0.,0.)"),
        TETRA.replace("(0.002,0.,0.)", "(1.E308,0.,0.)"),
        TETRA.replace("(-0.001,0.,0.)", "(-1.E100,0.,0.)"),
        TETRA.replace("(0.002,0.,0.)", "(0.002,0.001,0.)"),
        TETRA.replace("#31,.F.)", "#31,.T.)"),
        TETRA.replace("#150=PRODUCT('tetrahedron','Hand authored tetrahedron','',(#149));", "#150=PRODUCT(0,0,0,0);"),
        TETRA.replace("DATA;\n", "DATA;\n#999=PRODUCT('extra','extra','',(#149));\n"),
        TETRA.replace("LENGTH_MEASURE(1.E-11)","LENGTH_MEASURE(-1.)"),
        TETRA.replace("LENGTH_MEASURE(1.E-11),#141","LENGTH_MEASURE(1.E-11),#142"),
        TETRA.replace("DATA;\n","DATA;\n#999=(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#144)) GLOBAL_UNIT_ASSIGNED_CONTEXT((#141,#142,#143)) REPRESENTATION_CONTEXT('',''));\n"),
        TETRA.replace(
            "#121=ADVANCED_FACE('',(#111),#71,.F.)",
            "#121=ADVANCED_FACE('',(#111),#71,.T.)",
        ),
        TETRA.replace(
            "#131=CLOSED_SHELL('',(#121,#122,#123,#124))",
            "#131=CLOSED_SHELL('',(#121,#122,#123))",
        ),
        TETRA.replace("#141,#142,#143", "#142,#142,#143"),
        TETRA.replace("SI_UNIT($,.METRE.)", "SI_UNIT(.CENTI.,.METRE.)"),
        TETRA.replace(
            "AUTOMOTIVE_DESIGN",
            "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF",
        ),
        TETRA.replace("END-ISO-10303-21;", "END-ISO-10303-21; trailing"),
        TETRA.replace("DATA;\n", "DATA;\n#999=CARTESIAN_POINT('',(9.,9.,9.));\n"),
        TETRA.replace("DATA;\n", "DATA;\n#999=UNKNOWN_EXTENSION();\n"),
        TETRA.replace(
            "#132=MANIFOLD_SOLID_BREP",
            "#999=MANIFOLD_SOLID_BREP('extra',#131);\n#132=MANIFOLD_SOLID_BREP",
        ),
        TETRA.replace(
            "#146=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#132)",
            "#146=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#132,#131)",
        ),
        TETRA.replace(
            "LENGTH_MEASURE(1.E-11)",
            &format!("LENGTH_MEASURE({}1.{} )", "(".repeat(18), ")".repeat(18)),
        ),
        "/* unterminated".into(),
        "".into(),
        " ".repeat(STEP_IMPORT_MAX_BYTES + 1),
    ] {
        assert!(
            import_step_planar_mm(&invalid, t).is_err(),
            "accepted invalid input: {invalid}"
        );
    }
    assert!(import_step_planar_mm(TETRA, Tolerance { linear: f64::NAN }).is_err());
    // Never let a file's large uncertainty enlarge the caller's model tolerance.
    assert!(import_step_planar_mm(
        &TETRA
            .replace("1.E-11", "1.E3")
            .replace("(0.002,0.,0.)", "(0.002,0.000001,0.)"),
        t
    )
    .is_err());
}

#[test]
fn untrusted_truncation_and_token_mutations_never_panic() {
    let t = Tolerance::default();
    for i in (0..TETRA.len()).step_by(13) {
        assert!(std::panic::catch_unwind(|| import_step_planar_mm(&TETRA[..i], t)).is_ok());
        let mut mutated = TETRA.to_string();
        mutated.insert(i, '#');
        assert!(std::panic::catch_unwind(|| import_step_planar_mm(&mutated, t)).is_ok());
    }
    for text in [
        "🚀",
        "ISO-10303-21;/*🚀",
        "ISO-10303-21;HEADER;FILE_NAME('🚀');",
    ] {
        assert!(std::panic::catch_unwind(|| import_step_planar_mm(text, t)).is_ok());
    }
}
#[test]
fn curved_imports_and_strict_convex_holes_and_concavity_are_unsupported() {
    let t = Tolerance::default();
    let cylinder = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4.,
            height: 24.,
        },
        t,
    )
    .unwrap();
    assert!(matches!(
        import_step_planar_mm(&export_step_mm(&cylinder, t).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    let mut doc: serde_json::Value =
        serde_json::from_str(include_str!("../docs/workflow-skew-extrusion-example.json")).unwrap();
    let report: serde_json::Value =
        serde_json::from_str(&export_workflow_step_mm_json(&doc.to_string()).unwrap()).unwrap();
    assert!(matches!(
        import_step_convex_planar_mm(report["step"].as_str().unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    doc["operations"][0]["holes"] = serde_json::json!([]);
    doc["operations"][0]["outer"] =
        serde_json::json!([[0., 0.], [8., 0.], [8., 3.], [3., 3.], [3., 8.], [0., 8.]]);
    let report: serde_json::Value =
        serde_json::from_str(&export_workflow_step_mm_json(&doc.to_string()).unwrap()).unwrap();
    assert!(matches!(
        import_step_convex_planar_mm(report["step"].as_str().unwrap(), t),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn structured_resource_limits_fail_before_geometry_reconstruction() {
    let t = Tolerance::default();
    let long_loop = format!("#101=EDGE_LOOP('',({}));", vec!["#81"; 257].join(","));
    assert!(matches!(
        import_step_planar_mm(
            &TETRA.replace("#101=EDGE_LOOP('',(#81,#82,#83));", &long_loop),
            t
        ),
        Err(Error::Unsupported(
            "STEP import supports 256 coedges per face and 4096 total"
        ))
    ));
    let long_shell = format!("#131=CLOSED_SHELL('',({}));", vec!["#121"; 129].join(","));
    assert!(matches!(
        import_step_planar_mm(
            &TETRA.replace("#131=CLOSED_SHELL('',(#121,#122,#123,#124));", &long_shell),
            t
        ),
        Err(Error::Unsupported("STEP import supports 1..128 faces"))
    ));
    let long_list = format!("#999=APPLICATION_CONTEXT(({}));", vec!["0"; 4097].join(","));
    assert!(matches!(
        import_step_planar_mm(
            &TETRA.replace("DATA;\n", &format!("DATA;\n{long_list}\n")),
            t
        ),
        Err(Error::InvalidInput("STEP list exceeds 4096 values"))
    ));
    let many_entities = (1000..34000)
        .map(|id| format!("#{id}=LINE();\n"))
        .collect::<String>();
    assert!(matches!(
        import_step_planar_mm(
            &TETRA.replace("DATA;\n", &format!("DATA;\n{many_entities}")),
            t
        ),
        Err(Error::InvalidInput("STEP entity budget exceeded"))
    ));
    let many_values = (1000..18000)
        .map(|id| format!("#{id}=LINE(0,0,0,0,0,0,0,0);\n"))
        .collect::<String>();
    assert!(matches!(
        import_step_planar_mm(
            &TETRA.replace("DATA;\n", &format!("DATA;\n{many_values}")),
            t
        ),
        Err(Error::InvalidInput("STEP value budget exceeded"))
    ));
}

fn concave_profile(scale: f64) -> PolygonProfile {
    PolygonProfile {
        origin: Point3::new(0., 0., 0.),
        outer: [[0., 0.], [8., 0.], [8., 3.], [3., 3.], [3., 8.], [0., 8.]]
            .map(|p| [p[0] * scale, p[1] * scale])
            .to_vec(),
        holes: vec![
            [[1., 1.], [2., 1.], [2., 2.], [1., 2.]]
                .map(|p| [p[0] * scale, p[1] * scale])
                .to_vec(),
            [[1., 5.], [2., 5.], [2., 6.], [1., 6.]]
                .map(|p| [p[0] * scale, p[1] * scale])
                .to_vec(),
        ],
    }
}
#[test]
fn concave_two_hole_prisms_round_trip_with_skew_reversal_rotation_and_scale() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        for sign in [-1., 1.] {
            let source = extrude_polygon(
                &concave_profile(scale),
                Vec3::new(scale, -scale, 4. * scale * sign),
                t,
            )
            .unwrap();
            for solid in [
                source.clone(),
                source
                    .transformed(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(), t)
                    .unwrap(),
            ] {
                let certificate = certify_planar_prism(&solid, t).unwrap();
                assert_eq!(certificate.profile_corners, 14);
                assert_eq!(certificate.profile_holes, 2);
                assert!(
                    (solid.volume().unwrap() - 148. * scale.powi(3)).abs() < 1e-10 * scale.powi(3)
                );
                let step = export_step_planar_mm(&solid, t).unwrap();
                let restored = import_step_planar_mm(&step, t).unwrap();
                same_geometry(&solid, &restored, t);
                assert_eq!(certify_planar_prism(&restored, t).unwrap().profile_holes, 2);
                assert!(restored.tessellate(0.01 * scale, t).is_ok());
                assert!(import_step_convex_planar_mm(&step, t).is_err());
            }
        }
    }
}
#[test]
fn oblique_roof_nonprismatic_concave_solid_is_not_admitted_by_closure_alone() {
    let t = Tolerance::default();
    let source = extrude_polygon(&concave_profile(1.), Vec3::new(0., 0., 4.), t).unwrap();
    for slope in [0.1, 1e-10] {
        let plane = Surface::Plane {
            origin: Point3::new(0., 0., 2.),
            u: Vec3::new(1., 0., slope).normalized().unwrap(),
            v: Vec3::new(0., 1., 0.),
        };
        let cut = split_solid_by_plane(&source, &plane, GeometryTolerance::try_from(t).unwrap())
            .unwrap()
            .negative;
        cut.validate(t).unwrap();
        assert!(matches!(
            certify_planar_prism(&cut, t),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            import_step_planar_mm(&export_step_planar_mm(&cut, t).unwrap(), t),
            Err(Error::Unsupported(_))
        ));
    }
}
#[test]
fn inner_bounds_can_precede_outer_but_missing_duplicate_or_multiple_outer_bounds_fail() {
    let t = Tolerance::default();
    let source = extrude_polygon(&concave_profile(1.), Vec3::new(1., -1., 4.), t).unwrap();
    let step = export_step_planar_mm(&source, t).unwrap();
    let mut reordered = String::new();
    for line in step.lines() {
        if line.contains("=ADVANCED_FACE") {
            let start = line.find("('',(").unwrap() + 5;
            let end = start + line[start..].find(')').unwrap();
            let mut ids: Vec<_> = line[start..end].split(',').collect();
            ids.rotate_left(1);
            reordered.push_str(&format!(
                "{}{}{}\n",
                &line[..start],
                ids.join(","),
                &line[end..]
            ));
        } else {
            reordered.push_str(line);
            reordered.push('\n');
        }
    }
    same_geometry(&source, &import_step_planar_mm(&reordered, t).unwrap(), t);
    // Reverse each inner bound independently of the plane/face senses.
    let inner_loops: std::collections::BTreeSet<_> = step
        .lines()
        .filter(|line| line.contains("=FACE_BOUND"))
        .map(|line| line.split(',').nth(1).unwrap().to_string())
        .collect();
    let mut reversed_edges = std::collections::BTreeSet::new();
    let mut reversed = Vec::new();
    for line in step.lines() {
        let id = line.split('=').next().unwrap();
        if inner_loops.contains(id) {
            let start = line.find("('',(").unwrap() + 5;
            let end = start + line[start..].find(')').unwrap();
            let mut entries: Vec<_> = line[start..end].split(',').collect();
            reversed_edges.extend(entries.iter().map(|id| id.to_string()));
            entries.reverse();
            reversed.push(format!(
                "{}{}{}",
                &line[..start],
                entries.join(","),
                &line[end..]
            ));
        } else if line.contains("=FACE_BOUND") {
            reversed.push(line.replace(",.T.)", ",.F.)"));
        } else {
            reversed.push(line.to_string());
        }
    }
    for line in &mut reversed {
        if reversed_edges.contains(line.split('=').next().unwrap()) {
            *line = if line.ends_with(",.T.);") {
                line.replace(",.T.);", ",.F.);")
            } else {
                line.replace(",.F.);", ",.T.);")
            };
        }
    }
    same_geometry(
        &source,
        &import_step_planar_mm(&reversed.join("\n"), t).unwrap(),
        t,
    );

    let first_bound = step
        .lines()
        .find(|line| line.contains("=FACE_OUTER_BOUND"))
        .unwrap()
        .split('=')
        .next()
        .unwrap();
    let duplicated = step.replacen(
        &format!("('',({first_bound},"),
        &format!("('',({first_bound},{first_bound},"),
        1,
    );
    assert_ne!(duplicated, step);
    assert!(import_step_planar_mm(&duplicated, t).is_err());

    assert!(import_step_planar_mm(&step.replace("FACE_OUTER_BOUND", "FACE_BOUND"), t).is_err());
    assert!(import_step_planar_mm(&step.replace("=FACE_BOUND", "=FACE_OUTER_BOUND"), t).is_err());
    let mut damaged = source.clone();
    damaged.edges[0].vertices = [0, 0];
    assert!(certify_planar_prism(&damaged, t).is_err());
}

#[test]
fn imported_openings_retain_empty_material_and_conforming_display() {
    let t = Tolerance::default();
    let source = extrude_polygon(&concave_profile(1.), Vec3::new(0., 0., 4.), t).unwrap();
    let solid = import_step_planar_mm(&export_step_planar_mm(&source, t).unwrap(), t).unwrap();
    let policy = GeometryTolerance::try_from(t).unwrap();
    for (p, expected) in [
        (Point3::new(1.5, 1.5, 2.), PointLocation::Outside),
        (Point3::new(1.5, 5.5, 2.), PointLocation::Outside),
        (Point3::new(0.5, 0.5, 2.), PointLocation::Inside),
        (Point3::new(5., 5., 2.), PointLocation::Outside),
    ] {
        assert_eq!(
            classify_point_in_solid(&solid, p, policy).unwrap(),
            expected
        );
    }
    let mesh = solid.tessellate(0.01, t).unwrap();
    assert!((mesh.signed_volume() - 148.).abs() < 1e-10);
    let key = |p: Point3| {
        [
            (p.x * 1e9).round() as i64,
            (p.y * 1e9).round() as i64,
            (p.z * 1e9).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for triangle in &mesh.triangles {
        for i in 0..3 {
            let a = key(mesh.positions[triangle[i]]);
            let b = key(mesh.positions[triangle[(i + 1) % 3]]);
            let (edge, sign) = if a < b { ([a, b], 1) } else { ([b, a], -1) };
            let entry = uses.entry(edge).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += sign;
        }
        let points = triangle.map(|i| mesh.positions[i]);
        if points.iter().all(|p| p.z == 0.) || points.iter().all(|p| p.z == 4.) {
            let center = (points[0] + points[1] + points[2]) * (1. / 3.);
            for y in [1., 5.] {
                assert!(!(center.x > 1. && center.x < 2. && center.y > y && center.y < y + 1.));
            }
        }
    }
    assert!(uses
        .values()
        .all(|&(count, orientation)| count == 2 && orientation == 0));
}
