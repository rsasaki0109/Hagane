use hagane::*;
use std::collections::BTreeMap;
fn stock() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
fn closed(s: &Solid, t: Tolerance) {
    s.validate(t).unwrap();
    let mut uses = BTreeMap::<usize, Vec<bool>>::new();
    for f in &s.shell.faces {
        assert!(matches!(f.surface, Surface::Plane { .. }));
        for w in &f.wires {
            for c in &w.coedges {
                uses.entry(c.edge)
                    .or_default()
                    .push(c.forward == (f.orientation == 1));
                let range = s.edges[c.edge].curve.range();
                for q in [0., 0.37, 1.] {
                    let p = range[0] + q * (range[1] - range[0]);
                    let uv = c.pcurve.try_evaluate(p).unwrap();
                    assert!(
                        (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                            - s.edges[c.edge].curve.try_evaluate(p).unwrap())
                        .norm()
                            < t.linear * 4.
                    );
                }
            }
        }
    }
    assert_eq!(uses.len(), s.edges.len());
    assert!(uses.values().all(|v| v.len() == 2 && v[0] != v[1]));
}
fn same_final_patch(patch: &PlanarFacePatch, s: &Solid, t: Tolerance) {
    assert_eq!(patch.rings.len(), 1);
    let expected = &patch.rings[0];
    assert!(s.shell.faces.iter().any(|f| f.wires.len() == 1 && {
        let vertices: Vec<_> = f.wires[0]
            .coedges
            .iter()
            .map(|c| {
                let e = &s.edges[c.edge];
                s.vertices[e.vertices[usize::from(!c.forward)]].point
            })
            .collect();
        vertices.len() == expected.len()
            && expected
                .iter()
                .all(|p| vertices.iter().any(|q| (*p - *q).norm() < t.linear))
    }));
}
fn material(p: Point3, d: f64) -> bool {
    let a = [p.x.min(80. - p.x), p.y.min(60. - p.y), p.z.min(20. - p.z)];
    a.iter().all(|x| *x >= -1e-7)
        && a[0] + a[1] >= d - 1e-7
        && a[0] + a[2] >= d - 1e-7
        && a[1] + a[2] >= d - 1e-7
}
fn check_geometry(result: &PlanarEdgeChamfers, frame: Transform, t: GeometryTolerance) {
    let s = result.solid();
    closed(s, t.absolute());
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (32, 48, 18)
    );
    for patch in planar_face_patches(s, t.absolute()).unwrap() {
        let ring = &patch.rings[0];
        let uv: Vec<_> = ring.iter().map(|p| patch.surface.parameters(*p)).collect();
        for i in 0..uv.len() {
            assert_ne!(
                orient2d(uv[i], uv[(i + 1) % uv.len()], uv[(i + 2) % uv.len()]).unwrap(),
                Orientation::Clockwise
            );
        }
        for i in 1..ring.len() - 1 {
            for a in 0..=4 {
                for b in 0..=4 - a {
                    let p = ring[0] * (a as f64 / 4.)
                        + ring[i] * (b as f64 / 4.)
                        + ring[i + 1] * ((4 - a - b) as f64 / 4.);
                    assert!(material(frame.local_point(p), 3.), "p={p:?}");
                }
            }
        }
    }
    for patch in result.bevels() {
        same_final_patch(patch, s, t.absolute());
    }
    let removed = result
        .removed()
        .iter()
        .map(|r| {
            closed(r, t.absolute());
            r.volume().unwrap()
        })
        .sum::<f64>();
    assert!((s.volume().unwrap() - 93282.).abs() < 1e-6);
    assert!((removed - 2718.).abs() < 1e-6);
}
#[test]
fn all_twelve_equal_box_chamfers_have_analytic_volume_and_actual_closed_convex_faces() {
    let source = stock();
    let t = GeometryTolerance::default();
    let selections: Vec<_> = (0..12).map(|i| (i, 3.)).collect();
    let before = export_step_planar_mm(&source, t.absolute()).unwrap();
    let result =
        chamfer_straight_convex_edges_with_vertex_contacts(&source, &selections, t).unwrap();
    check_geometry(
        &result,
        Transform::translation(Vec3::new(0., 0., 0.)).unwrap(),
        t,
    );
    assert_eq!(
        export_step_planar_mm(&source, t.absolute()).unwrap(),
        before
    );
    assert!(matches!(
        chamfer_straight_convex_edges(&source, &selections, t),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn arbitrary_rigid_pose_and_reversed_cut_order_preserve_actual_contact_geometry() {
    let t = GeometryTolerance::default();
    let original = stock();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = original.transformed(frame, t.absolute()).unwrap();
    let selections: Vec<_> = (0..12).rev().map(|i| (i, 3.)).collect();
    let result =
        chamfer_straight_convex_edges_with_vertex_contacts(&source, &selections, t).unwrap();
    check_geometry(&result, frame, t);
}
#[test]
fn resource_invalid_contact_and_precision_errors_preserve_original_stock() {
    let t = GeometryTolerance::default();
    let source = stock();
    let before = export_step_planar_mm(&source, t.absolute()).unwrap();
    for cuts in [
        vec![],
        vec![(0, 3.); 65],
        vec![(usize::MAX, 3.)],
        vec![(0, 3.), (0, 4.)],
        vec![(0, f64::NAN)],
        vec![(0, 60.)],
    ] {
        assert!(chamfer_straight_convex_edges_with_vertex_contacts(&source, &cuts, t).is_err());
    }
    let far = source
        .transformed(
            Transform::translation(Vec3::new(1e12, 1e12, 1e12)).unwrap(),
            t.absolute(),
        )
        .unwrap();
    assert!(chamfer_straight_convex_edges_with_vertex_contacts(&far, &[(0, 3.)], t).is_err());
    assert_eq!(
        export_step_planar_mm(&source, t.absolute()).unwrap(),
        before
    );
}
