use hagane::*;
use std::collections::HashMap;
fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], 20., Tolerance::default()).unwrap()
}
fn transform() -> Transform {
    Transform::translation(Vec3::new(120., -70., 35.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.73).unwrap())
        .unwrap()
}
#[test]
fn placed_exact_surfaces_pcurves_and_bounds_match_independent_rigid_map() {
    let s = source();
    let t = transform();
    let p = s.transformed(t, Tolerance::default()).unwrap();
    p.validate(Tolerance::default()).unwrap();
    assert_eq!(p.volume().unwrap(), s.volume().unwrap());
    let bounds = p.bounds().unwrap();
    for (before, after) in s.solid.shell.faces.iter().zip(&p.solid.shell.faces) {
        for i in 0..=12 {
            for j in 0..=12 {
                let u = i as f64 / 12.;
                let v = j as f64 / 12.;
                let expected = t.point(before.surface.try_evaluate(u, v).unwrap());
                let actual = after.surface.try_evaluate(u, v).unwrap();
                assert!((actual - expected).norm() < 1e-10);
                assert!(
                    actual.x >= bounds.min.x - 1e-10
                        && actual.x <= bounds.max.x + 1e-10
                        && actual.y >= bounds.min.y - 1e-10
                        && actual.y <= bounds.max.y + 1e-10
                        && actual.z >= bounds.min.z - 1e-10
                        && actual.z <= bounds.max.z + 1e-10
                );
            }
        }
        for c in &after.wires[0].coedges {
            let edge = &p.solid.edges[c.edge];
            for j in 0..=12 {
                let at = j as f64 / 12.;
                let uv = c.pcurve.evaluate(at);
                assert!(
                    (edge.curve.try_evaluate(at).unwrap()
                        - after.surface.try_evaluate(uv[0], uv[1]).unwrap())
                    .norm()
                        < 1e-10
                );
            }
        }
    }
}
#[test]
fn placed_display_preserves_normals_node_identity_closure_and_error_bounds() {
    let s = source();
    let t = transform();
    let p = s.transformed(t, Tolerance::default()).unwrap();
    let a = s
        .tessellate_bounded(0.2, 65536, Tolerance::default())
        .unwrap();
    let b = p
        .tessellate_bounded(0.2, 65536, Tolerance::default())
        .unwrap();
    assert_eq!(a.subdivisions, b.subdivisions);
    assert_eq!(a.mesh.triangles, b.mesh.triangles);
    assert_eq!(a.vertex_nodes, b.vertex_nodes);
    assert_eq!(a.vertex_faces, b.vertex_faces);
    assert_eq!(a.vertex_uv, b.vertex_uv);
    for i in 0..a.mesh.positions.len() {
        assert!((b.mesh.positions[i] - t.point(a.mesh.positions[i])).norm() < 1e-10);
        assert!((b.mesh.normals[i] - t.vector(a.mesh.normals[i])).norm() < 1e-10);
    }
    let mut nodes = HashMap::new();
    for (i, node) in b.vertex_nodes.iter().enumerate() {
        if let Some(old) = nodes.insert(node, b.mesh.positions[i]) {
            assert_eq!(old, b.mesh.positions[i]);
        }
    }
    let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    for (index, ids) in b.mesh.triangles.iter().enumerate() {
        let face = &p.solid.shell.faces[b.vertex_faces[ids[0]]];
        let uv = ids.map(|i| b.vertex_uv[i]);
        let points = ids.map(|i| b.mesh.positions[i]);
        for i in 0..=5 {
            for j in 0..=5 - i {
                let weights = [i as f64 / 5., j as f64 / 5., 1. - (i + j) as f64 / 5.];
                let at = [0, 1].map(|axis| (0..3).map(|k| weights[k] * uv[k][axis]).sum());
                let exact = face.surface.try_evaluate(at[0], at[1]).unwrap();
                let linear = Point3::new(
                    (0..3).map(|k| weights[k] * points[k].x).sum(),
                    (0..3).map(|k| weights[k] * points[k].y).sum(),
                    (0..3).map(|k| weights[k] * points[k].z).sum(),
                );
                assert!((exact - linear).norm() <= b.error_bounds[index] + 1e-10);
            }
        }
        for i in 0..3 {
            let x = b.vertex_nodes[ids[i]];
            let y = b.vertex_nodes[ids[(i + 1) % 3]];
            let e = edges.entry((x.min(y), x.max(y))).or_default();
            e.0 += 1;
            e.1 += if x < y { 1 } else { -1 };
        }
    }
    for e in edges.values() {
        assert_eq!(*e, (2, 0));
    }
    assert!((a.mesh.signed_volume() - b.mesh.signed_volume()).abs() < 1e-9);
}
#[test]
fn composition_inverse_and_unresolved_world_display_are_checked() {
    let s = source();
    let t = transform();
    let moved = s.transformed(t, Tolerance::default()).unwrap();
    let restored = moved
        .transformed(t.inverse().unwrap(), Tolerance::default())
        .unwrap();
    for (a, b) in s.solid.vertices.iter().zip(&restored.solid.vertices) {
        assert!((a.point - b.point).norm() < 1e-10);
    }
    let second = Transform::rotation(Vec3::new(0., 1., 0.), -0.4).unwrap();
    let direct = s
        .transformed(second.compose(t).unwrap(), Tolerance::default())
        .unwrap();
    let composed = moved.transformed(second, Tolerance::default()).unwrap();
    for (a, b) in direct.solid.vertices.iter().zip(&composed.solid.vertices) {
        assert!((a.point - b.point).norm() < 1e-10);
    }
    let far = Transform::translation(Vec3::new(1e12, -1e12, 1e12)).unwrap();
    if let Ok(far) = s.transformed(far, Tolerance::default()) {
        assert!(far
            .tessellate_bounded(1e-6, 65536, Tolerance::default())
            .is_err());
    }
}
