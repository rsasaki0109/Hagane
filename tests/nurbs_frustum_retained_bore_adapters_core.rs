use hagane::*;

#[test]
fn both_legacy_sides_keep_original_coefficients_and_partition_volume() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let n = Vec3::new(0.2, 0.1, 1.).normalized().unwrap();
    let u = (Vec3::new(0., 1., 0.) - n * n.y).normalized().unwrap();
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u,
        v: n.cross(u),
    };
    let split = source.split_by_plane(&plane, p).unwrap();
    for original in [split.lower, split.upper] {
        let before = original.export_step_mm(p).unwrap();
        let parts = original.partition_coaxial_bore(2., p).unwrap();
        assert_eq!(
            format!("{:?}", original.solid()),
            format!("{:?}", parts.kept.outer().solid())
        );
        assert_eq!(before, original.export_step_mm(p).unwrap());
        let sum = parts.kept.volume(p).unwrap() + parts.removed.volume(p).unwrap();
        assert!((sum / original.volume(p).unwrap() - 1.).abs() < 1e-12);
        let larger = parts
            .kept
            .partition_coaxial_bore_enlargement(3., p)
            .unwrap();
        assert_eq!(
            format!("{:?}", larger.removed.inner().solid()),
            format!("{:?}", parts.kept.inner().solid())
        );
        assert_eq!(larger.kept.solid().vertices.len(), 16);
        assert_eq!(larger.removed.solid().edges.len(), 24);
    }
}
