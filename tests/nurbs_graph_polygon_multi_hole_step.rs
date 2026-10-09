use hagane::*;
fn number(v: f64) -> String {
    let s = v.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.replace('e', "E")
    } else {
        format!("{s}.")
    }
}
fn holes() -> Vec<Vec<[f64; 2]>> {
    vec![
        vec![[0.15, 0.2], [0.3, 0.15], [0.35, 0.35], [0.2, 0.4]],
        vec![[0.6, 0.17], [0.83, 0.23], [0.74, 0.42]],
        vec![[0.23, 0.62], [0.42, 0.65], [0.35, 0.83]],
        vec![[0.6, 0.63], [0.82, 0.62], [0.8, 0.83], [0.62, 0.85]],
    ]
}
fn check(body: &NurbsGraphPolygonMultiHoledSolid, text: &str) {
    let solid = body.brep();
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
    assert!(text.contains("SI_UNIT(.MILLI.,.METRE.)"));
    assert_eq!(text.matches("=VERTEX_POINT(").count(), solid.vertices.len());
    assert_eq!(text.matches("=EDGE_CURVE(").count(), solid.edges.len());
    assert_eq!(text.matches("=SURFACE_CURVE(").count(), solid.edges.len());
    assert_eq!(text.matches("=PCURVE(").count(), 2 * solid.edges.len());
    assert_eq!(
        text.matches("=ADVANCED_FACE(").count(),
        solid.shell.faces.len()
    );
    assert_eq!(text.matches("=FACE_BOUND(").count(), 2 * body.genus());
    assert_eq!(
        text.matches("=FACE_OUTER_BOUND(").count(),
        solid.shell.faces.len()
    );
    assert_eq!(text.matches("=CLOSED_SHELL(").count(), 1);
    assert_eq!(text.matches("=MANIFOLD_SOLID_BREP(").count(), 1);
    let mut nonunit = 0;
    for edge in &solid.edges {
        let Curve::Nurbs(curve) = &edge.curve else {
            panic!()
        };
        if curve.weights().iter().any(|w| *w != 1.) {
            nonunit += 1;
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
    assert!(nonunit > 0);
    for face in &solid.shell.faces {
        let Surface::Nurbs(s) = &face.surface else {
            panic!()
        };
        if s.weights().iter().any(|w| *w != 1.) {
            let rows = s
                .weights()
                .chunks(s.control_counts()[1])
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
}
#[test]
fn genus_two_and_four_keep_actual_rational_geometry_and_all_wires() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    for count in [2, 4] {
        let body = source
            .through_uv_polygons(holes()[..count].to_vec(), tol)
            .unwrap();
        let text = body.export_step_mm(tol).unwrap();
        check(&body, &text);
        assert!(import_step_mm(&text, tol).is_err());
        assert!(import_step_nurbs_graph_auto_mm(&text, tol).is_err());
    }
}
#[test]
fn maximum_sixty_four_corner_placed_trimmed_export() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.01, 0.99], [0.01, 0.99]], tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(),
            tol,
        )
        .unwrap();
    let circle = |center: [f64; 2], radius: f64, n: usize, phase: f64| {
        (0..n)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64 + phase;
                [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
            })
            .collect::<Vec<_>>()
    };
    let outer =
        NurbsGraphPolygonSolid::new(&source, circle([0.5, 0.5], 0.48, 16, 0.01), tol).unwrap();
    let body = outer
        .through_uv_polygons(
            vec![
                circle([0.3, 0.3], 0.06, 16, 0.03),
                circle([0.7, 0.3], 0.06, 16, 0.12),
                circle([0.5, 0.7], 0.06, 16, 0.07),
            ],
            tol,
        )
        .unwrap();
    let text = body.export_step_mm(tol).unwrap();
    check(&body, &text);
    assert_eq!(body.brep().vertices.len(), 128);
    assert_eq!(body.brep().edges.len(), 192);
    assert!(text.len() < 32 * 1024 * 1024);
}
#[test]
fn public_corruption_and_invalid_tolerance_reject_before_serialization() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = source
        .through_uv_polygons(holes()[..2].to_vec(), tol)
        .unwrap();
    let mut bad = body.clone();
    bad.solid.shell.faces[0].wires[2].coedges[0].edge = usize::MAX;
    assert!(bad.export_step_mm(tol).is_err());
    let mut bad = body.clone();
    let Curve::Nurbs(curve) = &bad.solid.edges[0].curve else {
        panic!()
    };
    let mut weights = curve.weights().to_vec();
    weights[0] += 1e-13;
    bad.solid.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            curve.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    assert!(bad.export_step_mm(tol).is_err());
    let mut bad = body.clone();
    bad.solid.vertices[0].point.x += tol.linear / 10.;
    assert!(bad.export_step_mm(tol).is_err());
    assert!(body.export_step_mm(Tolerance { linear: f64::NAN }).is_err());
    assert!(body.export_step_mm(Tolerance { linear: 0. }).is_err());
}
