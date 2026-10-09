use hagane::*;
#[test]
fn graph_volume_topology_and_mesh() {
    let tol = Tolerance::default();
    for b in [-12., 0., 36.] {
        let g = NurbsGraphSolid::new([80., 60., 24.], b, tol).unwrap();
        assert_eq!(g.volume().unwrap(), 80. * 60. * (24. + b / 9.));
        g.validate(tol).unwrap();
        assert!(g.brep().validate(tol).is_err());
        let m = g.tessellate_bounded(0.1, 65536, tol).unwrap();
        assert!(m.error_bounds.iter().all(|x| *x <= 0.1));
        let mut edges = std::collections::BTreeMap::<[usize; 2], (usize, i32)>::new();
        for t in &m.mesh.triangles {
            for i in 0..3 {
                let a = m.vertex_nodes[t[i]];
                let b = m.vertex_nodes[t[(i + 1) % 3]];
                let e = edges.entry([a.min(b), a.max(b)]).or_default();
                e.0 += 1;
                e.1 += if a < b { 1 } else { -1 };
            }
        }
        assert!(edges.values().all(|e| *e == (2, 0)));
        assert!(m.mesh.signed_volume() > 0.);
    }
}
#[test]
fn graph_rejects_invalid_and_mutated_brep() {
    let tol = Tolerance::default();
    for (d, b) in [
        ([0., 1., 1.], 0.),
        ([1., 1., 1.], -1.),
        ([f64::INFINITY, 1., 1.], 0.),
        ([1., 1., 1.], f64::NAN),
    ] {
        assert!(NurbsGraphSolid::new(d, b, tol).is_err());
    }
    let mut g = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    assert!(g.tessellate_bounded(0.0001, 6, tol).is_err());
    g.solid.shell.faces[0].wires[0].coedges[0].edge = 11;
    assert!(g.validate(tol).is_err());
    assert!(g.tessellate_bounded(0.1, 65536, tol).is_err());
}
#[test]
fn analytic_queries_reject_even_subtolerance_control_edits() {
    let tol = Tolerance::default();
    let mut graph = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let Surface::Nurbs(surface) = &graph.solid.shell.faces[1].surface else {
        panic!()
    };
    let mut points = surface.control_points().to_vec();
    points[4].z += tol.linear * 0.25;
    graph.solid.shell.faces[1].surface = Surface::Nurbs(Box::new(
        NurbsSurface::new(
            surface.degrees(),
            [
                surface.knots(0).unwrap().to_vec(),
                surface.knots(1).unwrap().to_vec(),
            ],
            [3, 3],
            points,
            surface.weights().to_vec(),
        )
        .unwrap(),
    ));
    assert!(graph.validate(tol).is_err());
    assert!(graph.volume().is_err());
    assert!(graph.bounds().is_err());
}
#[test]
fn graph_rigid_placement_retains_exact_canonical_brep_and_closed_mesh() {
    let tol = Tolerance::default();
    let local = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let translation = Transform::translation(Vec3::new(12., -7., 4.)).unwrap();
    let first = local.transformed(rotation, tol).unwrap();
    let placed = first.transformed(translation, tol).unwrap();
    let direct = local
        .transformed(translation.compose(rotation).unwrap(), tol)
        .unwrap();
    assert_eq!(placed.placement(), direct.placement());
    for (a, b) in placed.solid.vertices.iter().zip(&direct.solid.vertices) {
        assert_eq!(a.point, b.point);
    }
    assert_eq!(local.volume().unwrap(), placed.volume().unwrap());
    let bounds = placed.bounds().unwrap();
    for face in &placed.solid.shell.faces {
        let Surface::Nurbs(surface) = &face.surface else {
            panic!()
        };
        for i in 0..21 {
            for j in 0..21 {
                let p = surface.evaluate(i as f64 / 20., j as f64 / 20.).unwrap();
                assert!(
                    p.x >= bounds.min.x - 1e-12
                        && p.x <= bounds.max.x + 1e-12
                        && p.y >= bounds.min.y - 1e-12
                        && p.y <= bounds.max.y + 1e-12
                        && p.z >= bounds.min.z - 1e-12
                        && p.z <= bounds.max.z + 1e-12
                );
            }
        }
    }
    let mesh = placed.tessellate_bounded(0.01, 65536, tol).unwrap();
    let mut positions = std::collections::BTreeMap::new();
    let mut edges = std::collections::BTreeMap::<[usize; 2], (usize, i32)>::new();
    for (i, &node) in mesh.vertex_nodes.iter().enumerate() {
        if let Some(p) = positions.insert(node, mesh.mesh.positions[i]) {
            assert_eq!(p, mesh.mesh.positions[i]);
        }
    }
    for t in &mesh.mesh.triangles {
        for i in 0..3 {
            let a = mesh.vertex_nodes[t[i]];
            let b = mesh.vertex_nodes[t[(i + 1) % 3]];
            let entry = edges.entry([a.min(b), a.max(b)]).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|e| *e == (2, 0)));
}
#[test]
fn graph_placement_precision_is_explicit_and_scale_aware() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    assert!(graph
        .transformed(
            Transform::translation(Vec3::new(1e12, 0., 0.)).unwrap(),
            tol
        )
        .is_err());
    let tiny_tol = Tolerance::new(1e-20).unwrap();
    let tiny = NurbsGraphSolid::new([8e-12, 6e-12, 2e-12], 1e-12, tiny_tol).unwrap();
    let tiny = tiny
        .transformed(
            Transform::translation(Vec3::new(1e-11, -1e-11, 2e-11)).unwrap(),
            tiny_tol,
        )
        .unwrap();
    tiny.validate(tiny_tol).unwrap();
    assert!(tiny.tessellate_bounded(1e-13, 65536, tiny_tol).is_ok());
}
