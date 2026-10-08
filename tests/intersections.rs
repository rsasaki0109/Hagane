use hagane::*;
use std::f64::consts::PI;
fn tol() -> GeometryTolerance {
    GeometryTolerance::default()
}
fn xy(z: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(0.0, 0.0, z),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    }
}
fn cylinder() -> Surface {
    Surface::Cylinder {
        center: Point3::new(0.0, 0.0, 0.0),
        radius: 2.0,
        height: 4.0,
    }
}
fn points(
    anchor: Point3,
    d: Vec3,
    s: &Surface,
    t: GeometryTolerance,
) -> Vec<CylinderIntersectionPoint> {
    let LineCylinderIntersection::Points(p) = intersect_line_cylinder(anchor, d, s, t).unwrap()
    else {
        panic!("expected points")
    };
    p
}
#[test]
fn plane_line_has_unit_speed_and_matching_uv_on_both_planes() {
    let a = xy(3.0);
    let b = Surface::Plane {
        origin: Point3::new(5.0, 0.0, 0.0),
        u: Vec3::new(0.0, 1.0, 0.0),
        v: Vec3::new(0.0, 0.0, 1.0),
    };
    let PlanePlaneIntersection::Line(l) = intersect_plane_plane(&a, &b, tol()).unwrap() else {
        panic!()
    };
    assert_eq!(l.origin, Point3::new(5.0, 0.0, 3.0));
    assert_eq!(l.direction, Vec3::new(0.0, 1.0, 0.0));
    for t in [-17.0, 0.0, 5.0] {
        let p = l.evaluate(t).unwrap();
        for (s, c) in [(&a, &l.first), (&b, &l.second)] {
            let uv = c.evaluate(t);
            assert!((p - s.evaluate(uv[0], uv[1])).norm() < 1e-12);
        }
    }
    let PlanePlaneIntersection::Line(reverse) = intersect_plane_plane(&b, &a, tol()).unwrap()
    else {
        panic!()
    };
    assert_eq!(reverse.direction, l.direction * -1.0);
    assert!(l.evaluate(f64::NAN).is_err());
}
#[test]
fn plane_parallel_coincident_reversal_and_policy() {
    assert!(matches!(
        intersect_plane_plane(&xy(0.0), &xy(1.0), tol()).unwrap(),
        PlanePlaneIntersection::Parallel
    ));
    let mut b = xy(5e-9);
    if let Surface::Plane { v, .. } = &mut b {
        *v = *v * -1.0;
    }
    assert!(matches!(
        intersect_plane_plane(&xy(0.0), &b, tol()).unwrap(),
        PlanePlaneIntersection::Coincident
    ));
    let almost = xy(1.0)
        .transformed(Transform::rotation(Vec3::new(1.0, 0.0, 0.0), 1e-12).unwrap())
        .unwrap();
    assert!(matches!(
        intersect_plane_plane(&xy(0.0), &almost, tol()).unwrap(),
        PlanePlaneIntersection::Parallel
    ));
    assert!(matches!(
        intersect_plane_plane(
            &xy(0.0),
            &almost,
            GeometryTolerance::new(1e-8, 1e-14, 0.0).unwrap()
        )
        .unwrap(),
        PlanePlaneIntersection::Line(_)
    ));
}
#[test]
fn placed_planes_and_small_planes_preserve_intersection_geometry() {
    let a = xy(0.0);
    let b = xy(0.0)
        .transformed(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.9).unwrap())
        .unwrap();
    for scale in [1e-12, 1.0, 1e6] {
        let t = GeometryTolerance::new(scale * 1e-10, 1e-10, 0.0).unwrap();
        let transform = Transform::translation(Vec3::new(scale * 3.0, scale * 4.0, scale * 5.0))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(2.0, 1.0, 3.0), 0.7).unwrap())
            .unwrap();
        let PlanePlaneIntersection::Line(l) = intersect_plane_plane(
            &a.transformed(transform).unwrap(),
            &b.transformed(transform).unwrap(),
            t,
        )
        .unwrap() else {
            panic!()
        };
        assert!((l.origin - transform.point(Point3::new(0.0, 0.0, 0.0))).norm() < t.linear());
        for s in [&a, &b] {
            let p = transform
                .inverse()
                .unwrap()
                .point(l.evaluate(scale).unwrap());
            let uv = s.parameters(p);
            assert!((s.evaluate(uv[0], uv[1]) - p).norm() < t.linear());
        }
    }
}
#[test]
fn cylinder_secants_tangents_and_original_line_parameters() {
    let a = Point3::new(-5.0, 0.0, 2.0);
    let d = Vec3::new(2.0, 0.0, 0.0);
    let p = points(a, d, &cylinder(), tol());
    assert_eq!(p.len(), 2);
    for (p, t, x) in [(p[0], 1.5, -2.0), (p[1], 3.5, 2.0)] {
        assert_eq!(p.parameter, t);
        assert_eq!(p.point, Point3::new(x, 0.0, 2.0));
        assert_eq!(p.contact, IntersectionContact::Crossing);
        assert!((cylinder().evaluate(p.uv[0], p.uv[1]) - p.point).norm() < 1e-12);
    }
    let p = points(Point3::new(-5.0, 2.0, 2.0), d, &cylinder(), tol());
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].contact, IntersectionContact::Tangent);
    assert_eq!(p[0].parameter, 2.5);
    assert!(matches!(
        intersect_line_cylinder(Point3::new(-5.0, 3.0, 2.0), d, &cylinder(), tol()).unwrap(),
        LineCylinderIntersection::Empty
    ));
}
#[test]
fn cylinder_axial_clipping_includes_end_levels_but_no_end_disks() {
    let p = points(
        Point3::new(-4.0, 0.0, -2.0),
        Vec3::new(1.0, 0.0, 1.0),
        &cylinder(),
        tol(),
    );
    assert_eq!(p.len(), 2);
    assert!(p[0].uv[1].abs() < 1e-12);
    assert!((p[1].uv[1] - 4.0).abs() < 1e-12);
    let p = points(
        Point3::new(-4.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
        &cylinder(),
        tol(),
    );
    assert_eq!(p.len(), 1);
    assert!(matches!(
        intersect_line_cylinder(
            Point3::new(0.0, 0.0, -2.0),
            Vec3::new(0.0, 0.0, 1.0),
            &cylinder(),
            tol()
        )
        .unwrap(),
        LineCylinderIntersection::Empty
    ));
    assert!(matches!(
        intersect_line_cylinder(
            Point3::new(-5.0, 0.0, 5.0),
            Vec3::new(1.0, 0.0, 0.0),
            &cylinder(),
            tol()
        )
        .unwrap(),
        LineCylinderIntersection::Empty
    ));
}
#[test]
fn generator_intervals_are_sorted_with_reversed_and_scaled_directions() {
    for dz in [2.0, -2.0, 1e300, 1e-300] {
        let a = Point3::new(2.0, 0.0, 1.0);
        let d = Vec3::new(0.0, 0.0, dz);
        let LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } = intersect_line_cylinder(a, d, &cylinder(), tol()).unwrap()
        else {
            panic!()
        };
        assert_eq!(angle, 0.0);
        assert!(parameter_range[0] < parameter_range[1]);
        let z = parameter_range.map(|t| (a + d * t).z);
        assert!((z[0].min(z[1])).abs() < 1e-12);
        assert!((z[0].max(z[1]) - 4.0).abs() < 1e-12);
    }
}
#[test]
fn rigidly_placed_cylinder_hits_agree_with_independent_analytic_points() {
    let tr = Transform::translation(Vec3::new(12.0, -7.0, 3.0))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), PI / 3.0).unwrap())
        .unwrap();
    let c = cylinder().transformed(tr).unwrap();
    for y in [-1.5, -0.3, 0.0, 1.0] {
        let a = tr.point(Point3::new(-5.0, y, 2.0));
        let d = tr.vector(Vec3::new(3.0, 0.0, 0.0));
        let p = points(a, d, &c, tol());
        assert_eq!(p.len(), 2);
        for hit in &p {
            assert!(hit.uv[0] >= 0.0 && hit.uv[0] < 2.0 * PI);
        }
        let x = (4.0 - y * y).sqrt();
        for (hit, x) in [(p[0], -x), (p[1], x)] {
            assert!((hit.point - tr.point(Point3::new(x, y, 2.0))).norm() < 1e-12);
            assert!((hit.parameter - (5.0 + x) / 3.0).abs() < 1e-12);
        }
    }
}
#[test]
fn near_contacts_invalid_inputs_and_unsupported_surfaces_are_explicit() {
    for y in [2.0 - 1e-9, 2.0 + 1e-9] {
        assert!(intersect_line_cylinder(
            Point3::new(-5.0, y, 2.0),
            Vec3::new(1.0, 0.0, 0.0),
            &cylinder(),
            tol()
        )
        .is_err());
    }
    assert!(intersect_line_cylinder(
        Point3::new(2.0 + 1e-9, 0.0, 2.0),
        Vec3::new(0.0, 0.0, 1.0),
        &cylinder(),
        tol()
    )
    .is_err());
    assert!(intersect_line_cylinder(
        Point3::new(2.0, 0.0, 2.0),
        Vec3::new(1e-12, 0.0, 1.0),
        &cylinder(),
        tol()
    )
    .is_err());
    for d in [Vec3::new(0.0, 0.0, 0.0), Vec3::new(f64::NAN, 0.0, 0.0)] {
        assert!(
            intersect_line_cylinder(Point3::new(0.0, 0.0, 0.0), d, &cylinder(), tol()).is_err()
        );
    }
    assert!(intersect_plane_plane(&cylinder(), &xy(0.0), tol()).is_err());
    assert!(intersect_line_cylinder(
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        &xy(0.0),
        tol()
    )
    .is_err());
    let bad = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 0.0),
        u: Vec3::new(2.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    assert!(intersect_plane_plane(&bad, &xy(0.0), tol()).is_err());
}
#[test]
fn microscopic_cylinder_and_large_direction_scales() {
    let c = Surface::Cylinder {
        center: Point3::new(0.0, 0.0, 0.0),
        radius: 2e-12,
        height: 4e-12,
    };
    let t = GeometryTolerance::new(1e-22, 1e-10, 0.0).unwrap();
    for speed in [1e-300, 1e300] {
        let p = points(
            Point3::new(-5e-12, 0.0, 2e-12),
            Vec3::new(speed, 0.0, 0.0),
            &c,
            t,
        );
        assert_eq!(p.len(), 2);
        assert!((p[0].point.x + 2e-12).abs() < 1e-26);
        assert!((p[1].point.x - 2e-12).abs() < 1e-26);
    }
}

#[test]
fn nonfinite_unrepresentable_and_invalid_surface_data_are_rejected() {
    let a = Point3::new(0.0, 0.0, 2.0);
    let d = Vec3::new(1.0, 0.0, 0.0);
    for radius in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1e-9] {
        let c = Surface::Cylinder {
            center: Point3::new(0.0, 0.0, 0.0),
            radius,
            height: 4.0,
        };
        assert!(intersect_line_cylinder(a, d, &c, tol()).is_err());
    }
    for height in [f64::NAN, -1.0, 0.0, 1e-9] {
        let c = Surface::Cylinder {
            center: Point3::new(0.0, 0.0, 0.0),
            radius: 2.0,
            height,
        };
        assert!(intersect_line_cylinder(a, d, &c, tol()).is_err());
    }
    assert!(
        intersect_line_cylinder(Point3::new(f64::NAN, 0.0, 2.0), d, &cylinder(), tol()).is_err()
    );
    assert!(intersect_line_cylinder(Point3::new(1e30, 0.0, 2.0), d, &cylinder(), tol()).is_err());
    let a = Surface::Plane {
        origin: Point3::new(f64::MAX, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    let b = Surface::Plane {
        origin: Point3::new(-f64::MAX, 0.0, 0.0),
        u: Vec3::new(0.0, 1.0, 0.0),
        v: Vec3::new(0.0, 0.0, 1.0),
    };
    assert!(intersect_plane_plane(&a, &b, tol()).is_err());
    assert!(intersect_line_cylinder(Point3::new(-5.0, 0.0, 1e-16), d, &cylinder(), tol()).is_err());
}

#[test]
fn world_offsets_cannot_collapse_cylinder_generator_endpoints() {
    let c = Surface::FramedCylinder {
        frame: Frame3::translation(Vec3::new(0.0, 0.0, 1e30)).unwrap(),
        radius: 2.0,
        height: 4.0,
    };
    assert!(intersect_line_cylinder(
        Point3::new(2.0, 0.0, 1e30),
        Vec3::new(0.0, 0.0, 1.0),
        &c,
        tol()
    )
    .is_err());
}
