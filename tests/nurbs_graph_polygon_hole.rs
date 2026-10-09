use hagane::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 5e-12 * b.abs().max(1e-250), "{a} != {b}");
}
fn hole() -> Vec<[f64; 2]> {
    vec![[0.2, 0.3], [0.7, 0.3], [0.6, 0.7], [0.3, 0.8]]
}
#[test]
fn exact_genus_one_shared_geometry_and_mass() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let body = source.through_uv_polygon(hole(), tol).unwrap();
    body.validate(tol).unwrap();
    assert_eq!(
        (
            body.solid.vertices.len(),
            body.solid.edges.len(),
            body.solid.shell.faces.len()
        ),
        (16, 24, 10)
    );
    let mut incidence = [[0i32; 2]; 24];
    for face in &body.solid.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                incidence[c.edge][0] += 1;
                incidence[c.edge][1] += face.orientation as i32 * if c.forward { 1 } else { -1 };
                for k in 0..101 {
                    let t = k as f64 / 100.;
                    let uv = c.pcurve.evaluate(t);
                    assert!(
                        (body.solid.edges[c.edge].curve.try_evaluate(t).unwrap()
                            - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                            < 1e-11
                    );
                }
            }
        }
    }
    assert!(incidence.iter().all(|x| *x == [2, 0]));
    let removed = NurbsGraphPolygonSolid::new(&source, hole(), tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    let full = source.mass_properties(tol).unwrap();
    let actual = body.mass_properties(tol).unwrap();
    near(actual.volume, full.volume - removed.volume);
    near(
        actual.centroid.x,
        (full.volume * full.centroid.x - removed.volume * removed.centroid.x) / actual.volume,
    );
    near(
        actual.centroid.z,
        (full.volume * full.centroid.z - removed.volume * removed.centroid.z) / actual.volume,
    );
    assert_eq!(body.volume().unwrap(), actual.volume);
    assert!(body.brep().volume().is_err());
}
#[test]
fn thin_material_and_placed_signed_scales() {
    for scale in [1e-50, 1., 1e50] {
        let tol = Tolerance::new(scale * 1e-8).unwrap();
        let source =
            NurbsGraphSolid::new([8. * scale, 6. * scale, 3. * scale], -2. * scale, tol).unwrap();
        let opening = vec![
            [1e-6, 1e-6],
            [1. - 1e-6, 1e-6],
            [1. - 1e-6, 1. - 1e-6],
            [1e-6, 1. - 1e-6],
        ];
        let body = source.through_uv_polygon(opening.clone(), tol).unwrap();
        let p = body.mass_properties(tol).unwrap();
        assert!(p.volume > 0.);
        let t = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
        let moved = source
            .transformed(t, tol)
            .unwrap()
            .through_uv_polygon(opening, tol)
            .unwrap()
            .mass_properties(tol)
            .unwrap();
        near(moved.volume, p.volume);
        assert!((moved.centroid - t.point(p.centroid)).norm() < scale * 1e-11);
    }
}
#[test]
fn invalid_opening_and_exact_corruption() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let mut cw = hole();
    cw.reverse();
    for opening in [
        cw,
        vec![[0., 0.3], [0.5, 0.3], [0.3, 0.7]],
        vec![[0.2, 0.2], [1.1, 0.3], [0.3, 0.7]],
        vec![[0.2, 0.2], [0.8, 0.2], [0.4, 0.3], [0.4, 0.8]],
    ] {
        assert!(source.through_uv_polygon(opening, tol).is_err());
    }
    let mut body = source.through_uv_polygon(hole(), tol).unwrap();
    body.solid.shell.faces[0].wires[1].coedges[0].forward =
        !body.solid.shell.faces[0].wires[1].coedges[0].forward;
    assert!(body.validate(tol).is_err());
    assert!(body.volume().is_err());
    assert!(body.bounds().is_err());
}

#[test]
fn almost_collinear_triangulation_bridge_is_repaired_without_moving_geometry() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let outer = NurbsGraphPolygonSolid::new(
        &source,
        vec![
            [0.1, 0.1],
            [0.9, 0.1],
            [0.95, 0.7],
            [0.5, 0.95],
            [0.05, 0.7],
        ],
        tol,
    )
    .unwrap();
    let body = outer
        .through_uv_polygon(vec![[0.4, 0.3], [0.65, 0.5], [0.35, 0.6]], tol)
        .unwrap();
    body.validate(tol).unwrap();
    assert_eq!(body.opening(), &[[0.4, 0.3], [0.65, 0.5], [0.35, 0.6]]);
    body.tessellate_bounded(0.5, 65536, tol).unwrap();
}
#[test]
fn custom_pentagon_and_diamond_annulus() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let outer = NurbsGraphPolygonSolid::new(
        &source,
        vec![
            [0.15, 0.25],
            [0.65, 0.1],
            [0.9, 0.45],
            [0.65, 0.85],
            [0.2, 0.8],
        ],
        tol,
    )
    .unwrap();
    let hole = outer
        .through_uv_polygon(vec![[0.3, 0.5], [0.5, 0.3], [0.7, 0.5], [0.5, 0.7]], tol)
        .unwrap();
    hole.validate(tol).unwrap();
    hole.tessellate_bounded(0.5, 65536, tol).unwrap();
}
