use hagane::*;
fn stock(scale: f64, t: Tolerance) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8. * scale, 6. * scale, 4. * scale),
        },
        t,
    )
    .unwrap()
}
#[test]
fn every_box_edge_has_equal_setback_and_analytic_removed_volume() {
    let t = GeometryTolerance::default();
    let s = stock(1., t.absolute());
    for i in 0..s.edges.len() {
        let r = chamfer_straight_convex_edge(&s, i, 0.3, t).unwrap();
        let e = &s.edges[i];
        let length = (s.vertices[e.vertices[1]].point - s.vertices[e.vertices[0]].point).norm();
        assert!((r.removed().volume().unwrap() - 0.5 * 0.3 * 0.3 * length).abs() < 1e-10);
        assert!(
            (r.solid().volume().unwrap() + r.removed().volume().unwrap() - s.volume().unwrap())
                .abs()
                < 1e-9
        );
        r.solid().validate(t.absolute()).unwrap();
        r.removed().validate(t.absolute()).unwrap();
        assert_eq!(r.solid().shell.faces.len(), 7);
        assert_eq!(r.source_edge(), i);
        assert_eq!(r.bevel().rings[0].len(), 4);
        export_step_mm(r.solid(), t.absolute()).unwrap();
    }
}
#[test]
fn tiny_geometry_and_sequential_disjoint_edges() {
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = stock(1e-9, t.absolute());
    let r = chamfer_straight_convex_edge(&s, 0, 3e-10, t).unwrap();
    let e = &s.edges[0];
    let length = (s.vertices[e.vertices[1]].point - s.vertices[e.vertices[0]].point).norm();
    assert!((r.removed().volume().unwrap() / (0.5 * 9e-20 * length) - 1.).abs() < 1e-9);
    let t = GeometryTolerance::default();
    let s = stock(1., t.absolute());
    let first = chamfer_straight_convex_edge(&s, 0, 0.2, t).unwrap();
    let next = first
        .solid()
        .edges
        .iter()
        .enumerate()
        .find_map(|(i, _)| chamfer_straight_convex_edge(first.solid(), i, 0.15, t).ok())
        .unwrap();
    assert!(next.solid().volume().unwrap() < first.solid().volume().unwrap());
    next.solid().validate(t.absolute()).unwrap();
}
#[test]
fn invalid_and_unresolved_inputs_are_rejected_without_mutation() {
    let t = GeometryTolerance::default();
    let s = stock(1., t.absolute());
    let before = export_step_mm(&s, t.absolute()).unwrap();
    for d in [0., -1., f64::NAN, f64::INFINITY, 1e-10, 100.] {
        assert!(chamfer_straight_convex_edge(&s, 0, d, t).is_err());
    }
    assert!(chamfer_straight_convex_edge(&s, usize::MAX, 0.2, t).is_err());
    let mut damaged = s.clone();
    damaged.vertices[0].point.z += 0.1;
    assert!(chamfer_straight_convex_edge(&damaged, 0, 0.2, t).is_err());
    assert_eq!(before, export_step_mm(&s, t.absolute()).unwrap());
    let far = make_box(
        BoxSpec {
            min: Point3::new(1e12, 1e12, 1e12),
            size: Vec3::new(8., 6., 4.),
        },
        t.absolute(),
    )
    .unwrap();
    assert!(chamfer_straight_convex_edge(&far, 0, 0.2, t).is_err());
}
