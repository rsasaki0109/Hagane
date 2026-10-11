use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn source(s: f64) -> NurbsTorusSolid {
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), policy(s))
            .unwrap();
    NurbsTorusSolid::new(frame, [16. * s, 4. * s], policy(s)).unwrap()
}
fn witness(
    child: &NurbsTorusHorizontalSolid,
    point: Point3,
    normal: Vec3,
    p: GeometryTolerance,
    s: f64,
) {
    let q = child.point_query(point, p).unwrap();
    assert_eq!(q.location, PointLocation::Boundary);
    let w = q
        .boundary
        .expect("accepted boundary must identify actual retained face");
    let face = &child.solid().shell.faces[w.face];
    let evaluated = face.surface.try_evaluate(w.uv[0], w.uv[1]).unwrap();
    assert!((evaluated - w.point).norm() < 1e-8 * s);
    assert!((w.distance - (point - w.point).norm()).abs() < 1e-8 * s);
    assert!(w.distance <= p.length_at_scale(40_f64.hypot(8.) * s).unwrap());
    assert!(w.normal.dot(normal) > 1. - 1e-8);
    let actualnormal = face.surface.normal_at(w.uv[0], w.uv[1]).unwrap() * face.orientation as f64;
    assert!(w.normal.dot(actualnormal) > 1. - 1e-8);
    if let Surface::Nurbs(net) = &face.surface {
        let d = net.domain();
        assert!(
            w.uv[0] >= d[0][0] && w.uv[0] <= d[0][1] && w.uv[1] >= d[1][0] && w.uv[1] <= d[1][1]
        );
    }
}
#[test]
fn actual_restricted_walls_and_annular_caps_have_true_euclidean_normal_witnesses() {
    for s in [1e-4, 1., 10.] {
        let body = source(s);
        let p = policy(s);
        let before = format!("{:?}", body.solid());
        let step = body.export_step_mm(p).unwrap();
        for cut in [0., 1.25, -1.25] {
            let split = body.split_by_local_z(cut * s, p).unwrap();
            for child in [&split.lower, &split.upper] {
                let original = format!("{:?}", child.solid());
                let exported = child.export_step_mm(p).unwrap();
                for face in child.solid().shell.faces.iter().take(4) {
                    let Surface::Nurbs(net) = &face.surface else {
                        panic!("wall must retain rational surface")
                    };
                    let d = net.domain();
                    let uv = [0.31, (d[1][0] + d[1][1]) / 2.];
                    let point = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                    let normal =
                        face.surface.normal_at(uv[0], uv[1]).unwrap() * face.orientation as f64;
                    for (offset, expected) in [
                        (-2., PointLocation::Inside),
                        (-0.5, PointLocation::Boundary),
                        (0., PointLocation::Boundary),
                        (0.5, PointLocation::Boundary),
                        (2., PointLocation::Outside),
                    ] {
                        let shifted = point + normal * (offset * 1e-6 * s);
                        assert_eq!(child.classify_point(shifted, p).unwrap(), expected);
                        if expected == PointLocation::Boundary {
                            witness(child, shifted, normal, p, s);
                        } else {
                            assert!(child.point_query(shifted, p).unwrap().boundary.is_none());
                        }
                    }
                }
                let point = body.frame().point(Point3::new(16. * s, 0., cut * s));
                let normal = body.frame().axes()[2] * (if child.is_lower() { 1. } else { -1. });
                for (offset, expected) in [
                    (-2., PointLocation::Inside),
                    (-0.5, PointLocation::Boundary),
                    (0., PointLocation::Boundary),
                    (0.5, PointLocation::Boundary),
                    (2., PointLocation::Outside),
                ] {
                    let shifted = point + normal * (offset * 1e-6 * s);
                    assert_eq!(child.classify_point(shifted, p).unwrap(), expected);
                    if expected == PointLocation::Boundary {
                        witness(child, shifted, normal, p, s);
                    }
                }
                let relative = GeometryTolerance::new(1e-6 * s, 1e-10, 0.001).unwrap();
                let band = relative.length_at_scale(40_f64.hypot(8.) * s).unwrap();
                witness(child, point + normal * (0.5 * band), normal, relative, s);
                assert_eq!(
                    child
                        .classify_point(point + normal * (2. * band), relative)
                        .unwrap(),
                    PointLocation::Outside
                );
                assert_eq!(original, format!("{:?}", child.solid()));
                assert_eq!(exported, child.export_step_mm(p).unwrap());
            }
        }
        assert_eq!(before, format!("{:?}", body.solid()));
        assert_eq!(step, body.export_step_mm(p).unwrap());
    }
}
#[test]
fn finite_annular_cap_and_restricted_wall_membership_avoids_infinite_plane_shortcuts() {
    let body = source(1.);
    let split = body.split_by_local_z(1.25, policy(1.)).unwrap();
    for child in [&split.lower, &split.upper] {
        let z = if child.is_lower() { -1. } else { 2. };
        for (local, expected) in [
            (Point3::new(16., 0., z), PointLocation::Inside),
            (Point3::new(0., 0., 1.25), PointLocation::Outside),
            (Point3::new(30., 0., 1.25), PointLocation::Outside),
            (
                Point3::new(16., 0., if child.is_lower() { 3. } else { -3. }),
                PointLocation::Outside,
            ),
            (Point3::new(0., 0., 100.), PointLocation::Outside),
        ] {
            let point = body.frame().point(local);
            // Independently sample the retained actual walls, proving the chosen witnesses
            // have generous clearance, rather than using the classifier's distance formula.
            let mut sampled = f64::INFINITY;
            for face in &child.solid().shell.faces {
                if let Surface::Nurbs(net) = &face.surface {
                    let d = net.domain();
                    for i in 0..=16 {
                        for j in 0..=16 {
                            let u = if i == 16 {
                                d[0][1]
                            } else {
                                d[0][0] + (d[0][1] - d[0][0]) * i as f64 / 16.
                            };
                            let v = if j == 16 {
                                d[1][1]
                            } else {
                                d[1][0] + (d[1][1] - d[1][0]) * j as f64 / 16.
                            };
                            sampled = sampled.min((net.evaluate(u, v).unwrap() - point).norm());
                        }
                    }
                }
            }
            assert!(sampled > 0.1);
            assert_eq!(child.classify_point(point, policy(1.)).unwrap(), expected);
            assert!(child
                .point_query(point, policy(1.))
                .unwrap()
                .boundary
                .is_none());
        }
    }
}
#[test]
fn cut_rim_nonunique_witness_and_invalid_original_queries_reject_before_miss_then_recover() {
    let body = source(1.);
    let split = body.split_by_local_z(1.25, policy(1.)).unwrap();
    let radial = (16_f64 - 1.25 * 1.25).sqrt();
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(policy(1.)).unwrap();
        for rho in [16. - radial, 16. + radial] {
            assert!(child
                .point_query(body.frame().point(Point3::new(rho, 0., 1.25)), policy(1.))
                .is_err());
        }
        for bad in [
            Point3::new(f64::NAN, 0., 0.),
            Point3::new(0., f64::INFINITY, 0.),
            Point3::new(1e12, 0., 0.),
        ] {
            assert!(child.point_query(bad, policy(1.)).is_err());
        }
        let z = if child.is_lower() { -1. } else { 2. };
        assert_eq!(
            child
                .classify_point(body.frame().point(Point3::new(16., 0., z)), policy(1.))
                .unwrap(),
            PointLocation::Inside
        );
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(policy(1.)).unwrap());
    }
}
