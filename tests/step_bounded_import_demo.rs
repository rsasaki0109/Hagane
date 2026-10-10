use hagane::*;
use serde_json::Value;
fn fixture(posed: bool) -> String {
    let tolerance = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let mut source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tolerance.absolute(),
    )
    .unwrap();
    if posed {
        source = source
            .transformed(
                Transform::translation(Vec3::new(10., -20., 4.))
                    .unwrap()
                    .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.3).unwrap())
                    .unwrap(),
                tolerance.absolute(),
            )
            .unwrap();
    }
    let fillet =
        fillet_parallel_box_edges(&source, &[(8, 3.), (9, 3.), (10, 3.), (11, 3.)], tolerance)
            .unwrap();
    export_step_bounded_analytic_mm(fillet.solid(), 1e-6).unwrap()
}
#[test]
fn actual_imported_fillet_geometry_and_pcurves_drive_display_and_reexport() {
    for posed in [false, true] {
        let input = fixture(posed);
        let report: Value =
            serde_json::from_str(&import_step_bounded_analytic_json(&input).unwrap()).unwrap();
        assert_eq!(report["mode"], "bounded_analytic");
        assert_eq!(report["units"], "mm");
        assert_eq!(report["exact"], true);
        assert_eq!(report["source_bytes_preserved"], false);
        let b = &report["solid"]["brep"];
        assert_eq!(b["faces"], 10);
        assert_eq!(b["edges"], 24);
        assert_eq!(b["vertices"].as_array().unwrap().len(), 16);
        assert_eq!(b["closed"], true);
        let expected = 96000. - (4. - std::f64::consts::PI) * 9. * 20.;
        assert!((report["solid"]["volume"].as_f64().unwrap() - expected).abs() < 1e-6);
        for face in b["wires"].as_array().unwrap() {
            for wire in face.as_array().unwrap() {
                for coedge in wire.as_array().unwrap() {
                    for w in coedge["witnesses"].as_array().unwrap() {
                        for k in 0..3 {
                            assert!(
                                (w["point"][k].as_f64().unwrap()
                                    - w["surface_point"][k].as_f64().unwrap())
                                .abs()
                                    < 1e-6
                            );
                        }
                    }
                }
            }
        }
        for bound in report["solid"]["error_bounds"].as_array().unwrap() {
            assert!(bound.as_f64().unwrap() <= 0.1);
        }
        let imported = import_step_bounded_analytic_mm(
            report["step"].as_str().unwrap(),
            Tolerance::new(1e-6).unwrap(),
        )
        .unwrap();
        assert!((imported.volume().unwrap() - expected).abs() < 1e-6);
        assert!(import_step_mm(&input, Tolerance::new(1e-6).unwrap()).is_err());
    }
}
#[test]
fn malformed_missing_topology_oversize_and_bad_units_reject_then_recover() {
    let input = fixture(false);
    for invalid in [
        String::new(),
        "not STEP".into(),
        input[..input.len() / 2].into(),
        format!("{input}garbage"),
        "x".repeat(STEP_IMPORT_MAX_BYTES + 1),
        input.replace(".MILLI.,.METRE.", ".CENTI.,.METRE."),
    ] {
        assert!(import_step_bounded_analytic_json(&invalid).is_err());
    }
    assert!(import_step_bounded_analytic_json(&input).is_ok());
}

#[test]
fn metre_documents_convert_actual_geometry_to_millimetres() {
    let tolerance = GeometryTolerance::new(1e-9, 1e-10, 0.).unwrap();
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(0.08, 0.06, 0.02),
        },
        tolerance.absolute(),
    )
    .unwrap();
    let fillet = fillet_parallel_box_edges(
        &source,
        &[(8, 0.003), (9, 0.003), (10, 0.003), (11, 0.003)],
        tolerance,
    )
    .unwrap();
    let input = export_step_bounded_analytic_mm(fillet.solid(), 1e-9)
        .unwrap()
        .replace(".MILLI.,.METRE.", "$,.METRE.");
    let imported = import_step_bounded_analytic_mm(&input, Tolerance::new(1e-6).unwrap()).unwrap();
    let expected = 96000. - (4. - std::f64::consts::PI) * 9. * 20.;
    assert!((imported.volume().unwrap() - expected).abs() / expected < 1e-12);
    let oversized_physical_model = fixture(false).replace(".MILLI.,.METRE.", "$,.METRE.");
    assert!(matches!(
        import_step_bounded_analytic_mm(&oversized_physical_model, Tolerance::new(1e-6).unwrap()),
        Err(Error::Unsupported(_))
    ));
    assert!(import_step_bounded_analytic_json(&input).is_ok());
}

#[test]
fn actual_two_openings_have_shared_opposing_edges_inward_walls_and_roundtrip() {
    let input = bounded_analytic_openings_sample_step().unwrap();
    let report: Value =
        serde_json::from_str(&import_step_bounded_analytic_json(&input).unwrap()).unwrap();
    let solid = &report["solid"];
    let b = &solid["brep"];
    let expected =
        20. * (4800. - (4. - std::f64::consts::PI) * 64. - 96. - std::f64::consts::PI * 36.);
    assert!((solid["volume"].as_f64().unwrap() - expected).abs() < 1e-7);
    assert_eq!(b["vertices"].as_array().unwrap().len(), 32);
    assert_eq!(b["edges"], 48);
    assert_eq!(b["faces"], 18);
    let mut incidence = vec![Vec::new(); 48];
    let mut holes = 0;
    for face in b["wires"].as_array().unwrap() {
        holes += face.as_array().unwrap().len() - 1;
        for wire in face.as_array().unwrap() {
            for c in wire.as_array().unwrap() {
                incidence[c["edge"].as_u64().unwrap() as usize]
                    .push(c["forward"].as_bool().unwrap());
            }
        }
    }
    assert_eq!(holes, 4);
    assert_eq!(32_i64 - 48 + 18 - holes as i64, -2);
    for uses in incidence {
        assert_eq!(uses.len(), 2);
        assert_ne!(uses[0], uses[1]);
    }
    let mesh = &solid["mesh"];
    let mut inward = 0;
    for (i, face) in mesh["face_ids"].as_array().unwrap().iter().enumerate() {
        let surface = &b["surfaces"][face.as_u64().unwrap() as usize];
        if surface["kind"] == "framed_cylinder"
            && (surface["frame"]["origin"][0].as_f64().unwrap() - 20.).abs() < 1e-8
            && surface["radius"] == 6.
        {
            for id in mesh["triangles"][i].as_array().unwrap() {
                let id = id.as_u64().unwrap() as usize;
                let p = &mesh["positions"][id];
                let n = &mesh["normals"][id];
                assert!(
                    (p[0].as_f64().unwrap() - 20.) * n[0].as_f64().unwrap()
                        + p[1].as_f64().unwrap() * n[1].as_f64().unwrap()
                        < 0.
                );
                inward += 1;
            }
        }
    }
    assert!(inward > 0);
    for bound in solid["error_bounds"].as_array().unwrap() {
        assert!(bound.as_f64().unwrap() <= 0.1);
    }
    let imported = import_step_bounded_analytic_mm(
        report["step"].as_str().unwrap(),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    assert!((imported.volume().unwrap() - expected).abs() < 1e-7);
    let sample: Value =
        serde_json::from_str(&bounded_analytic_openings_sample_json().unwrap()).unwrap();
    assert_eq!(sample["input_step"], input);
    assert_eq!(sample["report"], report);
}
