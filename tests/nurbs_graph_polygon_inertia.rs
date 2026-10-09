use hagane::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 3e-11 * b.abs().max(1.), "{a} != {b}");
}
fn quad() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
#[test]
fn independent_flat_triangle_and_box_inertia() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let body =
        NurbsGraphPolygonSolid::new(&source, vec![[0., 0.], [1., 0.], [0., 1.]], tol).unwrap();
    let p = body.inertia_properties(tol).unwrap();
    near(p.volume, 48.);
    near(p.centroid.x, 8. / 3.);
    near(p.centroid.y, 2.);
    for (i, want) in [112., 48. * 35. / 9., 48. * 100. / 18.]
        .into_iter()
        .enumerate()
    {
        near(p.inertia[i][i], want);
    }
    near(p.inertia[0][1], 64.);
    near(p.inertia[0][2], 0.);
    near(p.inertia[1][2], 0.);
    let box_poly = NurbsGraphPolygonSolid::new(&source, quad(), tol)
        .unwrap()
        .inertia_properties(tol)
        .unwrap();
    for (i, want) in [320., 544., 800.].into_iter().enumerate() {
        near(box_poly.inertia[i][i], want);
    }
}
#[test]
fn curved_partition_and_world_rotation() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 20., tol).unwrap();
    let split = source.split_uv_line([0., 0.2], [1., 0.75], tol).unwrap();
    let full = source.inertia_properties(tol).unwrap();
    let a = split.negative.inertia_properties(tol).unwrap();
    let b = split.positive.inertia_properties(tol).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            let sum = [a, b]
                .iter()
                .map(|p| {
                    let d = p.centroid - full.centroid;
                    let v = [d.x, d.y, d.z];
                    p.inertia[i][j] + p.volume * (if i == j { d.dot(d) } else { 0. } - v[i] * v[j])
                })
                .sum::<f64>();
            near(sum, full.inertia[i][j]);
        }
    }
    let body = NurbsGraphPolygonSolid::new(&source, quad(), tol).unwrap();
    let p = body.inertia_properties(tol).unwrap();
    let frame = Transform::rotation(Vec3::new(0., 0., 1.), 0.4).unwrap();
    let moved = source.transformed(frame, tol).unwrap();
    let q = NurbsGraphPolygonSolid::new(&moved, quad(), tol)
        .unwrap()
        .inertia_properties(tol)
        .unwrap();
    let c = 0.4f64.cos();
    let s = 0.4f64.sin();
    near(
        q.inertia[0][0],
        c * c * p.inertia[0][0] + s * s * p.inertia[1][1] - 2. * c * s * p.inertia[0][1],
    );
    near(
        q.inertia[0][1],
        c * s * (p.inertia[0][0] - p.inertia[1][1]) + (c * c - s * s) * p.inertia[0][1],
    );
}
#[test]
fn annulus_public_mutation_and_numeric_limits() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let mut body = source
        .through_uv_polygon(vec![[0.2, 0.3], [0.4, 0.3], [0.4, 0.7], [0.2, 0.7]], tol)
        .unwrap();
    let p = body.inertia_properties(tol).unwrap();
    assert!(p.inertia[0][0] > 0.);
    assert_eq!(p.inertia[0][1], p.inertia[1][0]);
    body.solid.vertices[0].point.x += tol.linear / 10.;
    assert!(body.inertia_properties(tol).is_err());
    for scale in [1e-80, 1e100] {
        let tol = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], 0., tol).unwrap();
        let body = NurbsGraphPolygonSolid::new(&source, quad(), tol).unwrap();
        assert!(body.mass_properties(tol).is_ok());
        assert!(body.inertia_properties(tol).is_err());
    }
}
