use hagane::*;
#[test]
fn flat_and_symmetric_graph_centroids_match_analytic_column_moments() {
    let tol = Tolerance::default();
    for b in [-12., 0., 36.] {
        let body = NurbsGraphSolid::new([80., 60., 24.], b, tol).unwrap();
        let properties = body.mass_properties(tol).unwrap();
        assert_eq!(properties.volume, body.volume().unwrap());
        assert!((properties.centroid.x - 40.).abs() < 1e-12);
        assert!((properties.centroid.y - 30.).abs() < 1e-12);
        let z = (24f64.powi(2) + 2. * 24. * b / 9. + 4. * b * b / 225.) / (2. * (24. + b / 9.));
        assert!((properties.centroid.z - z).abs() < 1e-12);
    }
}
#[test]
fn asymmetric_hole_trim_and_placement_preserve_independent_flat_box_centroids() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let hole = [[0.1, 0.3], [0.2, 0.5]];
    let body = NurbsGraphHoledSolid::new(&source, hole, tol).unwrap();
    let p = body.mass_properties(tol).unwrap();
    let area = 8. * 6.;
    let removed = (8. * 0.2) * (6. * 0.3);
    let expected = Point3::new(
        (area * 4. - removed * 1.6) / (area - removed),
        (area * 3. - removed * 2.1) / (area - removed),
        1.,
    );
    assert!((p.centroid - expected).norm() < 1e-12);
    let transform = Transform::translation(Vec3::new(12., -7., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let placed_source = source.transformed(transform, tol).unwrap();
    let placed = NurbsGraphHoledSolid::new(&placed_source, hole, tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    assert!((placed.centroid - transform.point(p.centroid)).norm() < 1e-12);
    assert_eq!(placed.volume, p.volume);
    let trimmed = source
        .trimmed_uv([[0.1, 0.9], [0.2, 0.7]], tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    assert!((trimmed.centroid - Point3::new(4., 2.7, 1.)).norm() < 1e-12);
}
#[test]
fn tiny_and_large_local_moments_work_without_squared_height_or_world_moments() {
    for scale in [1e-100, 1e100] {
        let tol = Tolerance::new(scale * 1e-10).unwrap();
        let source =
            NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], scale, tol).unwrap();
        let p = source.mass_properties(tol).unwrap();
        assert!(p.volume.is_finite() && p.volume > 0.);
        assert!((p.centroid.x / scale - 4.).abs() < 1e-12);
        assert!((p.centroid.y / scale - 3.).abs() < 1e-12);
    }
    let tol = Tolerance::default();
    let mut source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    source.solid.vertices[0].point.x = f64::NAN;
    assert!(source.mass_properties(tol).is_err());
}
