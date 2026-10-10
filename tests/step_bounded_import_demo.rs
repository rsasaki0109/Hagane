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
