use hagane::*;
fn cube() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(4., 4., 4.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
fn classify(s: &Solid, p: Point3) -> PointLocation {
    classify_point_in_solid(s, p, GeometryTolerance::default()).unwrap()
}
#[test]
fn box_grid_matches_analytic_membership() {
    let s = cube();
    for x in [-1., 0., 1., 2., 4., 5.] {
        for y in [-1., 0., 1., 3., 4., 5.] {
            for z in [-1., 0., 1., 4., 5.] {
                let p = Point3::new(x, y, z);
                let outside = [x, y, z].iter().any(|&v| !(0.0..=4.0).contains(&v));
                let boundary = [x, y, z].iter().any(|&v| v == 0. || v == 4.);
                assert_eq!(
                    classify(&s, p),
                    if outside {
                        PointLocation::Outside
                    } else if boundary {
                        PointLocation::Boundary
                    } else {
                        PointLocation::Inside
                    },
                    "{p:?}"
                );
            }
        }
    }
}
#[test]
fn trim_holes_concavity_and_boundary_are_not_filled() {
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [8., 3.], [4., 3.], [4., 8.], [0., 8.]],
            holes: vec![vec![[1., 1.], [2., 1.], [2., 6.], [1., 6.]]],
        },
        Vec3::new(0., 0., 5.),
        Tolerance::default(),
    )
    .unwrap();
    for (p, l) in [
        (Point3::new(3., 6., 2.), PointLocation::Inside),
        (Point3::new(6., 6., 2.), PointLocation::Outside),
        (Point3::new(1.5, 3., 2.), PointLocation::Outside),
        (Point3::new(1., 3., 2.), PointLocation::Boundary),
        (Point3::new(1.5, 3., 0.), PointLocation::Outside),
        (Point3::new(3., 6., 0.), PointLocation::Boundary),
    ] {
        assert_eq!(classify(&s, p), l);
    }
}
#[test]
fn vertex_ray_and_subdivided_faces_use_independent_rays() {
    let s = cube();
    assert_eq!(classify(&s, Point3::new(3., 2., 1.)), PointLocation::Inside);
    let r = split_planar_face(
        &s,
        0,
        Point3::new(2., 0., 0.),
        Vec3::new(0., 1., 0.),
        GeometryTolerance::default(),
    )
    .unwrap();
    assert_eq!(
        classify(&r.solid, Point3::new(2., 2., 0.)),
        PointLocation::Boundary
    );
    assert_eq!(
        classify(&r.solid, Point3::new(2., 2., 1.)),
        PointLocation::Inside
    );
}
#[test]
fn euclidean_boundary_band_and_relative_budget() {
    let s = cube();
    let t = GeometryTolerance::new(1e-8, 1e-10, 0.).unwrap();
    for (p, l) in [
        (Point3::new(-0.5e-8, 2., 2.), PointLocation::Boundary),
        (Point3::new(-2e-8, 2., 2.), PointLocation::Outside),
        (Point3::new(-0.8e-8, -0.8e-8, 2.), PointLocation::Outside),
        (Point3::new(-0.5e-8, -0.5e-8, 2.), PointLocation::Boundary),
    ] {
        assert_eq!(classify_point_in_solid(&s, p, t).unwrap(), l);
    }
    let t = GeometryTolerance::new(1e-8, 1e-10, 1e-3).unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(-0.001, 2., 2.), t).unwrap(),
        PointLocation::Boundary
    );
}
#[test]
fn rotation_skew_and_small_scale() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let s = cube().transformed(tr, t.absolute()).unwrap();
    for (p, l) in [
        (Point3::new(2., 2., 2.), PointLocation::Inside),
        (Point3::new(2., 2., 0.), PointLocation::Boundary),
        (Point3::new(-1., 2., 2.), PointLocation::Outside),
    ] {
        assert_eq!(classify(&s, tr.point(p)), l);
    }
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(4e-6, 4e-6, 4e-6),
        },
        t.absolute(),
    )
    .unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(2e-6, 2e-6, 2e-6), t).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn unsupported_solids_bad_topology_and_nonfinite_points_are_errors() {
    let t = GeometryTolerance::default();
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 2.,
        },
        t.absolute(),
    )
    .unwrap();
    for p in [Point3::new(0., 0., 1.), Point3::new(100., 0., 1.)] {
        assert!(classify_point_in_solid(&s, p, t).is_err());
    }
    assert!(classify_point_in_solid(&cube(), Point3::new(f64::NAN, 0., 0.), t).is_err());
    let mut s = cube();
    s.shell.faces.pop();
    assert!(classify_point_in_solid(&s, Point3::new(2., 2., 2.), t).is_err());
}
#[test]
fn skew_extrusion_uses_exact_side_planes() {
    let s = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
            holes: vec![vec![[1., 1.], [2., 1.], [2., 2.], [1., 2.]]],
        },
        Vec3::new(2., -3., 5.),
        Tolerance::default(),
    )
    .unwrap();
    for (p, l) in [
        (Point3::new(3.8, 1.8, 2.), PointLocation::Inside),
        (Point3::new(2.3, 0.3, 2.), PointLocation::Outside),
        (Point3::new(0.8, 0.8, 2.), PointLocation::Boundary),
    ] {
        assert_eq!(classify(&s, p), l);
    }
}
#[test]
fn exhausted_near_parallel_ray_candidates_return_error() {
    let t = GeometryTolerance::new(1e-8, 1.5, 0.).unwrap();
    assert!(matches!(
        classify_point_in_solid(&cube(), Point3::new(2., 2., 2.), t),
        Err(Error::Unsupported(_))
    ));
}
