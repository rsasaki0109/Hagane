use hagane::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 3e-12 * b.abs().max(1e-250), "{a} {b}");
}
#[test]
fn flat_triangle_and_public_mutation() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let mut body =
        NurbsGraphPolygonSolid::new(&source, vec![[0.1, 0.2], [0.8, 0.2], [0.3, 0.9]], tol)
            .unwrap();
    let p = body.mass_properties(tol).unwrap();
    near(p.volume, 8. * 6. * 2. * 0.7 * 0.7 / 2.);
    near(p.centroid.x, 8. * 1.2 / 3.);
    near(p.centroid.y, 6. * 1.3 / 3.);
    near(p.centroid.z, 1.);
    assert_eq!(body.volume().unwrap(), p.volume);
    body.solid.vertices[0].point.x += 1e-10;
    assert!(body.mass_properties(tol).is_err());
    assert!(body.volume().is_err());
}
#[test]
fn full_quad_curved_and_triangle_partition() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], -2., tol).unwrap();
    let body =
        NurbsGraphPolygonSolid::new(&source, vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], tol)
            .unwrap();
    let p = body.mass_properties(tol).unwrap();
    let q = source.mass_properties(tol).unwrap();
    near(p.volume, q.volume);
    near(p.centroid.z, q.centroid.z);
    let a = NurbsGraphPolygonSolid::new(&source, vec![[0., 0.], [1., 0.], [1., 1.]], tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    let b = NurbsGraphPolygonSolid::new(&source, vec![[0., 0.], [1., 1.], [0., 1.]], tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    near(a.volume + b.volume, p.volume);
    near(
        (a.centroid.x * a.volume + b.centroid.x * b.volume) / p.volume,
        p.centroid.x,
    );
    near(
        (a.centroid.z * a.volume + b.centroid.z * b.volume) / p.volume,
        p.centroid.z,
    );
}
#[test]
fn trimmed_placed_micro_and_large_scaling() {
    for scale in [1e-50, 1., 1e50] {
        let tol = Tolerance::new(scale * 1e-9).unwrap();
        let source = NurbsGraphSolid::new([8. * scale, 6. * scale, 3. * scale], 2. * scale, tol)
            .unwrap()
            .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)
            .unwrap();
        let corners = vec![[0.2, 0.3], [0.8, 0.3], [0.5, 0.7]];
        let p = NurbsGraphPolygonSolid::new(&source, corners.clone(), tol)
            .unwrap()
            .mass_properties(tol)
            .unwrap();
        assert!(p.volume > 0.);
        let t = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
        let placed = source.transformed(t, tol).unwrap();
        let q = NurbsGraphPolygonSolid::new(&placed, corners, tol)
            .unwrap()
            .mass_properties(tol)
            .unwrap();
        near(q.volume, p.volume);
        assert!((q.centroid - t.point(p.centroid)).norm() < scale * 1e-12);
    }
}
