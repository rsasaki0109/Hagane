use hagane::*;
use std::collections::BTreeMap;
fn cube(scale: f64, t: Tolerance) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8. * scale, 6. * scale, 4. * scale),
        },
        t,
    )
    .unwrap()
}
fn edge_at(s: &Solid, x: f64, y: f64) -> usize {
    s.edges
        .iter()
        .position(|e| match e.curve {
            Curve::Line { a, b } => a.x == x && b.x == x && a.y == y && b.y == y,
            _ => false,
        })
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
                for fraction in [0., 0.3, 1.] {
                    let p = range[0] + fraction * (range[1] - range[0]);
                    let uv = c.pcurve.try_evaluate(p).unwrap();
                    let a = f.surface.try_evaluate(uv[0], uv[1]).unwrap();
                    let b = s.edges[c.edge].curve.try_evaluate(p).unwrap();
                    assert!((a - b).norm() < t.linear * 4.);
                }
            }
        }
    }
    assert_eq!(uses.len(), s.edges.len());
    assert!(uses.values().all(|v| v.len() == 2 && v[0] != v[1]));
}
#[test]
fn box_cut_has_analytic_removed_volume_width_and_both_face_setbacks() {
    let gt = GeometryTolerance::default();
    let t = gt.absolute();
    let source = cube(1., t);
    let edge = edge_at(&source, 0., 0.);
    let d = 0.5;
    let cut = chamfer_straight_convex_edge(&source, edge, d, gt).unwrap();
    closed(cut.solid(), t);
    closed(cut.removed(), t);
    assert!((cut.removed().volume().unwrap() - d * d * 4. / 2.).abs() < 1e-10);
    assert!(
        (cut.solid().volume().unwrap() + cut.removed().volume().unwrap()
            - source.volume().unwrap())
        .abs()
            < 1e-10
    );
    assert_eq!(cut.source_edge(), edge);
    assert_eq!(cut.setback(), d);
    let points: Vec<_> = cut
        .solid()
        .vertices
        .iter()
        .map(|v| v.point)
        .filter(|p| (p.x + p.y - d).abs() < t.linear)
        .collect();
    assert_eq!(points.len(), 4);
    for z in [0., 4.] {
        let a = points
            .iter()
            .find(|p| (**p - Point3::new(d, 0., z)).norm() < t.linear)
            .unwrap();
        let b = points
            .iter()
            .find(|p| (**p - Point3::new(0., d, z)).norm() < t.linear)
            .unwrap();
        assert!(((*a - *b).norm() - std::f64::consts::SQRT_2 * d).abs() < t.linear);
    }
    assert_eq!(source.vertices.len(), 8);
    assert_eq!(source.shell.faces.len(), 6);
}
#[test]
fn rigid_pose_micro_scaling_and_opposite_disjoint_cuts_preserve_real_geometry() {
    for scale in [1e-4, 1., 1e4] {
        let gt = GeometryTolerance::new(scale * 1e-9, 1e-10, 0.).unwrap();
        let t = gt.absolute();
        let original = cube(scale, t);
        let edge = edge_at(&original, 0., 0.);
        let frame = Transform::translation(Vec3::new(12. * scale, -8. * scale, 4. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
            .unwrap();
        let source = original.transformed(frame, t).unwrap();
        let cut = chamfer_straight_convex_edge(&source, edge, 0.5 * scale, gt).unwrap();
        closed(cut.solid(), t);
        let expected = 0.5 * scale.powi(3);
        assert!((cut.removed().volume().unwrap() / expected - 1.).abs() < 1e-8);
    }
    let gt = GeometryTolerance::default();
    let source = cube(1., gt.absolute());
    let first = chamfer_straight_convex_edge(&source, edge_at(&source, 0., 0.), 0.5, gt).unwrap();
    let second =
        chamfer_straight_convex_edge(first.solid(), edge_at(first.solid(), 8., 6.), 0.25, gt)
            .unwrap();
    closed(second.solid(), gt.absolute());
    assert!((second.solid().volume().unwrap() - (192. - 0.5 - 0.125)).abs() < 1e-9);
}
#[test]
fn rejected_geometry_and_nonfinite_inputs_do_not_mutate_stock() {
    let gt = GeometryTolerance::default();
    let source = cube(1., gt.absolute());
    let before = export_step_planar_mm(&source, gt.absolute()).unwrap();
    let edge = edge_at(&source, 0., 0.);
    for d in [0., -1., 6., f64::NAN, f64::INFINITY] {
        assert!(chamfer_straight_convex_edge(&source, edge, d, gt).is_err());
    }
    assert!(chamfer_straight_convex_edge(&source, usize::MAX, 0.5, gt).is_err());
    assert_eq!(
        export_step_planar_mm(&source, gt.absolute()).unwrap(),
        before
    );
    let concave = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 2.], [2., 2.], [2., 6.], [0., 6.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 4.),
        gt.absolute(),
    )
    .unwrap();
    assert!(chamfer_straight_convex_edge(&concave, edge_at(&concave, 2., 2.), 0.25, gt).is_err());
    let holed = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 6.], [0., 6.]],
            holes: vec![vec![[2., 2.], [3., 2.], [3., 3.], [2., 3.]]],
        },
        Vec3::new(0., 0., 4.),
        gt.absolute(),
    )
    .unwrap();
    assert!(chamfer_straight_convex_edge(&holed, edge_at(&holed, 0., 0.), 0.25, gt).is_err());
    let far = source
        .transformed(
            Transform::translation(Vec3::new(1e12, 1e12, 1e12)).unwrap(),
            gt.absolute(),
        )
        .unwrap();
    assert!(chamfer_straight_convex_edge(&far, edge, 0.5, gt).is_err());
    let cylinder = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        gt.absolute(),
    )
    .unwrap();
    assert!(chamfer_straight_convex_edge(&cylinder, 0, 0.5, gt).is_err());
    let shallow = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [4., 0.], [8., 1e-4], [8., 6.], [0., 6.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 4.),
        gt.absolute(),
    )
    .unwrap();
    let unresolved_angle = GeometryTolerance::new(gt.linear(), 1e-3, 0.).unwrap();
    assert!(chamfer_straight_convex_edge(
        &shallow,
        edge_at(&shallow, 4., 0.),
        0.5,
        unresolved_angle
    )
    .is_err());
}
#[test]
fn skew_prism_removes_the_independent_transverse_wedge_volume() {
    let gt = GeometryTolerance::default();
    let t = gt.absolute();
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(1e-100, -1e-100, 1e-100),
            outer: vec![[0., 0.], [8., 0.], [8., 6.], [0., 6.]],
            holes: vec![],
        },
        Vec3::new(2., -1., 4.),
        t,
    )
    .unwrap();
    let edge=s.edges.iter().position(|e|matches!(e.curve,Curve::Line{a,b} if (a.x-b.x).abs()==2.&&(a.y-b.y).abs()==1.&&(a.z-b.z).abs()==4.)).unwrap();
    let cut = chamfer_straight_convex_edge(&s, edge, 0.5, gt).unwrap();
    // Adjacent physical outward normals have |dot|=1/sqrt(85).
    // The perpendicular wedge area is d² sin(dihedral)/2.
    let expected = 0.125 * (84_f64 / 85.).sqrt() * 21_f64.sqrt();
    assert!((cut.removed().volume().unwrap() - expected).abs() < 1e-10);
    closed(cut.solid(), t);
    closed(cut.removed(), t);
    assert!(
        (cut.solid().volume().unwrap() + cut.removed().volume().unwrap() - s.volume().unwrap())
            .abs()
            < 1e-9
    );
}
