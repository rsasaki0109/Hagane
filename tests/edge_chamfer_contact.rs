use hagane::*;
fn stock() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 6., 4.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
#[test]
fn all_twelve_equal_chamfers_form_closed_analytic_polyhedron() {
    let s = stock();
    let t = GeometryTolerance::default();
    let requests: Vec<_> = (0..12).map(|i| (i, 0.3)).collect();
    let r = chamfer_straight_convex_edges_with_vertex_contacts(&s, &requests, t).unwrap();
    assert_eq!(
        (
            r.solid().vertices.len(),
            r.solid().edges.len(),
            r.solid().shell.faces.len()
        ),
        (32, 48, 18)
    );
    assert!(
        (r.solid().volume().unwrap() - (192. - 2. * 0.3f64.powi(2) * 18. + 6. * 0.3f64.powi(3)))
            .abs()
            < 1e-9
    );
    assert_eq!(r.bevels().len(), 12);
    for body in std::iter::once(r.solid()).chain(r.removed()) {
        body.validate(t.absolute()).unwrap();
        export_step_mm(body, t.absolute()).unwrap();
    }
    let total =
        r.removed().iter().map(|s| s.volume().unwrap()).sum::<f64>() + r.solid().volume().unwrap();
    assert!((total - 192.).abs() < 1e-9);
    let reversed: Vec<_> = requests.into_iter().rev().collect();
    let other = chamfer_straight_convex_edges_with_vertex_contacts(&s, &reversed, t).unwrap();
    assert!((other.solid().volume().unwrap() - r.solid().volume().unwrap()).abs() < 1e-9);
}
#[test]
fn equal_adjacent_contact_supported_but_old_contract_preserved() {
    let s = stock();
    let t = GeometryTolerance::default();
    let request = [(0, 0.3), (8, 0.3)];
    assert!(chamfer_straight_convex_edges(&s, &request, t).is_err());
    let r = chamfer_straight_convex_edges_with_vertex_contacts(&s, &request, t).unwrap();
    assert!(
        (r.solid().volume().unwrap() - (192. - 0.5 * 0.09 * 12. + 0.3f64.powi(3) / 3.)).abs()
            < 1e-9
    );
    let before = export_step_mm(&s, t.absolute()).unwrap();
    assert!(
        chamfer_straight_convex_edges_with_vertex_contacts(&s, &[(0, 0.3), (8, 100.)], t).is_err()
    );
    assert_eq!(before, export_step_mm(&s, t.absolute()).unwrap());
}
