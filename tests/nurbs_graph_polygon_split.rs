use hagane::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 3e-11 * b.abs().max(1.), "{a} != {b}");
}
#[test]
fn oblique_closed_partition_mass_and_shared_cut() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let split = source.split_uv_line([0.1, 0.2], [0.9, 0.7], tol).unwrap();
    split.validate(tol).unwrap();
    let a = split.negative.mass_properties(tol).unwrap();
    let b = split.positive.mass_properties(tol).unwrap();
    let full = source.mass_properties(tol).unwrap();
    near(a.volume + b.volume, full.volume);
    for (a, b, c) in [
        (a.centroid.x, b.centroid.x, full.centroid.x),
        (a.centroid.y, b.centroid.y, full.centroid.y),
        (a.centroid.z, b.centroid.z, full.centroid.z),
    ] {
        near(
            (a * split.negative.volume().unwrap() + b * split.positive.volume().unwrap())
                / full.volume,
            c,
        );
    }
    let Surface::Plane { origin, u, v } = split.plane else {
        panic!()
    };
    let normal = u.cross(v);
    let Surface::Nurbs(ref surface) = split.section.face.surface else {
        panic!()
    };
    for p in surface.control_points() {
        assert!((*p - origin).dot(normal).abs() < 1e-10);
    }
}
#[test]
fn trimmed_signed_placed_and_nested_polygon() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(),
            tol,
        )
        .unwrap();
    let body = NurbsGraphPolygonSolid::new(
        &source,
        vec![[0.2, 0.3], [0.8, 0.25], [0.7, 0.7], [0.3, 0.75]],
        tol,
    )
    .unwrap();
    let split = body.split_uv_line([0.1, 0.5], [0.9, 0.5], tol).unwrap();
    split.validate(tol).unwrap();
    near(
        split.negative.volume().unwrap() + split.positive.volume().unwrap(),
        body.volume().unwrap(),
    );
}
#[test]
fn contacts_invalid_and_public_corruption() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    for (a, b) in [
        ([0., 0.], [1., 1.]),
        ([0.2, 0.2], [0.2, 0.2]),
        ([0., 2.], [1., 2.]),
        ([f64::NAN, 0.], [1., 0.]),
        ([1e12, 1e12], [1e12 + 1., 1e12 + 1.]),
    ] {
        assert!(source.split_uv_line(a, b, tol).is_err());
    }
    let mut split = source.split_uv_line([0.5, 0.], [0.5, 1.], tol).unwrap();
    split.section.vertices[0].point.x += tol.linear / 10.;
    assert!(split.validate(tol).is_err());
}
