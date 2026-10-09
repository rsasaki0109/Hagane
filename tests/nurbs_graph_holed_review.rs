use hagane::*;
use std::collections::HashMap;
fn hole() -> [[f64; 2]; 2] {
    [[0.35, 0.5], [0.25, 0.4]]
}
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
fn exact(face: usize, uv: [f64; 2]) -> Point3 {
    let [q, t] = uv;
    let (u, v, z) = match face {
        0 => (q, t, 0.),
        1 => (q, t, 1.),
        2 => (q, 0.1, t),
        3 => (0.8, q, t),
        4 => (q, 0.7, t),
        5 => (0.2, q, t),
        6 => (0.35, q, t),
        7 => (q, 0.4, t),
        8 => (0.5, q, t),
        9 => (q, 0.25, t),
        _ => panic!("face"),
    };
    placement().point(Point3::new(
        20. * u,
        12. * v,
        z * (3. - 8. * u * (1. - u) * v * (1. - v)),
    ))
}
#[test]
fn analytic_material_volume_genus_one_topology_and_every_pcurve() {
    let source = source();
    let solid = NurbsGraphHoledSolid::new(&source, hole(), Tolerance::default()).unwrap();
    solid.validate(Tolerance::default()).unwrap();
    assert!(
        (solid.volume().unwrap() - (volume(source.source_domain()) - volume(hole()))).abs() < 1e-10
    );
    assert_eq!(solid.solid.vertices.len(), 16);
    assert_eq!(solid.solid.edges.len(), 24);
    assert_eq!(solid.solid.shell.faces.len(), 10);
    let euler = solid.solid.vertices.len() as isize - solid.solid.edges.len() as isize
        + solid
            .solid
            .shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>();
    assert_eq!(euler, 0);
    assert_eq!(solid.solid.shell.faces[0].wires.len(), 2);
    assert_eq!(solid.solid.shell.faces[1].wires.len(), 2);
    let mut incidence: HashMap<usize, Vec<bool>> = HashMap::new();
    for (f, face) in solid.solid.shell.faces.iter().enumerate() {
        for wire in &face.wires {
            for c in &wire.coedges {
                incidence
                    .entry(c.edge)
                    .or_default()
                    .push(if face.orientation == 1 {
                        c.forward
                    } else {
                        !c.forward
                    });
                let edge = &solid.solid.edges[c.edge];
                let d = edge.curve.range();
                for i in 0..=16 {
                    let t = d[0] + (d[1] - d[0]) * i as f64 / 16.;
                    let uv = c.pcurve.evaluate(t);
                    let p = edge.curve.try_evaluate(t).unwrap();
                    assert!((p - face.surface.try_evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-10);
                    assert!((p - exact(f, uv)).norm() < 1e-10);
                }
            }
        }
    }
    assert_eq!(incidence.len(), 24);
    for use_ in incidence.values() {
        assert_eq!(use_.len(), 2);
        assert_ne!(use_[0], use_[1]);
    }
}
#[test]
fn bounded_caps_exclude_hole_all_walls_match_independent_graph_and_mesh_is_closed() {
    let solid = NurbsGraphHoledSolid::new(&source(), hole(), Tolerance::default()).unwrap();
    let display = solid
        .tessellate_bounded(0.2, 65536, Tolerance::default())
        .unwrap();
    let mut nodes = HashMap::new();
    let normals = [
        Vec3::new(0., 0., -1.),
        Vec3::new(0., 0., 1.),
        Vec3::new(0., -1., 0.),
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(-1., 0., 0.),
        Vec3::new(1., 0., 0.),
        Vec3::new(0., -1., 0.),
        Vec3::new(-1., 0., 0.),
        Vec3::new(0., 1., 0.),
    ];
    for (i, &node) in display.vertex_nodes.iter().enumerate() {
        let f = display.vertex_faces[i];
        let uv = display.vertex_uv[i];
        let p = display.mesh.positions[i];
        assert!((p - exact(f, uv)).norm() < 1e-10);
        if let Some(old) = nodes.insert(node, p) {
            assert_eq!(old, p);
        }
        if f != 1 {
            assert!((display.mesh.normals[i] - placement().vector(normals[f])).norm() < 1e-10);
        }
    }
    let mut incidence: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    let mut counts = [0; 10];
    for (t, ids) in display.mesh.triangles.iter().enumerate() {
        let f = display.vertex_faces[ids[0]];
        counts[f] += 1;
        assert!(ids.iter().all(|i| display.vertex_faces[*i] == f));
        let uv = ids.map(|i| display.vertex_uv[i]);
        let p = ids.map(|i| display.mesh.positions[i]);
        let bound = display.error_bounds[t];
        assert!((0. ..=0.2).contains(&bound));
        for i in 0..=6 {
            for j in 0..=6 - i {
                let w = [i as f64 / 6., j as f64 / 6., 1. - (i + j) as f64 / 6.];
                let at = [0, 1].map(|axis| (0..3).map(|k| w[k] * uv[k][axis]).sum());
                if f < 2 {
                    assert!(
                        !(at[0] > 0.35 + 1e-12
                            && at[0] < 0.5 - 1e-12
                            && at[1] > 0.25 + 1e-12
                            && at[1] < 0.4 - 1e-12)
                    );
                }
                let chord = Point3::new(
                    (0..3).map(|k| w[k] * p[k].x).sum(),
                    (0..3).map(|k| w[k] * p[k].y).sum(),
                    (0..3).map(|k| w[k] * p[k].z).sum(),
                );
                assert!((exact(f, at) - chord).norm() <= bound + 1e-10);
            }
        }
        for i in 0..3 {
            let a = display.vertex_nodes[ids[i]];
            let b = display.vertex_nodes[ids[(i + 1) % 3]];
            let e = incidence.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(counts.iter().all(|c| *c > 0));
    for e in incidence.values() {
        assert_eq!(*e, (2, 0));
    }
    assert!(display.mesh.signed_volume() > 0.);
}
#[test]
fn near_contact_invalid_holes_corruption_and_resolved_thin_material() {
    let s = source();
    for h in [
        [[0.2, 0.5], [0.25, 0.4]],
        [[0.2 + 1e-15, 0.5], [0.25, 0.4]],
        [[0.35, 0.8], [0.25, 0.4]],
        [[0.5, 0.35], [0.25, 0.4]],
        [[f64::NAN, 0.5], [0.25, 0.4]],
    ] {
        assert!(NurbsGraphHoledSolid::new(&s, h, Tolerance::default()).is_err());
    }
    let good = NurbsGraphHoledSolid::new(&s, hole(), Tolerance::default()).unwrap();
    let mut dirty = good.clone();
    dirty.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = good.clone();
    dirty.solid.edges[0].vertices[0] = 999;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = good.clone();
    dirty.solid.vertices[0].point.z += 0.01;
    assert!(dirty.validate(Tolerance::default()).is_err());
    assert!(dirty.volume().is_err());
    assert!(dirty.bounds().is_err());
    assert!(good
        .tessellate_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
    assert!(good
        .tessellate_bounded(0.2, 1, Tolerance::default())
        .is_err());
    let tol = Tolerance::new(1e-12).unwrap();
    let source = NurbsGraphSolid::new([1., 1., 1.], 0., tol).unwrap();
    let narrow =
        NurbsGraphHoledSolid::new(&source, [[1e-8, 1. - 1e-8], [1e-8, 1. - 1e-8]], tol).unwrap();
    assert!(narrow.volume().unwrap() > 0.);
    assert!((narrow.volume().unwrap() - (4e-8 - 4e-16)).abs() < 1e-15);
    let mesh = narrow.tessellate_bounded(0.1, 65536, tol).unwrap();
    assert!((mesh.mesh.signed_volume() - narrow.volume().unwrap()).abs() < 1e-15);
    let mut incidence: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    for ids in &mesh.mesh.triangles {
        for i in 0..3 {
            let a = mesh.vertex_nodes[ids[i]];
            let b = mesh.vertex_nodes[ids[(i + 1) % 3]];
            let edge = incidence.entry((a.min(b), a.max(b))).or_default();
            edge.0 += 1;
            edge.1 += if a < b { 1 } else { -1 };
        }
    }
    for edge in incidence.values() {
        assert_eq!(*edge, (2, 0));
    }
}
