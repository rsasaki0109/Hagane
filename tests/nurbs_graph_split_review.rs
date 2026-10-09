use hagane::*;
use std::collections::HashMap;
fn placement() -> Transform {
    Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap()
}
fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], -2., Tolerance::default())
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], Tolerance::default())
        .unwrap()
        .transformed(placement(), Tolerance::default())
        .unwrap()
}
fn volume(r: [[f64; 2]; 2]) -> f64 {
    let f = |x: f64| x * x / 2. - x * x * x / 3.;
    240. * (3. * (r[0][1] - r[0][0]) * (r[1][1] - r[1][0])
        - 8. * (f(r[0][1]) - f(r[0][0])) * (f(r[1][1]) - f(r[1][0])))
}
fn closed(body: &NurbsGraphSolid) {
    let m = body
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    let mut nodes = HashMap::new();
    for (i, &node) in m.vertex_nodes.iter().enumerate() {
        if let Some(old) = nodes.insert(node, m.mesh.positions[i]) {
            assert_eq!(old, m.mesh.positions[i]);
        }
    }
    let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    for ids in m.mesh.triangles {
        for i in 0..3 {
            let a = m.vertex_nodes[ids[i]];
            let b = m.vertex_nodes[ids[(i + 1) % 3]];
            let e = edges.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    for e in edges.values() {
        assert_eq!(*e, (2, 0));
    }
}
#[test]
fn both_axes_partition_original_uv_volume_and_exact_world_section() {
    let s = source();
    for (axis, cut) in [(0, 0.45), (1, 0.4)] {
        let result = s.split_uv(axis, cut, Tolerance::default()).unwrap();
        result.validate(Tolerance::default()).unwrap();
        assert_eq!(result.axis(), axis);
        assert_eq!(result.parameter(), cut);
        let mut negative = s.source_domain();
        negative[axis][1] = cut;
        let mut positive = s.source_domain();
        positive[axis][0] = cut;
        assert_eq!(result.negative.source_domain(), negative);
        assert_eq!(result.positive.source_domain(), positive);
        assert!((result.negative.volume().unwrap() - volume(negative)).abs() < 1e-10);
        assert!((result.positive.volume().unwrap() - volume(positive)).abs() < 1e-10);
        assert!(
            (result.negative.volume().unwrap() + result.positive.volume().unwrap()
                - s.volume().unwrap())
            .abs()
                < 1e-10
        );
        closed(&result.negative);
        closed(&result.positive);
        let Surface::Plane { origin, u, v } = &result.plane else {
            panic!("exact cuttingplane required")
        };
        let normal = u.cross(*v).normalized().unwrap();
        let expected = placement().vector(if axis == 0 {
            Vec3::new(1., 0., 0.)
        } else {
            Vec3::new(0., 1., 0.)
        });
        assert!((normal.dot(expected).abs() - 1.).abs() < 1e-12);
        let Surface::Nurbs(section) = &result.section.face.surface else {
            panic!("exact graph section required")
        };
        let d = section.domain();
        assert_eq!(d[0], s.source_domain()[1 - axis]);
        assert_eq!(d[1], [0., 1.]);
        for i in 0..=12 {
            for j in 0..=12 {
                let q = d[0][0] + (d[0][1] - d[0][0]) * i as f64 / 12.;
                let t = j as f64 / 12.;
                let (u0, v0) = if axis == 0 { (cut, q) } else { (q, cut) };
                let expected = placement().point(Point3::new(
                    20. * u0,
                    12. * v0,
                    t * (3. - 8. * u0 * (1. - u0) * v0 * (1. - v0)),
                ));
                let actual = section.evaluate(q, t).unwrap();
                assert!((actual - expected).norm() < 1e-10);
                assert!((actual - *origin).dot(normal).abs() < 1e-10);
            }
        }
        let negative_face = &result.negative.solid.shell.faces[if axis == 0 { 3 } else { 5 }];
        let positive_face = &result.positive.solid.shell.faces[if axis == 0 { 2 } else { 4 }];
        let q = (d[0][0] + d[0][1]) / 2.;
        let a = negative_face.surface.normal_at(q, 0.5).unwrap() * negative_face.orientation as f64;
        let b = positive_face.surface.normal_at(q, 0.5).unwrap() * positive_face.orientation as f64;
        assert!((a + b).norm() < 1e-10);
        assert!((a - expected).norm() < 1e-10);
        for c in &result.section.face.wires[0].coedges {
            let edge = &result.section.edges[c.edge];
            let range = edge.curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.evaluate(t);
                assert!(
                    (edge.curve.try_evaluate(t).unwrap() - section.evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                        < 1e-10
                );
            }
        }
    }
}
#[test]
fn split_rejects_boundary_touching_out_of_range_and_unresolved_cuts() {
    let s = source();
    for (axis, cut) in [
        (0, 0.2),
        (0, 0.8),
        (0, 0.1),
        (0, 0.9),
        (0, 0.2 + 1e-15),
        (1, 0.1),
        (1, 0.7),
        (1, f64::NAN),
        (2, 0.5),
    ] {
        assert!(s.split_uv(axis, cut, Tolerance::default()).is_err());
    }
}
#[test]
fn public_section_children_and_plane_corruption_are_rejected() {
    let split = source().split_uv(0, 0.45, Tolerance::default()).unwrap();
    let mut dirty = split.clone();
    dirty.section.vertices[0].point.z += 0.01;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = split.clone();
    dirty.section.face.wires[0].coedges[0].edge = 999;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = split.clone();
    dirty.negative.solid.vertices[0].point.x += 0.01;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = split.clone();
    let Surface::Plane { origin, .. } = &mut dirty.plane else {
        panic!("cutplane required")
    };
    origin.x += 0.01;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = split;
    dirty.positive.solid.shell.faces[2].orientation *= -1;
    assert!(dirty.validate(Tolerance::default()).is_err());
}
