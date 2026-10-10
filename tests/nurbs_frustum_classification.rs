use hagane::*;

fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-5 * s, 1e-10, 0.).unwrap()
}
fn pose(s: f64) -> Frame3 {
    Transform::translation(Vec3::new(12. * s, -3. * s, 5. * s))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap()
}
// A rotationally symmetric solid's closest boundary reduces to the finite
// meridian wall segment and the two closed radial cap segments. This oracle
// uses physical distances, without the production parameterization.
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
fn oracle(r: [f64; 2], h: f64, p: Point3, band: f64) -> PointLocation {
    let q = [p.x.hypot(p.y), p.z];
    let distance = segment_distance(q, [r[0], 0.], [r[1], h])
        .min(segment_distance(q, [0., 0.], [r[0], 0.]))
        .min(segment_distance(q, [0., h], [r[1], h]));
    if distance <= band {
        PointLocation::Boundary
    } else if p.z > 0. && p.z < h && q[0] < r[0] + (r[1] - r[0]) * p.z / h {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

#[test]
fn finite_wall_and_cap_distance_oracle_across_tapers_frames_and_scales() {
    for s in [1e-4, 1., 10.] {
        for radii in [[12., 6.], [6., 12.], [6., 6.], [0.3, 0.1]] {
            let r = radii.map(|v| v * s);
            let h = 20. * s;
            let tol = policy(s);
            for frame in [Frame3::IDENTITY, pose(s)] {
                let solid = NurbsFrustumSolid::new(frame, r, h, tol).unwrap();
                for zi in -2..=12 {
                    for ri in 0..=15 {
                        let z = zi as f64 * h / 10.;
                        let rho = ri as f64 * r[0].max(r[1]) / 10.;
                        for angle in [0.17_f64, 1.83, 4.1] {
                            let p = Point3::new(rho * angle.cos(), rho * angle.sin(), z);
                            assert_eq!(
                                solid.classify_point(frame.point(p), tol).unwrap(),
                                oracle(r, h, p, tol.linear()),
                                "r={r:?}, p={p:?}"
                            );
                        }
                    }
                }
                let original = format!("{:?}", solid.solid());
                assert_eq!(
                    solid
                        .classify_point(frame.point(Point3::new(0., 0., h / 2.)), tol)
                        .unwrap(),
                    PointLocation::Inside
                );
                assert_eq!(format!("{:?}", solid.solid()), original);
            }
        }
    }
}

#[test]
fn actual_surface_normal_offsets_and_diagonal_rim_use_euclidean_band() {
    for radii in [[12., 6.], [6., 12.], [6., 6.]] {
        let tol = policy(1.);
        let solid = NurbsFrustumSolid::new(pose(1.), radii, 20., tol).unwrap();
        for face in &solid.solid().shell.faces[2..] {
            let Surface::Nurbs(surface) = &face.surface else {
                panic!()
            };
            for u in [0.13, 0.51, 0.89] {
                for v in [0.2, 0.7] {
                    let p = surface.evaluate(u, v).unwrap();
                    let n = surface.normal(u, v).unwrap();
                    for (offset, expected) in [
                        (0.4, PointLocation::Boundary),
                        (-0.4, PointLocation::Boundary),
                        (2., PointLocation::Outside),
                        (-2., PointLocation::Inside),
                    ] {
                        assert_eq!(
                            solid
                                .classify_point(p + n * (offset * tol.linear()), tol)
                                .unwrap(),
                            expected
                        );
                    }
                }
            }
        }
        // Both radial and axial offsets lie inside separate coordinate bands,
        // but their diagonal distance to the finite rim exceeds the band.
        for (r, z, sign) in [(radii[0], 0., -1.), (radii[1], 20., 1.)] {
            for a in [0.4, 0.8] {
                let p = Point3::new(r + a * tol.linear(), 0., z + sign * a * tol.linear());
                let expected = oracle(radii, 20., p, tol.linear());
                assert_eq!(
                    expected,
                    if a == 0.4 {
                        PointLocation::Boundary
                    } else {
                        PointLocation::Outside
                    }
                );
                assert_eq!(
                    solid.classify_point(pose(1.).point(p), tol).unwrap(),
                    expected
                );
            }
        }
    }
}

#[test]
fn full_relative_policy_and_invalid_queries_preserve_scope_and_body() {
    let tol = policy(1.);
    let solid = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 6.], 20., tol).unwrap();
    let original = format!("{:?}", solid.solid());
    let relative = GeometryTolerance::new(1e-5, 1e-10, 1e-4).unwrap();
    assert_eq!(
        solid
            .classify_point(Point3::new(0., 0., -0.001), relative)
            .unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        solid
            .classify_point(Point3::new(0., 0., -0.001), tol)
            .unwrap(),
        PointLocation::Outside
    );
    let unresolved = GeometryTolerance::new(1e-5, 1e-10, 0.2).unwrap();
    assert!(solid
        .classify_point(Point3::new(0., 0., 10.), unresolved)
        .is_err());
    for p in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(f64::INFINITY, 0., 0.),
        Point3::new(1e300, 0., 0.),
    ] {
        assert!(solid.classify_point(p, tol).is_err());
    }
    assert!(matches!(
        classify_point_in_solid(solid.solid(), Point3::new(0., 0., 10.), tol),
        Err(Error::Unsupported(_))
    ));
    let mut corrupted = solid.solid().clone();
    corrupted.shell.faces[2].orientation *= -1;
    assert!(
        NurbsFrustumSolid::from_brep(corrupted, Frame3::IDENTITY, [12., 6.], 20., tol).is_err()
    );
    assert_eq!(format!("{:?}", solid.solid()), original);
}
