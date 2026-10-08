use hagane::*;
fn classify(s: &Solid, p: Point3) -> PointLocation {
    classify_point_in_solid(s, p, GeometryTolerance::default()).unwrap()
}
fn cylinder() -> Solid {
    make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -2.),
            radius: 3.,
            height: 4.,
        },
        Tolerance::default(),
    )
    .unwrap()
}
#[test]
fn cylinder_and_tube_grid_matches_analytic_material_and_boundary() {
    let tube = make_tube(
        TubeSpec {
            base: Point3::new(0., 0., -2.),
            outer_radius: 3.,
            inner_radius: 1.,
            height: 4.,
        },
        Tolerance::default(),
    )
    .unwrap();
    for (solid, inner) in [(cylinder(), 0.), (tube, 1.)] {
        for x in -7..=7 {
            for y in -7..=7 {
                for z in -5..=5 {
                    let p = Point3::new(x as f64 * 0.5, y as f64 * 0.5, z as f64 * 0.5);
                    let r = p.x.hypot(p.y);
                    let within = r >= inner && r <= 3. && p.z >= -2. && p.z <= 2.;
                    let boundary =
                        within && (r == 3. || (inner > 0. && r == inner) || p.z.abs() == 2.);
                    let expected = if boundary {
                        PointLocation::Boundary
                    } else if within {
                        PointLocation::Inside
                    } else {
                        PointLocation::Outside
                    };
                    assert_eq!(classify(&solid, p), expected, "{p:?}, inner={inner}");
                }
            }
        }
    }
}
#[test]
fn box_bores_keep_voids_empty_including_caps_and_periodic_seams() {
    let box_spec = BoxSpec {
        min: Point3::new(-4., -3., -2.),
        size: Vec3::new(8., 6., 4.),
    };
    let cutter = CylinderSpec {
        base: Point3::new(0., 0., -3.),
        radius: 1.,
        height: 6.,
    };
    let s = subtract_through_cylinder(box_spec, cutter, Tolerance::default()).unwrap();
    for (p, l) in [
        (Point3::new(0., 0., 0.), PointLocation::Outside),
        (Point3::new(0., 0., 2.), PointLocation::Outside),
        (Point3::new(1., 0., 0.), PointLocation::Boundary),
        (Point3::new(-1., 0., 2.), PointLocation::Boundary),
        (Point3::new(2., 0., 0.), PointLocation::Inside),
        (Point3::new(2., 0., 2.), PointLocation::Boundary),
        (Point3::new(4., 0., 0.), PointLocation::Boundary),
    ] {
        assert_eq!(classify(&s, p), l);
    }
    let cutters = [
        CylinderSpec {
            base: Point3::new(-2., 0., -3.),
            radius: 0.5,
            height: 6.,
        },
        CylinderSpec {
            base: Point3::new(2., 0., -3.),
            radius: 0.5,
            height: 6.,
        },
    ];
    let s = subtract_through_cylinders(box_spec, &cutters, Tolerance::default()).unwrap();
    for (p, l) in [
        (Point3::new(-2., 0., 0.), PointLocation::Outside),
        (Point3::new(2., 0., 0.), PointLocation::Outside),
        (Point3::new(0., 0., 0.), PointLocation::Inside),
        (Point3::new(2.5, 0., 0.), PointLocation::Boundary),
    ] {
        assert_eq!(classify(&s, p), l);
    }
}
#[test]
fn euclidean_rim_band_and_relative_policy_are_respected() {
    let s = cylinder();
    let t = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(3. + 6e-6, 0., 2. + 6e-6), t).unwrap(),
        PointLocation::Boundary
    );
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(3. + 8e-6, 0., 2. + 8e-6), t).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(3. - 8e-6, 0., 2. - 8e-6), t).unwrap(),
        PointLocation::Boundary
    );
    let t = GeometryTolerance::new(1e-8, 1e-10, 1e-4).unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(3. + 1e-4, 0., 0.), t).unwrap(),
        PointLocation::Boundary
    );
}
#[test]
fn rigid_placement_and_tiny_cylinders_share_the_kernel() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let s = cylinder().transformed(tr, t.absolute()).unwrap();
    for (p, l) in [
        (Point3::new(0., 0., 0.), PointLocation::Inside),
        (Point3::new(3., 0., 0.), PointLocation::Boundary),
        (Point3::new(4., 0., 0.), PointLocation::Outside),
        (Point3::new(1., 0., 2.), PointLocation::Boundary),
    ] {
        assert_eq!(classify(&s, tr.point(p)), l);
    }
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -2e-6),
            radius: 3e-6,
            height: 4e-6,
        },
        t.absolute(),
    )
    .unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(1e-6, 0., 0.), t).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn bounded_arcs_and_invalid_inputs() {
    let t = GeometryTolerance::default();
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 8., 6., 1., t.absolute()).unwrap();
    let s = extrude_arc_line(&profile, 4., t.absolute()).unwrap();
    assert_eq!(classify(&s, Point3::new(0., 0., 1.)), PointLocation::Inside);
    assert_eq!(
        classify(&s, Point3::new(100., 100., 100.)),
        PointLocation::Outside
    );
    assert!(classify_point_in_solid(&cylinder(), Point3::new(f64::NAN, 0., 0.), t).is_err());
    let mut s = cylinder();
    s.shell.faces[0].orientation *= -1;
    assert!(classify_point_in_solid(&s, Point3::new(0., 0., 0.), t).is_err());
    assert!(classify_point_in_solid(
        &cylinder(),
        Point3::new(0., 0., 0.),
        GeometryTolerance::new(1e-8, 1.5, 0.).unwrap()
    )
    .is_err());
}
