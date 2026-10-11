use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rotation =
        Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let f = Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), policy(s))
        .unwrap();
    NurbsTorusSolid::new(f, [20. * s, 5. * s], policy(s)).unwrap()
}
#[test]
fn actual_tensor_boundary_points_and_true_surface_normals_obey_finite_metric_band() {
    for scale in [1e-4, 1., 10.] {
        let shape = body(scale, true);
        let p = policy(scale);
        let before = format!("{:?}", shape.solid());
        let step = shape.export_step_mm(p).unwrap();
        for face_id in [0, 3, 5, 10, 15] {
            let face = &shape.solid().shell.faces[face_id];
            for uv in [[0., 0.], [0.23, 0.67], [0.5, 0.5], [1., 1.]] {
                let point = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let normal =
                    face.surface.normal_at(uv[0], uv[1]).unwrap() * face.orientation as f64;
                assert_eq!(
                    shape.classify_point(point, p).unwrap(),
                    PointLocation::Boundary
                );
                for (offset, expected) in [
                    (-1.5, PointLocation::Inside),
                    (-0.5, PointLocation::Boundary),
                    (0.5, PointLocation::Boundary),
                    (1.5, PointLocation::Outside),
                ] {
                    let shifted = point + normal * (offset * 1e-6 * scale);
                    let query = shape.point_query(shifted, p).unwrap();
                    assert_eq!(query.location, expected);
                    if expected == PointLocation::Boundary {
                        let witness = query.boundary.unwrap();
                        let actual = shape.solid().shell.faces[witness.face]
                            .surface
                            .try_evaluate(witness.uv[0], witness.uv[1])
                            .unwrap();
                        assert!((actual - witness.point).norm() < 1e-8 * scale);
                        assert!(
                            (witness.distance - (shifted - witness.point).norm()).abs()
                                < 1e-8 * scale
                        );
                        assert!(witness.distance <= 1e-6 * scale);
                        assert!(witness.normal.dot(normal) > 1. - 1e-8);
                    } else {
                        assert!(query.boundary.is_none());
                    }
                    assert_eq!(
                        shape.classify_point(shifted, p).unwrap(),
                        expected,
                        "face{face_id} uv{uv:?} offset{offset}"
                    );
                }
            }
        }
        assert_eq!(before, format!("{:?}", shape.solid()));
        assert_eq!(step, shape.export_step_mm(p).unwrap());
    }
}
#[test]
fn tube_core_central_void_and_relative_band_use_actual_source_extent() {
    for scale in [1e-4, 1., 10.] {
        let shape = body(scale, false);
        let f = shape.frame();
        let world = |p: Point3| f.point(p * scale);
        for (point, expected) in [
            (Point3::new(0., 0., 0.), PointLocation::Outside),
            (Point3::new(0., 0., 100.), PointLocation::Outside),
            (Point3::new(20., 0., 0.), PointLocation::Inside),
            (Point3::new(0., 20., 0.), PointLocation::Inside),
            (Point3::new(25., 0., 0.), PointLocation::Boundary),
            (Point3::new(15., 0., 0.), PointLocation::Boundary),
            (Point3::new(20., 0., 5.), PointLocation::Boundary),
            (Point3::new(20., 0., -5.), PointLocation::Boundary),
        ] {
            assert_eq!(
                shape.classify_point(world(point), policy(scale)).unwrap(),
                expected
            );
        }
        let relative = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.001).unwrap();
        let band = relative.length_at_scale(50_f64.hypot(10.) * scale).unwrap();
        let face = &shape.solid().shell.faces[5];
        let point = face.surface.try_evaluate(0.23, 0.67).unwrap();
        let normal = face.surface.normal_at(0.23, 0.67).unwrap();
        assert_eq!(
            shape
                .classify_point(point + normal * (0.5 * band), relative)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            shape
                .classify_point(point + normal * (1.5 * band), relative)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            shape
                .classify_point(point - normal * (1.5 * band), relative)
                .unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn precision_threshold_nonfinite_and_remote_queries_refuse_without_mutation() {
    let shape = body(1., false);
    let p = policy(1.);
    let before = format!("{:?}", shape.solid());
    let step = shape.export_step_mm(p).unwrap();
    let f = shape.frame();
    for point in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(f64::INFINITY, 0., 0.),
        Point3::new(1e12, 1e12, 1e12),
    ] {
        assert!(shape.classify_point(point, p).is_err());
    }
    let boundary = f.point(Point3::new(25. + 1e-6, 0., 0.));
    assert!(matches!(
        shape.classify_point(boundary, p),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        shape
            .classify_point(f.point(Point3::new(20., 0., 0.)), p)
            .unwrap(),
        PointLocation::Inside
    );
    let mut corrupted = shape.solid().clone();
    corrupted.shell.faces[0].orientation = -1;
    assert!(NurbsTorusSolid::from_brep(corrupted, f, shape.radii(), p).is_err());
    assert_eq!(before, format!("{:?}", shape.solid()));
    assert_eq!(step, shape.export_step_mm(p).unwrap());
}
