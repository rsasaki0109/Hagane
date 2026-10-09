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
fn curved_holed_and_nonconvex_solids_are_explicitly_unsupported() {
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
        import_step_planar_mm(report["step"].as_str().unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    doc["operations"][0]["holes"] = serde_json::json!([]);
    doc["operations"][0]["outer"] =
        serde_json::json!([[0., 0.], [8., 0.], [8., 3.], [3., 3.], [3., 8.], [0., 8.]]);
    let report: serde_json::Value =
        serde_json::from_str(&export_workflow_step_mm_json(&doc.to_string()).unwrap()).unwrap();
    assert!(matches!(
        import_step_planar_mm(report["step"].as_str().unwrap(), t),
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
