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
fn adjacent_unequal_planes_interact_with_analytic_union_volume() {
    let s = stock();
    let t = GeometryTolerance::default();
    let (a, b) = (0.3, 0.5);
    let r = chamfer_straight_convex_edges(&s, &[(0, a), (8, b)], t).unwrap();
    // Integral_0^a (a-y)(b-y)dy is the shared corner volume.
    let overlap = a * a * b / 2. - a * a * a / 6.;
    let removed = 0.5 * a * a * 8. + 0.5 * b * b * 4. - overlap;
    assert!((s.volume().unwrap() - r.solid().volume().unwrap() - removed).abs() < 1e-9);
    assert_eq!(r.bevels().len(), 2);
    assert_eq!(r.removed().len(), 2);
    assert!(r.bevels()[0].rings[0].iter().all(|p| p.x + p.y >= b - 1e-8));
    for body in std::iter::once(r.solid()).chain(r.removed()) {
        body.validate(t.absolute()).unwrap();
        export_step_mm(body, t.absolute()).unwrap();
    }
    let reversed = chamfer_straight_convex_edges(&s, &[(8, b), (0, a)], t).unwrap();
    assert!((reversed.solid().volume().unwrap() - r.solid().volume().unwrap()).abs() < 1e-9);
}
#[test]
fn disjoint_and_single_selection_are_supported() {
    let s = stock();
    let t = GeometryTolerance::default();
    let r = chamfer_straight_convex_edges(&s, &[(0, 0.3), (6, 0.4)], t).unwrap();
    assert!(
        (r.solid().volume().unwrap() - (192. - 0.5 * (0.3 * 0.3 + 0.4 * 0.4) * 8.)).abs() < 1e-9
    );
    let one = chamfer_straight_convex_edges(&s, &[(0, 0.3)], t).unwrap();
    assert!(
        (one.solid().volume().unwrap()
            - chamfer_straight_convex_edge(&s, 0, 0.3, t)
                .unwrap()
                .solid()
                .volume()
                .unwrap())
        .abs()
            < 1e-9
    );
}
#[test]
fn contacts_duplicates_and_failed_later_cuts_are_transactional() {
    let s = stock();
    let t = GeometryTolerance::default();
    let before = export_step_mm(&s, t.absolute()).unwrap();
    for request in [
        vec![],
        vec![(0, 0.3), (0, 0.4)],
        vec![(0, 0.3), (8, 0.3)],
        vec![(0, 0.3), (8, 100.)],
        (0..12).map(|i| (i, 0.3)).collect(),
    ] {
        assert!(chamfer_straight_convex_edges(&s, &request, t).is_err());
    }
    assert!(chamfer_straight_convex_edges(&s, &vec![(0, 0.3); 65], t).is_err());
    assert_eq!(before, export_step_mm(&s, t.absolute()).unwrap());
}
