use hagane::*;
fn surface(frame: Frame3) -> Surface {
    Surface::ExtrudedCircle {
        frame,
        radius: 2.,
        height: 4.,
        drift: [1., 0.5],
    }
}
fn points(hit: LineCylinderIntersection) -> Vec<CylinderIntersectionPoint> {
    let LineCylinderIntersection::Points(p) = hit else {
        panic!("expected finite points: {hit:?}")
    };
    p
}
#[test]
fn independent_radial_roots_axial_clipping_parameters_and_rigid_placement() {
    let tol = GeometryTolerance::default();
    let placed = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    for f in [Frame3::IDENTITY, placed] {
        for y in [-3., -1.5, -0.5, 0., 0.5, 1.5, 3.] {
            for (z, dz) in [(2., 0.), (-1., 0.5), (1., 0.5), (1., 4.)] {
                let a = f.point(Point3::new(-5. + z, y + 0.5 * z, z));
                let d = f.vector(Vec3::new(2. + dz, 0.5 * dz, dz));
                let result = intersect_line_extruded_circle(a, d, &surface(f), tol).unwrap();
                let mut expected = Vec::new();
                if y.abs() < 2. {
                    let h = (4. - y * y).sqrt();
                    for t in [(5. - h) / 2., (5. + h) / 2.] {
                        let v = z + dz * t;
                        if (0. ..=4.).contains(&v) {
                            expected.push((t, v));
                        }
                    }
                }
                if expected.is_empty() {
                    assert_eq!(result, LineCylinderIntersection::Empty);
                    continue;
                }
                let hits = points(result);
                assert_eq!(hits.len(), expected.len());
                for (hit, (t, v)) in hits.iter().zip(expected) {
                    assert!((hit.parameter - t).abs() < 1e-10);
                    assert!((hit.uv[1] - v).abs() < 1e-10);
                    assert_eq!(hit.contact, IntersectionContact::Crossing);
                    assert!((hit.point - (a + d * t)).norm() < 1e-10);
                    assert!((surface(f).evaluate(hit.uv[0], hit.uv[1]) - hit.point).norm() < 1e-10);
                }
            }
        }
    }
}
#[test]
fn tangency_generators_reversed_direction_and_dimensionless_line_scaling() {
    let tol = GeometryTolerance::default();
    let s = surface(Frame3::IDENTITY);
    let tangent = points(
        intersect_line_extruded_circle(Point3::new(-3., 3., 2.), Vec3::new(2., 0., 0.), &s, tol)
            .unwrap(),
    );
    assert_eq!(tangent.len(), 1);
    assert_eq!(tangent[0].contact, IntersectionContact::Tangent);
    assert_eq!(tangent[0].parameter, 2.5);
    let a = Point3::new(2., 0., 0.);
    let g = Vec3::new(4., 2., 4.);
    for (a, d) in [(a, g), (a + g, g * (-1.))] {
        let LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } = intersect_line_extruded_circle(a, d, &s, tol).unwrap()
        else {
            panic!("expected generator overlap")
        };
        assert!((parameter_range[0]).abs() < 1e-12 && (parameter_range[1] - 1.).abs() < 1e-12);
        assert!(angle.abs() < 1e-12);
    }
    assert_eq!(
        intersect_line_extruded_circle(Point3::new(3., 0., 0.), g, &s, tol).unwrap(),
        LineCylinderIntersection::Empty
    );
    for scale in [1e-200, 1., 1e200, 1e300] {
        let hits = points(
            intersect_line_extruded_circle(
                Point3::new(-3., 1., 2.),
                Vec3::new(scale, 0., 0.),
                &s,
                tol,
            )
            .unwrap(),
        );
        assert_eq!(hits.len(), 2);
        for (hit, travel) in hits.iter().zip([3., 7.]) {
            assert!((hit.parameter * scale - travel).abs() < 1e-12);
        }
    }
}
#[test]
fn tiny_geometry_zero_drift_and_conservative_contact_guards() {
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let s = Surface::ExtrudedCircle {
        frame: Frame3::IDENTITY,
        radius: 2e-6,
        height: 4e-6,
        drift: [1., 0.5],
    };
    let hits = points(
        intersect_line_extruded_circle(
            Point3::new(-3e-6, 1e-6, 2e-6),
            Vec3::new(1., 0., 0.),
            &s,
            tol,
        )
        .unwrap(),
    );
    assert_eq!(hits.len(), 2);
    let tol = GeometryTolerance::default();
    let s = surface(Frame3::IDENTITY);
    for y in [3. - 1e-9, 3. + 1e-9] {
        assert!(intersect_line_extruded_circle(
            Point3::new(-3., y, 2.),
            Vec3::new(1., 0., 0.),
            &s,
            tol
        )
        .is_err());
    }
    assert!(intersect_line_extruded_circle(
        Point3::new(2. + 1e-9, 0., 0.),
        Vec3::new(4., 2., 4.),
        &s,
        tol
    )
    .is_err());
    assert!(intersect_line_extruded_circle(
        Point3::new(2., 0., 0.),
        Vec3::new(4. + 1e-11, 2., 4.),
        &s,
        tol
    )
    .is_err());
    let relative = GeometryTolerance::new(1e-10, 1e-10, 1e-3).unwrap();
    assert!(intersect_line_extruded_circle(
        Point3::new(-3., 3.001, 2.),
        Vec3::new(1., 0., 0.),
        &s,
        relative
    )
    .is_err());
    let zero = Surface::ExtrudedCircle {
        frame: Frame3::IDENTITY,
        radius: 2.,
        height: 4.,
        drift: [0., 0.],
    };
    let normal = Surface::Cylinder {
        center: Point3::new(0., 0., 0.),
        radius: 2.,
        height: 4.,
    };
    let a = Point3::new(-5., 1., 2.);
    let d = Vec3::new(2., 0., 0.);
    assert_eq!(
        intersect_line_extruded_circle(a, d, &zero, tol).unwrap(),
        intersect_line_cylinder(a, d, &normal, tol).unwrap()
    );
}
#[test]
fn invalid_and_unresolved_inputs_never_report_success() {
    let tol = GeometryTolerance::default();
    let s = surface(Frame3::IDENTITY);
    for d in [
        Vec3::new(0., 0., 0.),
        Vec3::new(f64::NAN, 0., 0.),
        Vec3::new(f64::INFINITY, 0., 0.),
    ] {
        assert!(intersect_line_extruded_circle(Point3::new(0., 0., 0.), d, &s, tol).is_err());
    }
    for a in [
        Point3::new(f64::NAN, 0., 0.),
        Point3::new(f64::MAX, 0., -f64::MAX),
    ] {
        assert!(intersect_line_extruded_circle(a, Vec3::new(1., 0., 0.), &s, tol).is_err());
    }
    for (r, h, drift) in [
        (-1., 4., [1., 0.]),
        (2., 0., [1., 0.]),
        (2., 4., [f64::NAN, 0.]),
        (2., 4., [f64::MAX, f64::MAX]),
    ] {
        let bad = Surface::ExtrudedCircle {
            frame: Frame3::IDENTITY,
            radius: r,
            height: h,
            drift,
        };
        assert!(intersect_line_extruded_circle(
            Point3::new(0., 0., 0.),
            Vec3::new(1., 0., 0.),
            &bad,
            tol
        )
        .is_err());
    }
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    assert!(matches!(
        intersect_line_extruded_circle(Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), &plane, tol),
        Err(Error::Unsupported(_))
    ));
    let far = surface(Frame3::translation(Point3::new(1e15, -1e15, 1e15)).unwrap());
    assert!(intersect_line_extruded_circle(
        Point3::new(1e15 - 3., -1e15 + 2., 1e15 + 2.),
        Vec3::new(1., 0., 0.),
        &far,
        tol
    )
    .is_err());
}
#[test]
fn exact_axial_rims_and_near_end_ambiguity_are_distinguished() {
    let tol = GeometryTolerance::default();
    let s = surface(Frame3::IDENTITY);
    for z in [0., 4.] {
        let hits = points(
            intersect_line_extruded_circle(
                Point3::new(-5. + z, 0.5 * z, z),
                Vec3::new(2., 0., 0.),
                &s,
                tol,
            )
            .unwrap(),
        );
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|p| p.uv[1] == z));
    }
    for z in [1e-14, 4. - 1e-14] {
        assert!(intersect_line_extruded_circle(
            Point3::new(-5. + z, 0.5 * z, z),
            Vec3::new(2., 0., 0.),
            &s,
            tol
        )
        .is_err());
    }
    assert_eq!(
        intersect_line_extruded_circle(Point3::new(-5., 0., -1.), Vec3::new(2., 0., 0.), &s, tol)
            .unwrap(),
        LineCylinderIntersection::Empty
    );
    assert!(intersect_line_extruded_circle(
        Point3::new(-3., 1., 2.),
        Vec3::new(1e-320, 0., 0.),
        &s,
        tol
    )
    .is_err());
}
