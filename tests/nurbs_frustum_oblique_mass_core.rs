use hagane::*;
fn plane(z: f64, t: f64) -> Surface {
    let n = Vec3::new(t, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: n.cross(u),
    }
}
#[test]
fn centroid_and_parallel_axis_inertia_recompose_actual_source() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., p).unwrap();
        let split = source.split_by_plane(&plane(12., 0.1), p).unwrap();
        let original = source.inertia_properties(p).unwrap();
        let parts = [
            split.lower.inertia_properties(p).unwrap(),
            split.upper.inertia_properties(p).unwrap(),
        ];
        for i in 0..3 {
            let component = |x: Point3| [x.x, x.y, x.z][i];
            let mean = parts
                .iter()
                .map(|m| m.volume * component(m.centroid))
                .sum::<f64>()
                / original.volume;
            assert!((mean - component(original.centroid)).abs() < 1e-10);
        }
        for i in 0..3 {
            for j in 0..3 {
                let mut sum = 0.;
                for m in &parts {
                    let d = m.centroid - original.centroid;
                    let ds = [d.x, d.y, d.z];
                    sum += m.inertia[i][j]
                        + m.volume * (if i == j { d.dot(d) } else { 0. } - ds[i] * ds[j]);
                }
                assert!((sum - original.inertia[i][j]).abs() < 1e-6);
            }
        }
    }
}
#[test]
fn cylinder_offaxis_centroid_and_unresolved_cones_are_explicit() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let split = source.split_by_plane(&plane(10., 0.2), p).unwrap();
    let m = split.lower.mass_properties(p).unwrap();
    assert!((m.centroid.x - (-0.2 * 144. / 40.)).abs() < 1e-12);
    assert!((m.centroid.z - (5. + 0.04 * 144. / 80.)).abs() < 1e-12);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 16. + 1e-8], 24., p).unwrap();
    let split = source.split_by_plane(&plane(12., 0.1), p).unwrap();
    assert!(split.lower.volume(p).unwrap() > 0.);
    assert!(matches!(
        split.lower.mass_properties(p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        split.lower.inertia_properties(p),
        Err(Error::Unsupported(_))
    ));
}
