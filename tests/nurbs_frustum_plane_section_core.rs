use hagane::*;
fn plane(origin: Point3, normal: Vec3) -> Surface {
    let n = normal.normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let v = n.cross(u);
    Surface::Plane { origin, u, v }
}
#[test]
fn actual_closed_rational_section_has_source_uv_and_independent_plane_geometry() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let cut = plane(Point3::new(0., 0., 12.), Vec3::new(0.1, 0., 1.));
    let section = body.section_by_plane(&cut, p).unwrap();
    section.validate(p).unwrap();
    for q in 0..4 {
        let edge = &section.edges()[q];
        let Curve::Nurbs(curve) = &edge.curve else {
            panic!()
        };
        assert_eq!(curve.degree(), 2);
        let PCurve::Nurbs(uv) = &section.uses()[q].pcurve else {
            panic!()
        };
        assert_eq!(uv.degree(), 3);
        assert_eq!(edge.vertices, [q, (q + 1) % 4]);
        for i in 0..101 {
            let t = i as f64 / 100.;
            let point = curve.evaluate(t).unwrap();
            let xy = uv.evaluate(t).unwrap();
            let actual = body.solid().shell.faces[2 + q]
                .surface
                .try_evaluate(xy.x, xy.y)
                .unwrap();
            assert!((actual - point).norm() < 1e-10);
            assert!((0.1 * point.x + point.z - 12.).abs() < 1e-10);
            let radius = 16. - point.z / 3.;
            assert!((point.x.hypot(point.y) - radius).abs() < 1e-10);
        }
    }
    let reversed = match cut {
        Surface::Plane { origin, u, v } => Surface::Plane {
            origin,
            u,
            v: v * -1.,
        },
        _ => unreachable!(),
    };
    let opposite = body.section_by_plane(&reversed, p).unwrap();
    for q in 0..4 {
        let (Curve::Nurbs(a), Curve::Nurbs(b)) =
            (&section.edges()[q].curve, &opposite.edges()[q].curve)
        else {
            panic!()
        };
        assert_eq!(a.control_points(), b.control_points());
        assert_eq!(a.weights(), b.weights());
    }
}
#[test]
fn rigid_placed_section_and_cap_crossing_refusals() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), rotation.axes(), p).unwrap();
    let body = NurbsFrustumSolid::new(frame, [16., 8.], 24., p).unwrap();
    let local = plane(Point3::new(0., 0., 12.), Vec3::new(0.1, 0., 1.));
    let Surface::Plane { origin, u, v } = local else {
        panic!()
    };
    let world = Surface::Plane {
        origin: frame.point(origin),
        u: frame.vector(u),
        v: frame.vector(v),
    };
    let section = body.section_by_plane(&world, p).unwrap();
    section.validate(p).unwrap();
    for z in [0., 24., 30.] {
        let cut = Surface::Plane {
            origin: frame.point(Point3::new(0., 0., z)),
            u: frame.axes()[0],
            v: frame.axes()[1],
        };
        assert!(body.section_by_plane(&cut, p).is_err());
    }
    assert!(body
        .section_by_plane(
            &Surface::Plane {
                origin: Point3::new(1e14, 0., 0.),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.)
            },
            p
        )
        .is_err());
}
