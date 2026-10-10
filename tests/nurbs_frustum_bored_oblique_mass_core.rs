use hagane::*;
fn cut() -> Surface {
    Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    }
}
#[test]
fn horizontal_annular_cylinder_has_actual_half_body_metrics() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let split = source.split_by_plane(&cut(), p).unwrap();
    for (body, z) in [(&split.lower, 6.), (&split.upper, 18.)] {
        let m = body.inertia_properties(p).unwrap();
        let v = std::f64::consts::PI * 12. * 128.;
        assert_eq!(m.volume, body.volume(p).unwrap());
        assert!((m.volume - v).abs() < 1e-9);
        assert!((m.centroid.z - z).abs() < 1e-12);
        assert!((m.inertia[0][0] - v * (40. + 12.)).abs() < 1e-8);
        assert!((m.inertia[2][2] - v * 80.).abs() < 1e-8);
        assert!((body.mass_properties(p).unwrap().centroid - m.centroid).norm() < 1e-12);
    }
}
#[test]
fn unresolved_nearuniform_cone_moments_preserve_exact_successful_geometry() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 16. + 1e-8], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let split = source
        .split_by_plane(
            &Surface::Plane {
                origin: Point3::new(0., 0., 12.),
                u,
                v: n.cross(u),
            },
            p,
        )
        .unwrap();
    for body in [&split.lower, &split.upper] {
        let before = body.export_step_mm(p).unwrap();
        let volume = body.volume(p).unwrap();
        assert!(matches!(
            body.mass_properties(p),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            body.inertia_properties(p),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(body.export_step_mm(p).unwrap(), before);
        assert_eq!(body.volume(p).unwrap(), volume);
    }
}
