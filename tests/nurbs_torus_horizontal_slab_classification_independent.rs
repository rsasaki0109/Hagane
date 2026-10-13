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
    child: &NurbsTorusHorizontalSlab,
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
fn actual_slab_wall_and_two_cap_normals_have_retained_uv_witnesses_in_the_full_source_band() {
    for s in [1e-4, 1., 10.] {
        let source = source(s);
        let p = policy(s);
        for heights in [[-1.25, 1.25], [0.1, 0.3], [-0.3, -0.1], [-1.25, 0.]] {
            let h = heights.map(|v| v * s);
            let slab = source.slab_between_local_z(h, p).unwrap();
            let before = format!("{:?}", slab.solid());
            let step = slab.export_step_mm(p).unwrap();
            for face in &slab.solid().shell.faces {
                if let Surface::Nurbs(net) = &face.surface {
                    let d = net.domain();
                    let uv = [0.31, (d[1][0] + d[1][1]) / 2.];
                    let point = net.evaluate(uv[0], uv[1]).unwrap();
                    let normal =
                        face.surface.normal_at(uv[0], uv[1]).unwrap() * face.orientation as f64;
                    for (offset, expected) in [
                        (-2., PointLocation::Inside),
                        (-0.5, PointLocation::Boundary),
                        (0., PointLocation::Boundary),
                        (0.5, PointLocation::Boundary),
                        (2., PointLocation::Outside),
                    ] {
                        let x = point + normal * (offset * 1e-6 * s);
                        assert_eq!(slab.classify_point(x, p).unwrap(), expected);
                        if expected == PointLocation::Boundary {
                            witness(&slab, x, normal, p, s);
                        } else {
                            assert!(slab.point_query(x, p).unwrap().boundary.is_none());
                        }
                    }
                    let seam = net.evaluate(0., uv[1]).unwrap();
                    let seam_normal =
                        face.surface.normal_at(0., uv[1]).unwrap() * face.orientation as f64;
                    witness(&slab, seam, seam_normal, p, s);
                }
            }
            for (z, sign) in [(h[0], -1.), (h[1], 1.)] {
                let point = source.frame().point(Point3::new(16. * s, 0., z));
                let normal = source.frame().axes()[2] * sign;
                for (offset, expected) in [
                    (-2., PointLocation::Inside),
                    (-0.5, PointLocation::Boundary),
                    (0., PointLocation::Boundary),
                    (0.5, PointLocation::Boundary),
                    (2., PointLocation::Outside),
                ] {
                    let x = point + normal * (offset * 1e-6 * s);
                    assert_eq!(slab.classify_point(x, p).unwrap(), expected);
                    if expected == PointLocation::Boundary {
                        witness(&slab, x, normal, p, s);
                    }
                }
            }
            assert_eq!(before, format!("{:?}", slab.solid()));
            assert_eq!(step, slab.export_step_mm(p).unwrap());
        }
    }
}
#[test]
fn finite_annulus_and_euclidean_rim_distance_do_not_fabricate_halfspace_boundaries() {
    let source = source(1.);
    let p = policy(1.);
    let frame = source.frame();
    for heights in [[0.1, 0.3], [-1.25, 1.25], [-1.25, 0.]] {
        let slab = source.slab_between_local_z(heights, p).unwrap();
        for point in [
            Point3::new(0., 0., heights[0]),
            Point3::new(0., 0., heights[1]),
            Point3::new(30., 0., heights[0]),
            Point3::new(30., 0., heights[1]),
            Point3::new(16., 0., -3.),
            Point3::new(16., 0., 3.),
            Point3::new(0., 0., 100.),
        ] {
            let q = slab.point_query(frame.point(point), p).unwrap();
            assert_eq!(q.location, PointLocation::Outside);
            assert!(q.boundary.is_none());
        }
        for face in [0usize, 3] {
            let actual_removed_wall = source.solid().shell.faces[face]
                .surface
                .try_evaluate(0.31, 0.5)
                .unwrap();
            assert_eq!(
                slab.classify_point(actual_removed_wall, p).unwrap(),
                PointLocation::Outside
            );
        }
        let middle = (heights[0] + heights[1]) / 2.;
        // Independent strict material predicate: within finite z band, inside tube disk
        // in the (rho,z) meridian, with large margin from every boundary.
        for rho in [0., 8., 14., 16., 18., 24.] {
            let inside =
                ((rho - 16_f64).hypot(middle) < 4.) && (middle > heights[0] && middle < heights[1]);
            assert_eq!(
                slab.classify_point(frame.point(Point3::new(rho, 0., middle)), p)
                    .unwrap(),
                if inside {
                    PointLocation::Inside
                } else {
                    PointLocation::Outside
                }
            );
        }
    }
    let slab = source.slab_between_local_z([-1.25, 0.], p).unwrap();
    for offset in [0.6, 0.8] {
        let point = frame.point(Point3::new(20. + offset * 1e-6, 0., offset * 1e-6));
        match slab.point_query(point, p) {
            Ok(q) => {
                assert_eq!(
                    q.location,
                    if offset < 0.7 {
                        PointLocation::Boundary
                    } else {
                        PointLocation::Outside
                    }
                );
                if let Some(w) = q.boundary {
                    assert!(w.distance <= 1e-6);
                }
            }
            Err(e) => assert!(matches!(e, Error::Unsupported(_))),
        }
    }
}
#[test]
fn physical_rims_relative_clearance_and_invalid_original_queries_reject_then_recover() {
    let source = source(1.);
    let p = policy(1.);
    let frame = source.frame();
    let slab = source.slab_between_local_z([0.1, 0.3], p).unwrap();
    let before = format!("{:?}", slab.solid());
    let step = slab.export_step_mm(p).unwrap();
    for z in [0.1, 0.3] {
        let radial = (16_f64 - z * z).sqrt();
        for rho in [16. - radial, 16. + radial] {
            assert!(slab
                .point_query(frame.point(Point3::new(rho, 0., z)), p)
                .is_err());
        }
    }
    for point in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(0., f64::INFINITY, 0.),
        Point3::new(1e12, 0., 0.),
    ] {
        assert!(slab.point_query(point, p).is_err());
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(
        slab.point_query(frame.point(Point3::new(0., 0., 100.)), relative)
            .is_err(),
        "original slab thickness must be resolved before any miss shortcut"
    );
    let broad_slab = source.slab_between_local_z([-1.25, 1.25], p).unwrap();
    let band = relative.length_at_scale(40_f64.hypot(8.)).unwrap();
    let point = frame.point(Point3::new(16., 0., 1.25));
    let normal = frame.axes()[2];
    witness(
        &broad_slab,
        point + normal * (0.5 * band),
        normal,
        relative,
        1.,
    );
    assert_eq!(
        slab.classify_point(frame.point(Point3::new(16., 0., 0.2)), p)
            .unwrap(),
        PointLocation::Inside
    );
    assert_eq!(before, format!("{:?}", slab.solid()));
    assert_eq!(step, slab.export_step_mm(p).unwrap());
}
