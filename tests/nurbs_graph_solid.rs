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
