use hagane::*;
use std::f64::consts::PI;
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
fn all_subsets_of_each_parallel_axis_have_exact_round_volume_and_topology() {
    let t = GeometryTolerance::default();
    let s = stock(1., t.absolute());
    for (edges, h) in [([8, 9, 10, 11], 4.), ([0, 2, 4, 6], 8.), ([1, 3, 5, 7], 6.)] {
        for mask in 1..16 {
            let request: Vec<_> = (0..4)
                .filter(|k| mask & (1 << k) != 0)
                .map(|k| (edges[k], 0.3 + 0.1 * k as f64))
                .collect();
            let r = fillet_parallel_box_edges(&s, &request, t).unwrap();
            let removed = h * (1. - PI / 4.) * request.iter().map(|(_, r)| r * r).sum::<f64>();
            assert!((r.removed_volume() - removed).abs() < 1e-12);
            assert!((r.solid().volume().unwrap() + removed - 192.).abs() < 1e-10);
            let k = request.len();
            assert_eq!(
                (
                    r.solid().vertices.len(),
                    r.solid().edges.len(),
                    r.solid().shell.faces.len()
                ),
                (8 + 2 * k, 12 + 3 * k, 6 + k)
            );
            assert_eq!(r.fillet_faces().len(), k);
            assert_eq!(r.selections(), request);
            r.solid().validate(t.absolute()).unwrap();
        }
    }
}
#[test]
fn arbitrary_placement_and_tiny_resolved_geometry() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let s = stock(1., t.absolute())
        .transformed(tr, t.absolute())
        .unwrap();
    let r = fillet_parallel_box_edges(&s, &[(8, 0.3), (10, 0.5)], t).unwrap();
    assert!((r.solid().volume().unwrap() - (192. - 4. * (1. - PI / 4.) * 0.34)).abs() < 1e-9);
    let t = GeometryTolerance::new(1e-110, 1e-10, 0.).unwrap();
    let s = stock(1e-100, t.absolute());
    let r = fillet_parallel_box_edges(&s, &[(8, 3e-101)], t).unwrap();
    let expected = 4e-100 * (1. - PI / 4.) * 9e-202;
    assert!((r.removed_volume() / expected - 1.).abs() < 1e-10);
}
#[test]
fn radius_axis_topology_and_precision_failures_do_not_mutate_source() {
    let t = GeometryTolerance::default();
    let s = stock(1., t.absolute());
    let before = export_step_mm(&s, t.absolute()).unwrap();
    for request in [
        vec![],
        vec![(8, 0.)],
        vec![(8, f64::NAN)],
        vec![(8, 0.3), (8, 0.4)],
        vec![(8, 0.3), (0, 0.4)],
        vec![(8, 4.), (9, 4.)],
        vec![(usize::MAX, 0.3)],
    ] {
        assert!(fillet_parallel_box_edges(&s, &request, t).is_err());
    }
    let mut malformed = s.clone();
    malformed.vertices[6].point.x += 0.1;
    assert!(fillet_parallel_box_edges(&malformed, &[(8, 0.3)], t).is_err());
    let r = fillet_parallel_box_edges(&s, &[(8, 0.3)], t).unwrap();
    assert!(fillet_parallel_box_edges(r.solid(), &[(8, 0.3)], t).is_err());
    let far = s
        .transformed(
            Transform::translation(Vec3::new(1e12, 0., 0.)).unwrap(),
            t.absolute(),
        )
        .unwrap();
    assert!(fillet_parallel_box_edges(&far, &[(8, 0.3)], t).is_err());
    assert_eq!(before, export_step_mm(&s, t.absolute()).unwrap());
}
