use hagane::*;
fn number(v: f64) -> String {
    let s = v.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.replace('e', "E")
    } else {
        format!("{s}.")
    }
}
fn polygon() -> Vec<[f64; 2]> {
    vec![[0.1, 0.2], [0.8, 0.1], [0.9, 0.7], [0.3, 0.9]]
}
fn check_weights(solid: &Solid, text: &str) {
    let mut count = 0;
    for edge in &solid.edges {
        let Curve::Nurbs(curve) = &edge.curve else {
            panic!()
        };
        if curve.weights().iter().any(|w| *w != 1.) {
            count += 1;
            let weights = curve
                .weights()
                .iter()
                .copied()
                .map(number)
                .collect::<Vec<_>>()
                .join(",");
            assert!(text.contains(&format!("RATIONAL_B_SPLINE_CURVE(({weights}))")));
        }
    }
    assert!(count > 0);
    let mut surfaces = 0;
    for face in &solid.shell.faces {
        let Surface::Nurbs(surface) = &face.surface else {
            panic!()
        };
        if surface.weights().iter().any(|w| *w != 1.) {
            surfaces += 1;
            let nv = surface.control_counts()[1];
            let rows = surface
                .weights()
                .chunks(nv)
                .map(|row| {
                    format!(
                        "({})",
                        row.iter()
                            .copied()
                            .map(number)
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            assert!(text.contains(&format!("RATIONAL_B_SPLINE_SURFACE(({rows}))")));
        }
    }
    assert!(surfaces > 0);
    assert_eq!(text.matches("=PCURVE(").count(), 2 * solid.edges.len());
    assert_eq!(text.matches("=EDGE_CURVE(").count(), solid.edges.len());
    assert_eq!(
        text.matches("=ADVANCED_FACE(").count(),
        solid.shell.faces.len()
    );
}
#[test]
fn retained_near_unit_weights_and_topology_are_not_normalized() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, polygon(), tol).unwrap();
    let text = body.export_step_mm(tol).unwrap();
    check_weights(body.brep(), &text);
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
    assert!(text.contains("SI_UNIT(.MILLI.,.METRE.)"));
    assert_eq!(text.matches("=FACE_BOUND(").count(), 0);
    assert!(import_step_nurbs_graph_mm(&text, tol).is_err());
    assert!(import_step_nurbs_graph_holed_mm(&text, tol).is_err());
}
#[test]
fn annular_placed_signed_surface_records_and_inner_wires() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(),
            tol,
        )
        .unwrap();
    let outer = NurbsGraphPolygonSolid::new(
        &source,
        vec![
            [0.15, 0.25],
            [0.65, 0.15],
            [0.85, 0.45],
            [0.65, 0.85],
            [0.2, 0.8],
        ],
        tol,
    )
    .unwrap();
    let body = outer
        .through_uv_polygon(vec![[0.3, 0.5], [0.5, 0.3], [0.7, 0.5], [0.5, 0.7]], tol)
        .unwrap();
    let text = body.export_step_mm(tol).unwrap();
    check_weights(body.brep(), &text);
    assert_eq!(text.matches("=FACE_BOUND(").count(), 2);
    assert_eq!(text.matches("=CLOSED_SHELL(").count(), 1);
    for row in text.lines().filter(|s| s.contains("=(BOUNDED_SURFACE()")) {
        assert!(row.ends_with("SURFACE());"));
    }
    for row in text.lines().filter(|s| s.contains("=(BOUNDED_CURVE()")) {
        assert!(row.ends_with("REPRESENTATION_ITEM(''));"));
    }
}
#[test]
fn legacy_simple_records_and_invalid_public_geometry() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let text = source.export_step_mm(tol).unwrap();
    assert!(!text.contains("RATIONAL_B_SPLINE"));
    assert_eq!(
        import_step_nurbs_graph_mm(&text, tol)
            .unwrap()
            .export_step_mm(tol)
            .unwrap(),
        text
    );
    let mut body = NurbsGraphPolygonSolid::new(&source, polygon(), tol).unwrap();
    let Curve::Nurbs(curve) = &body.solid.edges[0].curve else {
        panic!()
    };
    let mut weights = curve.weights().to_vec();
    weights[0] += 1e-13;
    body.solid.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            curve.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    assert!(body.export_step_mm(tol).is_err());
    assert!(source
        .export_step_mm(Tolerance { linear: f64::NAN })
        .is_err());
}
