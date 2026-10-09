use hagane::*;
use std::collections::HashMap;
fn graph() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], 20., Tolerance::default()).unwrap()
}
#[test]
fn exact_analytic_volume_bounds_and_every_edge_pcurve_are_consistent() {
    let graph = graph();
    graph.validate(Tolerance::default()).unwrap();
    assert!((graph.volume().unwrap() - 240. * (3. + 20. / 9.)).abs() < 1e-10);
    assert_eq!(
        graph.bounds().unwrap(),
        Bounds {
            min: Point3::new(0., 0., 0.),
            max: Point3::new(20., 12., 8.)
        }
    );
    let solid = &graph.solid;
    assert_eq!(solid.vertices.len(), 8);
    assert_eq!(solid.edges.len(), 12);
    assert_eq!(solid.shell.faces.len(), 6);
    let mut uses: HashMap<usize, Vec<bool>> = HashMap::new();
    let mut top_faces = 0;
    for face in &solid.shell.faces {
        assert_eq!(face.wires.len(), 1);
        for c in &face.wires[0].coedges {
            uses.entry(c.edge)
                .or_default()
                .push(if face.orientation == 1 {
                    c.forward
                } else {
                    !c.forward
                });
            let edge = &solid.edges[c.edge];
            let range = edge.curve.range();
            for j in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * j as f64 / 16.;
                let uv = c.pcurve.evaluate(t);
                assert!(
                    (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - edge.curve.try_evaluate(t).unwrap())
                    .norm()
                        < 1e-9
                );
            }
        }
        if let Surface::Nurbs(surface) = &face.surface {
            if surface.evaluate(0.5, 0.5).unwrap().z <= 3. {
                continue;
            }
            top_faces += 1;
            for i in 0..=8 {
                for j in 0..=8 {
                    let u = i as f64 / 8.;
                    let v = j as f64 / 8.;
                    let p = surface.evaluate(u, v).unwrap();
                    assert!(
                        (p - Point3::new(20. * u, 12. * v, 3. + 80. * u * (1. - u) * v * (1. - v)))
                            .norm()
                            < 1e-10
                    );
                }
            }
        }
    }
    assert_eq!(top_faces, 1);
    assert_eq!(uses.len(), 12);
    for use_ in uses.values() {
        assert_eq!(use_.len(), 2);
        assert_ne!(use_[0], use_[1]);
    }
}
#[test]
fn display_is_logically_closed_with_independent_top_bounds_and_mesh_volume() {
    let graph = graph();
    let display = graph
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    assert_eq!(display.error_bounds.len(), display.mesh.triangles.len());
    assert_eq!(display.vertex_nodes.len(), display.mesh.positions.len());
    let mut nodes = HashMap::new();
    for (i, &node) in display.vertex_nodes.iter().enumerate() {
        if let Some(old) = nodes.insert(node, display.mesh.positions[i]) {
            assert_eq!(old, display.mesh.positions[i]);
        }
        assert!((display.mesh.normals[i].norm() - 1.).abs() < 1e-10);
    }
    let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    let mut volume = 0.;
    let mut top_count = 0;
    for (t, ids) in display.mesh.triangles.iter().enumerate() {
        let p = ids.map(|i| display.mesh.positions[i]);
        let normal = (p[1] - p[0]).cross(p[2] - p[0]);
        assert!(normal.norm() > 0.);
        let bound = display.error_bounds[t];
        assert!((0. ..=0.1).contains(&bound));
        volume += p[0].x * (p[1].y * p[2].z - p[1].z * p[2].y) / 6.
            + p[0].y * (p[1].z * p[2].x - p[1].x * p[2].z) / 6.
            + p[0].z * (p[1].x * p[2].y - p[1].y * p[2].x) / 6.;
        if normal.z > 0. && p.iter().all(|p| p.z >= 3.) {
            top_count += 1;
            for a in 0..=6 {
                for b in 0..=6 - a {
                    let f = [a as f64 / 6., b as f64 / 6., 1. - (a + b) as f64 / 6.];
                    let x = (0..3).map(|i| f[i] * p[i].x).sum::<f64>();
                    let y = (0..3).map(|i| f[i] * p[i].y).sum::<f64>();
                    let z = (0..3).map(|i| f[i] * p[i].z).sum::<f64>();
                    let u = x / 20.;
                    let v = y / 12.;
                    assert!((z - (3. + 80. * u * (1. - u) * v * (1. - v))).abs() <= bound + 1e-10);
                }
            }
        }
        for i in 0..3 {
            let a = display.vertex_nodes[ids[i]];
            let b = display.vertex_nodes[ids[(i + 1) % 3]];
            let e = edges.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(top_count > 0);
    for (count, balance) in edges.values() {
        assert_eq!(*count, 2);
        assert_eq!(*balance, 0);
    }
    assert!(volume > 0.);
    assert!((volume - graph.volume().unwrap()).abs() <= 240. * 0.1);
}
#[test]
fn corruption_invalid_inputs_and_precision_are_explicit_errors() {
    let good = graph();
    let mut bad = good.clone();
    bad.solid.shell.faces[0].wires[0].coedges[0].edge = 999;
    assert!(bad.validate(Tolerance::default()).is_err());
    let mut bad = good.clone();
    bad.solid.edges[0].vertices[0] = 999;
    assert!(bad.validate(Tolerance::default()).is_err());
    let mut bad = good.clone();
    bad.solid.vertices[0].point.z += 1.;
    assert!(bad.validate(Tolerance::default()).is_err());
    let mut bad = good.clone();
    bad.solid.shell.faces[0].orientation *= -1;
    assert!(bad.validate(Tolerance::default()).is_err());
    for dims in [
        [0., 1., 1.],
        [-1., 1., 1.],
        [1., f64::NAN, 1.],
        [1., 1., f64::INFINITY],
        [1e-12, 1e-12, 1e-12],
    ] {
        assert!(NurbsGraphSolid::new(dims, 1., Tolerance::default()).is_err());
    }
    for b in [-1., f64::NAN, f64::INFINITY] {
        assert!(NurbsGraphSolid::new([1., 1., 1.], b, Tolerance::default()).is_err());
    }
    assert!(NurbsGraphSolid::new([1e-6, 1e9, 1.], 1., Tolerance::new(1e-12).unwrap()).is_err());
    let signed = NurbsGraphSolid::new([20., 12., 3.], -2., Tolerance::default()).unwrap();
    assert!((signed.volume().unwrap() - 240. * (3. - 2. / 9.)).abs() < 1e-10);
    assert_eq!(signed.bounds().unwrap().max.z, 3.);
    let mut dirty = good.clone();
    let Surface::Nurbs(top) = &dirty.solid.shell.faces[1].surface else {
        panic!("retained rational surface required")
    };
    let mut points = top.control_points().to_vec();
    points[4].z += Tolerance::default().linear / 2.;
    let changed = NurbsSurface::new(
        top.degrees(),
        [
            top.knots(0).unwrap().to_vec(),
            top.knots(1).unwrap().to_vec(),
        ],
        top.control_counts(),
        points,
        top.weights().to_vec(),
    )
    .unwrap();
    dirty.solid.shell.faces[1].surface = Surface::Nurbs(Box::new(changed));
    assert!(dirty.validate(Tolerance::default()).is_err());
    assert!(dirty.volume().is_err());
    assert!(dirty.bounds().is_err());
    let tiny = NurbsGraphSolid::new([1e-9; 3], 1e-9, Tolerance::new(1e-14).unwrap()).unwrap();
    tiny.validate(Tolerance::new(1e-14).unwrap()).unwrap();
    assert!(tiny.volume().unwrap() > 0.);
    assert!(good
        .tessellate_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
    assert!(good
        .tessellate_bounded(0.1, 1, Tolerance::default())
        .is_err());
}
