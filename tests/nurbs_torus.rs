use hagane::*;
#[test]
fn actual_tensor_ring_is_closed_and_rejects_coefficient_corruption() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], policy).unwrap();
    assert_eq!(
        (
            body.solid().vertices.len(),
            body.solid().edges.len(),
            body.solid().shell.faces.len()
        ),
        (16, 32, 16)
    );
    assert!(
        (body.volume(policy).unwrap() / (2. * std::f64::consts::PI.powi(2) * 16. * 16.) - 1.).abs()
            < 1e-14
    );
    for face in &body.solid().shell.faces {
        for u in [0.13, 0.47, 0.82] {
            for v in [0.09, 0.35, 0.79] {
                let point = face.surface.try_evaluate(u, v).unwrap();
                let radial = point.x.hypot(point.y);
                assert!(((radial - 16.).hypot(point.z) - 4.).abs() < 1e-12);
                let normal = face.surface.normal_at(u, v).unwrap();
                let expected = Vec3::new(
                    point.x * (1. - 16. / radial),
                    point.y * (1. - 16. / radial),
                    point.z,
                );
                assert!(normal.dot(expected) > 0.);
            }
        }
    }
    let mut bad = body.solid().clone();
    bad.shell.faces[0].orientation = -1;
    assert!(NurbsTorusSolid::from_brep(bad, body.frame(), body.radii(), policy).is_err());
    assert!(NurbsTorusSolid::new(Frame3::IDENTITY, [4., 4.], policy).is_err());
    assert!(NurbsTorusSolid::new(Frame3::IDENTITY, [4., 5.], policy).is_err());
}
