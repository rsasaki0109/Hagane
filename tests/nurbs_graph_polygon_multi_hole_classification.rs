use hagane::*;
fn holes() -> Vec<Vec<[f64; 2]>> {
    vec![
        vec![[0.2, 0.2], [0.35, 0.2], [0.35, 0.35], [0.2, 0.35]],
        vec![[0.55, 0.2], [0.7, 0.2], [0.7, 0.35], [0.55, 0.35]],
        vec![[0.2, 0.55], [0.35, 0.55], [0.35, 0.7], [0.2, 0.7]],
        vec![[0.55, 0.55], [0.7, 0.55], [0.7, 0.7], [0.55, 0.7]],
    ]
}
#[test]
fn every_void_removed_cap_and_material_between_two_through_four_holes() {
    let tol = Tolerance::default();
    let gt = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    let source = NurbsGraphSolid::new([10., 10., 3.], 2., tol).unwrap();
    for count in 2..=4 {
        let body = source
            .through_uv_polygons(holes()[..count].to_vec(), tol)
            .unwrap();
        for hole in body.openings() {
            let u = (hole[0][0] + hole[2][0]) / 2.;
            let v = (hole[0][1] + hole[2][1]) / 2.;
            let h = 3. + 8. * u * (1. - u) * v * (1. - v);
            for z in [0., 1., h] {
                assert_eq!(
                    body.classify_point(Point3::new(10. * u, 10. * v, z), gt)
                        .unwrap(),
                    PointLocation::Outside
                );
            }
            let u = hole[0][0];
            let v = (hole[0][1] + hole[3][1]) / 2.;
            assert_eq!(
                body.classify_point(Point3::new(10. * u, 10. * v, 1.), gt)
                    .unwrap(),
                PointLocation::Boundary
            );
        }
        assert_eq!(
            body.classify_point(Point3::new(4.5, 4.5, 1.), gt).unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn finite_corner_euclidean_band_and_placed_signed_trim() {
    let tol = Tolerance::default();
    let gt = GeometryTolerance::new(1e-4, 1e-10, 0.).unwrap();
    let source = NurbsGraphSolid::new([10., 10., 3.], 0., tol).unwrap();
    let body = source
        .through_uv_polygons(holes()[..2].to_vec(), tol)
        .unwrap();
    for (point, want) in [
        (Point3::new(-0.8e-4, -0.8e-4, 1.), PointLocation::Outside),
        (Point3::new(-0.5e-4, -0.5e-4, 1.), PointLocation::Boundary),
        (
            Point3::new(2. - 0.8e-4, 2. - 0.8e-4, 1.),
            PointLocation::Inside,
        ),
        (
            Point3::new(2. - 0.5e-4, 2. - 0.5e-4, 1.),
            PointLocation::Boundary,
        ),
    ] {
        assert_eq!(body.classify_point(point, gt).unwrap(), want);
    }
    let frame = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let source = NurbsGraphSolid::new([10., 10., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], tol)
        .unwrap()
        .transformed(frame, tol)
        .unwrap();
    let body = source.through_uv_polygons(holes(), tol).unwrap();
    for (p, want) in [
        (Point3::new(4.5, 4.5, 1.), PointLocation::Inside),
        (Point3::new(6., 6., 0.), PointLocation::Outside),
        (Point3::new(2., 2.7, 1.), PointLocation::Boundary),
    ] {
        assert_eq!(body.classify_point(frame.point(p), gt).unwrap(), want);
    }
}
#[test]
fn full_sixty_four_corners_and_canonical_failures() {
    let tol = Tolerance::default();
    let gt = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let circle = |c: [f64; 2], r: f64, n: usize, p: f64| {
        (0..n)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64 + p;
                [c[0] + r * a.cos(), c[1] + r * a.sin()]
            })
            .collect::<Vec<_>>()
    };
    let outer =
        NurbsGraphPolygonSolid::new(&source, circle([0.5, 0.5], 0.49, 16, 0.01), tol).unwrap();
    let mut body = outer
        .through_uv_polygons(
            vec![
                circle([0.3, 0.3], 0.06, 16, 0.03),
                circle([0.7, 0.3], 0.06, 16, 0.12),
                circle([0.5, 0.7], 0.06, 16, 0.07),
            ],
            tol,
        )
        .unwrap();
    assert_eq!(
        body.classify_point(Point3::new(40., 30., 1.), gt).unwrap(),
        PointLocation::Inside
    );
    assert_eq!(
        body.classify_point(Point3::new(24., 18., 0.), gt).unwrap(),
        PointLocation::Outside
    );
    assert!(body
        .classify_point(Point3::new(f64::NAN, 0., 0.), gt)
        .is_err());
    body.solid.shell.faces[1].wires[2].coedges[0].edge = usize::MAX;
    assert!(body
        .classify_point(Point3::new(-10., -10., 0.), gt)
        .is_err());
}
