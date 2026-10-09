use hagane::*;
fn surface() -> NurbsSurface {
    let k = vec![0., 0., 1., 1.];
    NurbsSurface::new(
        [1, 1],
        [k.clone(), k],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
        ],
        vec![1.; 4],
    )
    .unwrap()
}
fn outer() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
#[test]
fn sixteen_openings_keep_distinct_topology_and_resource_limit() {
    let mut holes = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            let u = 0.1 + i as f64 * 0.2;
            let v = 0.1 + j as f64 * 0.2;
            holes.push([[u, u + 0.05], [v, v + 0.05]]);
        }
    }
    let face =
        NurbsPolygonHoledFace::new(surface(), outer(), holes.clone(), 1, Tolerance::default())
            .unwrap();
    assert_eq!(face.face.wires.len(), 17);
    assert_eq!(face.vertices.len(), 68);
    assert_eq!(face.edges.len(), 68);
    let display = face
        .tessellate_bounded(0.01, 65536, Tolerance::default())
        .unwrap();
    assert!(!display.mesh.triangles.is_empty());
    holes.push(holes[0]);
    assert!(matches!(
        NurbsPolygonHoledFace::new(surface(), outer(), holes, 1, Tolerance::default()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn uv_separation_is_independent_of_physical_tolerance() {
    let tol = Tolerance::new(0.01).unwrap();
    let face = NurbsPolygonHoledFace::new(
        surface(),
        outer(),
        vec![[[0.2, 0.3], [0.2, 0.3]], [[0.3001, 0.4], [0.2, 0.3]]],
        1,
        tol,
    )
    .unwrap();
    assert_eq!(face.face.wires.len(), 3);
    assert_eq!(face.holes().len(), 2);
    face.tessellate_bounded(0.01, 65536, tol).unwrap();
}
