use hagane::*;
#[test]
fn exact_opening_is_closed_brep_and_closed_bounded_mesh() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol).unwrap();
    let hole = [[0.3, 0.7], [0.2, 0.6]];
    let solid = NurbsGraphHoledSolid::new(&source, hole, tol).unwrap();
    solid.validate(tol).unwrap();
    assert_eq!(solid.solid.vertices.len(), 16);
    assert_eq!(solid.solid.edges.len(), 24);
    assert_eq!(solid.solid.shell.faces.len(), 10);
    let expected =
        source.volume().unwrap() - source.trimmed_uv(hole, tol).unwrap().volume().unwrap();
    assert!((solid.volume().unwrap() - expected).abs() < 1e-10);
    assert!(solid.solid.validate(tol).is_err());
    let display = solid.tessellate_bounded(0.1, 65536, tol).unwrap();
    assert_eq!(
        display.mesh.triangles.len(),
        64 * display.subdivisions.pow(2)
    );
    let mut edges = std::collections::BTreeMap::<[usize; 2], (usize, i32)>::new();
    let mut positions = std::collections::BTreeMap::new();
    for (index, &node) in display.vertex_nodes.iter().enumerate() {
        if let Some(previous) = positions.insert(node, display.mesh.positions[index]) {
            assert_eq!(previous, display.mesh.positions[index]);
        }
    }
    for (index, tri) in display.mesh.triangles.iter().enumerate() {
        for k in 0..3 {
            let a = display.vertex_nodes[tri[k]];
            let b = display.vertex_nodes[tri[(k + 1) % 3]];
            let edge = edges.entry([a.min(b), a.max(b)]).or_default();
            edge.0 += 1;
            edge.1 += if a < b { 1 } else { -1 };
        }
        let face = display.mesh.face_ids[index];
        let Surface::Nurbs(surface) = &solid.solid.shell.faces[face].surface else {
            panic!()
        };
        let uv = [
            (display.vertex_uv[tri[0]][0]
                + display.vertex_uv[tri[1]][0]
                + display.vertex_uv[tri[2]][0])
                / 3.,
            (display.vertex_uv[tri[0]][1]
                + display.vertex_uv[tri[1]][1]
                + display.vertex_uv[tri[2]][1])
                / 3.,
        ];
        let bary = (display.mesh.positions[tri[0]]
            + display.mesh.positions[tri[1]]
            + display.mesh.positions[tri[2]])
            * (1. / 3.);
        assert!(
            (surface.evaluate(uv[0], uv[1]).unwrap() - bary).norm()
                <= display.error_bounds[index] + 1e-12
        );
    }
    assert!(edges.values().all(|e| *e == (2, 0)));
    assert!(display.error_bounds.iter().all(|b| *b <= 0.1));
}
#[test]
fn opening_preserves_placement_and_rejects_contacts_and_public_edits() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(12., -7., 4.)).unwrap(),
            tol,
        )
        .unwrap();
    let hole = [[0.3, 0.7], [0.2, 0.6]];
    let original = NurbsGraphHoledSolid::new(&source, hole, tol).unwrap();
    original.tessellate_bounded(0.01, 65536, tol).unwrap();
    for bad in [
        [[0.1, 0.7], [0.2, 0.6]],
        [[0.3, 0.7], [0.2, 0.9]],
        [[0.5, 0.5], [0.2, 0.6]],
        [[f64::NAN, 0.7], [0.2, 0.6]],
    ] {
        assert!(NurbsGraphHoledSolid::new(&source, bad, tol).is_err());
    }
    let mut dirty = original.clone();
    dirty.solid.vertices[8].point.x += tol.linear / 4.;
    assert!(dirty.validate(tol).is_err());
    assert!(dirty.volume().is_err());
    assert!(dirty.bounds().is_err());
    assert!(dirty.tessellate_bounded(0.1, 65536, tol).is_err());
    assert!(original.tessellate_bounded(1e-8, 32, tol).is_err());
}
#[test]
fn thin_material_volume_uses_positive_strips_and_precision_limits_are_explicit() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 24.], 0., tol).unwrap();
    let epsilon = 1e-5;
    let holed = NurbsGraphHoledSolid::new(
        &source,
        [[epsilon, 1. - epsilon], [epsilon, 1. - epsilon]],
        tol,
    )
    .unwrap();
    let expected = 80. * 60. * 24. * (4. * epsilon - 4. * epsilon * epsilon);
    assert!((holed.volume().unwrap() - expected).abs() < 1e-10);
    assert!(holed.tessellate_bounded(1e-12, 65536, tol).is_err());
    let mesh = holed.tessellate_bounded(0.1, 32, tol).unwrap();
    assert_eq!(mesh.mesh.triangles.len(), 64);
    assert!(NurbsGraphHoledSolid::new(&source, [[1e-12, 0.7], [0.2, 0.6]], tol).is_err());
}
