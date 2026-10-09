use hagane::*;
fn polygon(n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let a = 0.13 + std::f64::consts::TAU * i as f64 / n as f64;
            [0.5 + 0.35 * a.cos(), 0.5 + 0.35 * a.sin()]
        })
        .collect()
}
#[test]
fn actual_geometry_roundtrips_triangle_and_near_unit_rational_sixteen() {
    let t = Tolerance::default();
    for (n, b) in [(3, 30.), (16, 30.), (4, 0.), (5, -2.)] {
        let source = NurbsGraphSolid::new([80., 60., 20.], b, t).unwrap();
        let body = NurbsGraphPolygonSolid::new(&source, polygon(n), t).unwrap();
        let step = body.export_step_mm(t).unwrap();
        let imported = import_step_nurbs_graph_polygon_mm(&step, t).unwrap();
        imported.validate(t).unwrap();
        assert_eq!(imported.brep().vertices.len(), 2 * n);
        assert_eq!(imported.brep().edges.len(), 3 * n);
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-8);
        let reparsed =
            import_step_nurbs_graph_polygon_mm(&imported.export_step_mm(t).unwrap(), t).unwrap();
        assert_eq!(
            reparsed.export_step_mm(t).unwrap(),
            imported.export_step_mm(t).unwrap()
        );
        for edge in &body.brep().edges {
            let Curve::Nurbs(c) = &edge.curve else {
                panic!()
            };
            assert!(imported.brep().edges.iter().any(|e|matches!(&e.curve,Curve::Nurbs(x) if x.control_points()==c.control_points()&&x.weights()==c.weights()&&x.knots()==c.knots())));
        }
    }
}
#[test]
fn no_tolerance_healing_of_actual_coordinate_or_affine() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    let b = NurbsGraphPolygonSolid::new(&s, polygon(3), t).unwrap();
    let text = b.export_step_mm(t).unwrap();
    let changed = text.replacen("80.", "80.00000000000001", 1);
    assert_ne!(text, changed);
    assert!(import_step_nurbs_graph_polygon_mm(&changed, t).is_err());
    let record = text.lines().find(|line| line.contains("=VECTOR(")).unwrap();
    let start = record.rfind(',').unwrap() + 1;
    let end = record.rfind(')').unwrap();
    let length = record[start..end].parse::<f64>().unwrap();
    let replacement = format!("{}{}{}", &record[..start], length.next_up(), &record[end..]);
    let changed = text.replacen(record, &replacement, 1);
    assert!(import_step_nurbs_graph_polygon_mm(&changed, t).is_err());
}
#[test]
fn scope_and_certificate_failures_are_explicit() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    assert!(import_step_nurbs_graph_polygon_mm(&s.export_step_mm(t).unwrap(), t).is_err());
    let p = NurbsGraphPolygonSolid::new(&s, polygon(4), t).unwrap();
    let placed = NurbsGraphSolid::new([80., 60., 20.], 30., t)
        .unwrap()
        .transformed(Transform::translation(Vec3::new(10., 0., 0.)).unwrap(), t)
        .unwrap();
    let placed = NurbsGraphPolygonSolid::new(&placed, polygon(4), t).unwrap();
    assert!(import_step_nurbs_graph_polygon_mm(&placed.export_step_mm(t).unwrap(), t).is_err());
    assert!(import_step_nurbs_graph_polygon_mm(
        &p.export_step_mm(t).unwrap(),
        Tolerance { linear: f64::NAN }
    )
    .is_err());
}
#[test]
fn original_unshifted_sixteen_corner_fixture_preserves_actual_basis() {
    let t = Tolerance::default();
    let points = (0..16)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 16.;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, points, t).unwrap();
    let imported = import_step_nurbs_graph_polygon_mm(&body.export_step_mm(t).unwrap(), t).unwrap();
    for e in &body.brep().edges {
        let Curve::Nurbs(c) = &e.curve else { panic!() };
        assert!(imported.brep().edges.iter().any(|e|matches!(&e.curve,Curve::Nurbs(q) if q.control_points()==c.control_points() && q.weights()==c.weights() && q.knots()==c.knots())));
    }
    assert_eq!(imported.brep().vertices.len(), 32);
}
