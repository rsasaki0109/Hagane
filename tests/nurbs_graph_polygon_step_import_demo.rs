use hagane::*;
use serde_json::Value;

fn fixture(corners: usize, bulge: f64) -> (NurbsGraphPolygonSolid, String) {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], bulge, tol).unwrap();
    let polygon = (0..corners)
        .map(|i| {
            let a = 0.123 + std::f64::consts::TAU * i as f64 / corners as f64;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect();
    let body = NurbsGraphPolygonSolid::new(&source, polygon, tol).unwrap();
    let step = body.export_step_mm(tol).unwrap();
    (body, step)
}

#[test]
fn actual_imported_polygon_geometry_drives_display_mass_inertia_and_query() {
    for (corners, bulge) in [(3, -12.), (5, 30.), (16, 0.001)] {
        let (original, step) = fixture(corners, bulge);
        let imported = import_step_nurbs_graph_polygon_mm(&step, Tolerance::default()).unwrap();
        imported.validate(Tolerance::default()).unwrap();
        assert_eq!(
            imported.source().dimensions(),
            original.source().dimensions()
        );
        assert!((imported.source().bulge() - bulge).abs() < 1e-10);
        assert_eq!(
            imported
                .classify_point(Point3::new(40., 30., 1.), GeometryTolerance::default())
                .unwrap(),
            PointLocation::Inside
        );
        let data: Value =
            serde_json::from_str(&nurbs_graph_polygon_step_import_demo_json(&step, 1.).unwrap())
                .unwrap();
        assert_eq!(data["import"]["kind"], "polygon");
        assert_eq!(data["import"]["source_geometry_preserved"], true);
        assert_eq!(data["import"]["pcurve_representation_verified"], true);
        assert_eq!(data["polygon"].as_array().unwrap().len(), corners);
        assert!(!data["mesh"]["triangles"].as_array().unwrap().is_empty());
        assert!(data["inertia_error"].is_null());
        assert!((data["volume"].as_f64().unwrap() - original.volume().unwrap()).abs() < 1e-7);
        assert_eq!(
            data["brep"]["faces"].as_u64().unwrap() as usize,
            corners + 2
        );
        assert_eq!(
            data["placement"]["translation"],
            serde_json::json!([0., 0., 0.])
        );
    }
}

#[test]
fn display_failure_and_bad_step_are_explicit_and_recover() {
    let (_, step) = fixture(5, 30.);
    for error in [0., -1., f64::NAN, f64::INFINITY, 1e-12] {
        assert!(nurbs_graph_polygon_step_import_demo_json(&step, error).is_err());
    }
    for bad in [
        String::new(),
        "not STEP".into(),
        step[..step.len() / 2].into(),
        format!("{step}garbage"),
        "x".repeat(STEP_IMPORT_MAX_BYTES + 1),
    ] {
        assert!(nurbs_graph_polygon_step_import_demo_json(&bad, 1.).is_err());
    }
    assert!(nurbs_graph_polygon_step_import_demo_json(&step, 1.).is_ok());
}
