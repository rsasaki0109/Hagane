use hagane::*;
fn region(scale: f64, t: Tolerance) -> ArcLineRegion {
    ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: rounded_rectangle_profile(
            Point3::new(0., 0., 0.),
            12. * scale,
            10. * scale,
            scale,
            t,
        )
        .unwrap()
        .segments,
        holes: vec![
            rounded_rectangle_profile(
                Point3::new(0., 0., 0.),
                4. * scale,
                3. * scale,
                0.5 * scale,
                t,
            )
            .unwrap()
            .segments,
        ],
    }
}
fn rounded_contains(x: f64, y: f64, width: f64, height: f64, r: f64) -> bool {
    let dx = (x.abs() - (width / 2. - r)).max(0.);
    let dy = (y.abs() - (height / 2. - r)).max(0.);
    dx.hypot(dy) < r
}
#[test]
fn sheared_material_grids_with_curved_holes_and_signed_placement() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    for sign in [-1., 1.] {
        for drift in [[0.75, -0.5], [-2., 1.5]] {
            let d = Vec3::new(drift[0] * 4., drift[1] * 4., sign * 4.);
            let solid =
                extrude_arc_line_region_along(&region(1., t.absolute()), d, t.absolute()).unwrap();
            let placed = solid.transformed(tr, t.absolute()).unwrap();
            for x in -8..=8 {
                for y in -7..=7 {
                    // Avoid profile boundaries; independent membership is the inverse shear.
                    let x = x as f64 * 0.8 + 0.13;
                    let y = y as f64 * 0.8 + 0.17;
                    for fraction in [-0.2, 0.31, 0.79, 1.2] {
                        let expected = if (0. ..1.).contains(&fraction)
                            && rounded_contains(x, y, 12., 10., 1.)
                            && !rounded_contains(x, y, 4., 3., 0.5)
                        {
                            PointLocation::Inside
                        } else {
                            PointLocation::Outside
                        };
                        let p = Point3::new(x + d.x * fraction, y + d.y * fraction, d.z * fraction);
                        assert_eq!(
                            classify_point_in_solid(&solid, p, t).unwrap(),
                            expected,
                            "{p:?}"
                        );
                        assert_eq!(
                            classify_point_in_solid(&placed, tr.point(p), t).unwrap(),
                            expected,
                            "placed {p:?}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn curved_wall_euclidean_bands_normals_caps_and_microscopic_geometry() {
    for (scale, epsilon) in [(1., 1e-5), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        let s = extrude_arc_line_region_along(
            &region(scale, t.absolute()),
            Vec3::new(3. * scale, -2. * scale, 4. * scale),
            t.absolute(),
        )
        .unwrap();
        for face in &s.shell.faces {
            if let Surface::ExtrudedCircle { height, .. } = face.surface {
                let u = 0.37;
                let p = face.surface.evaluate(u, height * 0.41);
                let n = face.surface.normal(u) * face.orientation as f64;
                for (offset, expected) in [
                    (-1.4, PointLocation::Inside),
                    (-0.6, PointLocation::Boundary),
                    (0., PointLocation::Boundary),
                    (0.6, PointLocation::Boundary),
                    (1.4, PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&s, p + n * (offset * epsilon), t).unwrap(),
                        expected,
                        "scale={scale}, face={face:?}, offset={offset}"
                    );
                }
                for v in [0., height] {
                    assert_eq!(
                        classify_point_in_solid(&s, face.surface.evaluate(u, v), t).unwrap(),
                        PointLocation::Boundary
                    );
                }
            }
        }
        for p in [
            Point3::new(4. * scale, 0., 0.),
            Point3::new(7. * scale, -2. * scale, 4. * scale),
        ] {
            assert_eq!(
                classify_point_in_solid(&s, p, t).unwrap(),
                PointLocation::Boundary
            );
        }
    }
}
#[test]
fn generator_vertices_invalid_topology_and_unresolved_precision() {
    let t = GeometryTolerance::default();
    let mut s = extrude_arc_line_region_along(
        &region(1., t.absolute()),
        Vec3::new(3., -2., 4.),
        t.absolute(),
    )
    .unwrap();
    for vertex in &s.vertices {
        assert_eq!(
            classify_point_in_solid(&s, vertex.point, t).unwrap(),
            PointLocation::Boundary
        );
    }
    assert!(classify_point_in_solid(&s, Point3::new(f64::NAN, 0., 0.), t).is_err());
    let huge = s
        .transformed(
            Transform::translation(Vec3::new(1e12, 1e12, 1e12)).unwrap(),
            Tolerance::new(0.01).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        classify_point_in_solid(
            &huge,
            Point3::new(1e12 + 4., 1e12, 1e12 + 2.),
            GeometryTolerance::new(0.001, 1e-10, 0.).unwrap()
        ),
        Err(Error::Unsupported(_))
    ));
    s.shell.faces[3].orientation *= -1;
    assert!(classify_point_in_solid(&s, Point3::new(100., 0., 0.), t).is_err());
}
#[test]
fn skew_rim_band_uses_combined_euclidean_distance() {
    let t = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    let region = ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: vec![
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 2.,
                start_angle: 0.,
                sweep: std::f64::consts::PI,
            },
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 2.,
                start_angle: std::f64::consts::PI,
                sweep: std::f64::consts::PI,
            },
        ],
        holes: vec![],
    };
    let s = extrude_arc_line_region_along(&region, Vec3::new(3., 0., 4.), t.absolute()).unwrap();
    let rim = Point3::new(5., 0., 4.);
    let normal = Vec3::new(0.8, 0., -0.6);
    let generator = Vec3::new(0.6, 0., 0.8);
    for (offset, expected) in [
        (0.6, PointLocation::Boundary),
        (0.8, PointLocation::Outside),
    ] {
        let p = rim + (normal + generator) * (offset * t.absolute().linear);
        assert_eq!(classify_point_in_solid(&s, p, t).unwrap(), expected);
    }
}
