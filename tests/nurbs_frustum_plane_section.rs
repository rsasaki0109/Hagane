use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn body(radii: [f64; 2], posed: bool) -> NurbsFrustumSolid {
    let frame = if posed {
        Transform::translation(Vec3::new(12., -3., 5.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap()
    } else {
        Frame3::IDENTITY
    };
    NurbsFrustumSolid::new(frame, radii, 24., policy()).unwrap()
}
fn plane(body: &NurbsFrustumSolid, height: f64, a: f64, b: f64, reverse: bool) -> Surface {
    let u = Vec3::new(1., 0., a).normalized().unwrap();
    let n = Vec3::new(-a, -b, 1.).normalized().unwrap();
    let v = n.cross(u).normalized().unwrap() * if reverse { -1. } else { 1. };
    Surface::Plane {
        origin: body.frame().point(Point3::new(0., 0., height)),
        u: body.frame().vector(u),
        v: body.frame().vector(v),
    }
}

#[test]
fn actual_rational_section_and_cubic_surface_provenance() {
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        for posed in [false, true] {
            for (a, b) in [(0., 0.), (0.1, 0.05), (1e-14, 0.)] {
                let body = body(radii, posed);
                let surface = plane(&body, 12., a, b, false);
                let section = body.section_by_plane(&surface, policy()).unwrap();
                section.validate(policy()).unwrap();
                assert_eq!(section.edges().len(), 4);
                assert_eq!(section.uses().len(), 4);
                let slope = (radii[1] - radii[0]) / 24.;
                for (index, (edge, use_)) in section.edges().iter().zip(section.uses()).enumerate()
                {
                    let Curve::Nurbs(curve) = &edge.curve else {
                        panic!("exact rational conic");
                    };
                    assert_eq!(curve.degree(), 2);
                    assert!(curve.weights().iter().all(|w| *w > 0.));
                    let PCurve::Nurbs(pcurve) = &use_.pcurve else {
                        panic!("exact rational source provenance");
                    };
                    assert_eq!(pcurve.degree(), 3);
                    assert_eq!(use_.face_id, 2 + index);
                    for i in 0..=64 {
                        let t = i as f64 / 64.;
                        let point = edge.curve.try_evaluate(t).unwrap();
                        let uv = use_.pcurve.try_evaluate(t).unwrap();
                        assert!(
                            (body.solid().shell.faces[use_.face_id]
                                .surface
                                .try_evaluate(uv[0], uv[1])
                                .unwrap()
                                - point)
                                .norm()
                                < 1e-8
                        );
                        let p = body.frame().local_point(point);
                        let theta = p.y.atan2(p.x);
                        let q = a * theta.cos() + b * theta.sin();
                        let z = (12. + radii[0] * q) / (1. - slope * q);
                        assert!((p.z - z).abs() < 1e-8);
                        assert!((p.x.hypot(p.y) - (radii[0] + slope * z)).abs() < 1e-8);
                        let Surface::Plane { origin, u, v } = surface else {
                            unreachable!();
                        };
                        assert!((point - origin).dot(u.cross(v)).abs() < 1e-8);
                    }
                    assert!(
                        (edge.curve.try_evaluate(0.).unwrap()
                            - section.vertices()[edge.vertices[0]].point)
                            .norm()
                            < 1e-10
                    );
                    assert!(
                        (edge.curve.try_evaluate(1.).unwrap()
                            - section.vertices()[edge.vertices[1]].point)
                            .norm()
                            < 1e-10
                    );
                }
            }
        }
    }
}

#[test]
fn equal_cylinder_ellipse_and_plane_reversal_preserve_actual_geometry() {
    let body = body([12., 12.], true);
    let a = 0.2;
    let b = -0.1;
    let forward = body
        .section_by_plane(&plane(&body, 12., a, b, false), policy())
        .unwrap();
    let reverse = body
        .section_by_plane(&plane(&body, 12., a, b, true), policy())
        .unwrap();
    for (f, r) in forward.edges().iter().zip(reverse.edges()) {
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let fp = f.curve.try_evaluate(t).unwrap();
            let rp = r.curve.try_evaluate(t).unwrap();
            assert!((fp - rp).norm() < 1e-9);
            let p = body.frame().local_point(fp);
            assert!((p.x * p.x + p.y * p.y - 144.).abs() < 1e-8);
            assert!((p.z - 12. - a * p.x - b * p.y).abs() < 1e-8);
            let distance = (p - Point3::new(0., 0., 12.)).norm();
            assert!(distance >= 12. - 1e-9 && distance <= 12. * (1. + a * a + b * b).sqrt() + 1e-9);
        }
    }
}

#[test]
fn cap_crossing_tangent_and_unresolved_inputs_do_not_return_sections() {
    let body = body([16., 8.], false);
    let mut inputs = vec![
        plane(&body, 0., 0., 0., false),
        plane(&body, 24., 0., 0., false),
        plane(&body, 12., 5., 0., false),
        plane(&body, 30., 0., 0., false),
    ];
    inputs.push(Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    });
    inputs.push(Surface::Plane {
        origin: Point3::new(f64::NAN, 0., 12.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    });
    inputs.push(Surface::Plane {
        origin: Point3::new(1e12, 0., 12.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    });
    for input in inputs {
        assert!(body.section_by_plane(&input, policy()).is_err());
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(body
        .section_by_plane(&plane(&body, 0.1, 0., 0., false), relative)
        .is_err());
    body.section_by_plane(&plane(&body, 12., 0.1, 0.05, false), policy())
        .unwrap();
}

#[test]
fn normalized_sections_preserve_geometry_at_small_and_large_scales() {
    for scale in [1e-50, 1e50] {
        let p = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
        let body =
            NurbsFrustumSolid::new(Frame3::IDENTITY, [16. * scale, 8. * scale], 24. * scale, p)
                .unwrap();
        let surface = plane(&body, 12. * scale, 0.1, 0.05, false);
        let section = body.section_by_plane(&surface, p).unwrap();
        section.validate(p).unwrap();
        for (edge, use_) in section.edges().iter().zip(section.uses()) {
            for i in 0..=64 {
                let t = i as f64 / 64.;
                let point = edge.curve.try_evaluate(t).unwrap();
                let uv = use_.pcurve.try_evaluate(t).unwrap();
                let actual = body.solid().shell.faces[use_.face_id]
                    .surface
                    .try_evaluate(uv[0], uv[1])
                    .unwrap();
                assert!((point - actual).norm() < 1e-8 * scale);
                let x = point.x / scale;
                let y = point.y / scale;
                let z = point.z / scale;
                assert!((z - 12. - 0.1 * x - 0.05 * y).abs() < 1e-8);
                assert!((x.hypot(y) - (16. - z / 3.)).abs() < 1e-8);
            }
        }
    }
}
