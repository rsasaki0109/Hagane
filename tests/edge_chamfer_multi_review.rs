use hagane::*;
use std::collections::BTreeMap;
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
#[test]
fn unequal_adjacent_cuts_match_independent_overlap_volume_and_final_bevels() {
    let t = GeometryTolerance::default();
    let source = stock();
    let selections = [(8, 0.5), (3, 0.3)];
    let result = chamfer_straight_convex_edges(&source, &selections, t).unwrap();
    let a = 0.5;
    let b = 0.3;
    let m = b;
    let overlap = a * b * m - (a + b) * m * m / 2. + m * m * m / 3.;
    let removed = a * a * 4. / 2. + b * b * 6. / 2. - overlap;
    assert!((result.solid().volume().unwrap() - (192. - removed)).abs() < 1e-10);
    assert_eq!(result.selections(), selections);
    assert_eq!(result.removed().len(), 2);
    assert_eq!(result.bevels().len(), 2);
    assert!(
        (result
            .removed()
            .iter()
            .map(|s| s.volume().unwrap())
            .sum::<f64>()
            - removed)
            .abs()
            < 1e-10
    );
    closed(result.solid(), t.absolute());
    for part in result.removed() {
        closed(part, t.absolute());
    }
    for patch in result.bevels() {
        same_final_patch(patch, result.solid(), t.absolute());
    }
    // The first bevel is clipped by the second cut; its old full rectangle is stale.
    assert_eq!(result.bevels()[0].rings[0].len(), 5);
    let reversed = chamfer_straight_convex_edges(&source, &[(3, 0.3), (8, 0.5)], t).unwrap();
    assert!((reversed.solid().volume().unwrap() - result.solid().volume().unwrap()).abs() < 1e-10);
    assert_eq!(
        reversed.solid().vertices.len(),
        result.solid().vertices.len()
    );
    for v in &result.solid().vertices {
        assert!(reversed
            .solid()
            .vertices
            .iter()
            .any(|q| (q.point - v.point).norm() < t.linear()));
    }
}
#[test]
fn all_equal_edge_contact_and_invalid_selections_reject_without_changing_source() {
    let t = GeometryTolerance::default();
    let source = stock();
    let before = export_step_planar_mm(&source, t.absolute()).unwrap();
    // Future contact-aware oracle: V=192-2d²(8+6+4)+6d³. The current
    // transverse-only splitter honestly rejects these equal-setback contacts.
    let all: Vec<_> = (0..12).map(|i| (i, 0.5)).collect();
    assert!(matches!(
        chamfer_straight_convex_edges(&source, &all, t),
        Err(Error::Unsupported(_))
    ));
    for selections in [
        vec![],
        vec![(8, 0.5), (8, 0.3)],
        vec![(usize::MAX, 0.5)],
        vec![(8, 0.5), (3, 0.5)],
        vec![(8, 0.5), (3, 6.)],
        vec![(8, f64::NAN)],
    ] {
        assert!(chamfer_straight_convex_edges(&source, &selections, t).is_err());
    }
    assert_eq!(
        export_step_planar_mm(&source, t.absolute()).unwrap(),
        before
    );
}
#[test]
fn skew_prism_rigid_transverse_interaction_closes_and_conserves_volume() {
    let t = GeometryTolerance::default();
    let profile = PolygonProfile {
        origin: Point3::new(0., 0., 0.),
        outer: vec![[0., 0.], [8., 0.], [8., 6.], [0., 6.]],
        holes: vec![],
    };
    let source = extrude_polygon(&profile, Vec3::new(2., -1., 4.), t.absolute()).unwrap();
    let along = source
        .edges
        .iter()
        .position(|e| matches!(e.curve,Curve::Line{a,b} if a.x==0.&&a.y==0.&&b.x==2.&&b.y== -1.))
        .unwrap();
    let base = source
        .edges
        .iter()
        .position(|e| matches!(e.curve,Curve::Line{a,b} if a.x==0.&&b.x==0.&&a.z==0.&&b.z==0.))
        .unwrap();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = source.transformed(frame, t.absolute()).unwrap();
    let result = chamfer_straight_convex_edges(&source, &[(along, 0.5), (base, 0.3)], t).unwrap();
    closed(result.solid(), t.absolute());
    let removed = result
        .removed()
        .iter()
        .map(|s| {
            closed(s, t.absolute());
            s.volume().unwrap()
        })
        .sum::<f64>();
    assert!((removed + result.solid().volume().unwrap() - source.volume().unwrap()).abs() < 1e-9);
    for patch in result.bevels() {
        same_final_patch(patch, result.solid(), t.absolute());
    }
}
