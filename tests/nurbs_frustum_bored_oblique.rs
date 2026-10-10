use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn plane(frame: Frame3, scale: f64, height: f64) -> Surface {
    let normal = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
    let u = Vec3::new(-normal.y, normal.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    Surface::Plane {
        origin: frame.point(Point3::new(0., 0., height * scale)),
        u: vector(u),
        v: vector(normal.cross(u)),
    }
}
fn topology(s: &Solid, scale: f64) {
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (16, 24, 10)
    );
    assert_eq!(s.shell.faces[0].wires.len(), 2);
    assert_eq!(s.shell.faces[1].wires.len(), 2);
    let mut uses = [(0, 0); 24];
    for face in &s.shell.faces {
        for c in face.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].0 += 1;
            uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
            for i in 0..=32 {
                let t = i as f64 / 32.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                let a = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let b = s.edges[c.edge].curve.try_evaluate(t).unwrap();
                assert!((a - b).norm() < 1e-8 * scale);
            }
        }
    }
    assert!(uses.iter().all(|v| *v == (2, 0)));
}
#[test]
fn actual_annular_children_conserve_cylinder_and_tapered_volume_under_pose_scale() {
    for scale in [1e-4, 1., 10.] {
        for radii in [[16., 16.], [16., 8.]] {
            let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
            let frame = Frame3::new_with_tolerance(
                Point3::new(13., -7., 5.) * scale,
                rotation.axes(),
                policy(scale),
            )
            .unwrap();
            let source =
                NurbsFrustumSolid::new(frame, radii.map(|r| r * scale), 24. * scale, policy(scale))
                    .unwrap()
                    .bore_coaxial(4. * scale, policy(scale))
                    .unwrap();
            let before = format!("{:?}", source.solid());
            let original = source.volume(policy(scale)).unwrap();
            let cut = source
                .split_by_plane(&plane(frame, scale, 12.), policy(scale))
                .unwrap();
            let mut volumes = Vec::new();
            for child in [&cut.lower, &cut.upper] {
                child.validate(policy(scale)).unwrap();
                topology(child.solid(), scale);
                let volume = child.volume(policy(scale)).unwrap();
                assert!(volume > 0.);
                volumes.push(volume);
                let step = child.export_step_mm(policy(scale)).unwrap();
                assert!(step.contains("MANIFOLD_SOLID_BREP"));
                assert_eq!(step.matches("FACE_BOUND(").count(), 2);
                assert_eq!(child.source().radius(), source.radius());
            }
            assert!(cut.lower.is_lower());
            assert!(!cut.upper.is_lower());
            assert!((volumes.iter().sum::<f64>() - original).abs() < 2e-10 * original);
            if radii[0] == radii[1] {
                let expected = std::f64::consts::PI * (16. * 16. - 4. * 4.) * 12. * scale.powi(3);
                for v in volumes {
                    assert!((v - expected).abs() < 2e-10 * expected);
                }
            }
            for (upper, lower) in [(4, 0), (16, 12)] {
                for q in 0..4 {
                    for t in [0., 0.23, 0.71, 1.] {
                        let a = cut.lower.solid().edges[upper + q]
                            .curve
                            .try_evaluate(t)
                            .unwrap();
                        let b = cut.upper.solid().edges[lower + q]
                            .curve
                            .try_evaluate(t)
                            .unwrap();
                        assert!((a - b).norm() < 1e-9 * scale);
                    }
                }
            }
            let a = &cut.lower.solid().shell.faces[1];
            let b = &cut.upper.solid().shell.faces[0];
            let na = a.surface.normal_at(0., 0.).unwrap() * a.orientation as f64;
            let nb = b.surface.normal_at(0., 0.).unwrap() * b.orientation as f64;
            assert!(na.dot(nb) < -0.999999);
            assert_eq!(before, format!("{:?}", source.solid()));
        }
    }
}
#[test]
fn failed_cut_domains_keep_actual_source_and_recover() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy(1.))
        .unwrap()
        .bore_coaxial(4., policy(1.))
        .unwrap();
    let before = format!("{:?}", source.solid());
    let step = source.export_step_mm(policy(1.)).unwrap();
    for height in [0., 24., -1., 25., f64::NAN, f64::INFINITY, 1e-7] {
        assert!(source
            .split_by_plane(&plane(Frame3::IDENTITY, 1., height), policy(1.))
            .is_err());
    }
    let crossing = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    };
    assert!(source.split_by_plane(&crossing, policy(1.)).is_err());
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap();
    assert!(source
        .split_by_plane(&plane(Frame3::IDENTITY, 1., 0.2), relative)
        .is_err());
    assert_eq!(before, format!("{:?}", source.solid()));
    assert_eq!(step, source.export_step_mm(policy(1.)).unwrap());
    assert!(source
        .split_by_plane(&plane(Frame3::IDENTITY, 1., 12.), policy(1.))
        .is_ok());
}
