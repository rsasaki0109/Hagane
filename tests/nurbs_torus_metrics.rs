use hagane::*;
#[test]
fn torus_metrics_and_actual_control_hull() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let frame = Frame3::new_with_tolerance(
        Point3::new(12., -5., 8.),
        [
            Vec3::new(0., 1., 0.),
            Vec3::new(0., 0., 1.),
            Vec3::new(1., 0., 0.),
        ],
        p,
    )
    .unwrap();
    let body = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
    let volume = 2. * std::f64::consts::PI.powi(2) * 16. * 16.;
    let m = body.mass_properties(p).unwrap();
    assert!((m.volume - volume).abs() < volume * 1e-13);
    assert_eq!(m.centroid, frame.origin());
    let inertia = body.inertia_properties(p).unwrap();
    let transverse = volume * (128. + 10.);
    let longitudinal = volume * (256. + 12.);
    for (i, expected) in [longitudinal, transverse, transverse]
        .into_iter()
        .enumerate()
    {
        assert!((inertia.inertia[i][i] - expected).abs() < expected * 1e-13);
        for j in 0..3 {
            if i != j {
                assert_eq!(inertia.inertia[i][j], 0.);
            }
        }
    }
    let bounds = body.bounds(p).unwrap();
    for face in &body.solid().shell.faces {
        for a in 0..=8 {
            for b in 0..=8 {
                let x = face
                    .surface
                    .try_evaluate(a as f64 / 8., b as f64 / 8.)
                    .unwrap();
                assert!(
                    x.x >= bounds.min.x
                        && x.x <= bounds.max.x
                        && x.y >= bounds.min.y
                        && x.y <= bounds.max.y
                        && x.z >= bounds.min.z
                        && x.z <= bounds.max.z
                );
            }
        }
    }
    let mesh = body.tessellate(0.1, p).unwrap();
    assert!((mesh.signed_volume() - volume).abs() < volume * 0.03);
    assert!(body.tessellate(1e-15, p).is_err());
    assert!(body.tessellate(f64::NAN, p).is_err());
}
