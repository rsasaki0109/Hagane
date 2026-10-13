use hagane::*;
#[test]
fn half_centered_tensor_and_thin_cap_are_positive_without_world_subtraction() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let halves = source.split_by_local_z(0., p).unwrap();
    for body in [halves.lower, halves.upper] {
        let m = body.inertia_properties(p).unwrap();
        let sign = if body.is_lower() { -1. } else { 1. };
        let mean = sign * 16. / (3. * std::f64::consts::PI);
        assert!((m.centroid.z - mean).abs() < 1e-12);
        assert_eq!(m.volume, body.volume(p).unwrap());
        let radial = 128. + 6.;
        let variance = 4. - mean * mean;
        assert!((m.inertia[0][0] / m.volume - (radial + variance)).abs() < 1e-10);
        assert!((m.inertia[2][2] / m.volume - 2. * radial).abs() < 1e-10);
    }
    for c in [-3.999, 3.999] {
        let split = source.split_by_local_z(c, p).unwrap();
        let child = if c < 0. { split.lower } else { split.upper };
        let before = child.export_step_mm(p).unwrap();
        let m = child.inertia_properties(p).unwrap();
        assert!(m.inertia[0][0] > 0. && m.inertia[2][2] > 0.);
        assert_eq!(m.volume, child.volume(p).unwrap());
        assert!(if c < 0. {
            m.centroid.z < c
        } else {
            m.centroid.z > c
        });
        assert_eq!(before, child.export_step_mm(p).unwrap());
    }
}
