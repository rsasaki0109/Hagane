use hagane::*;
#[test]
fn finite_slab_caps_arcs_and_physical_rims_use_actual_provenance() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    for heights in [[-1., 1.], [0.1, 0.3], [-0.3, -0.1], [0., 1.]] {
        let slab = source.slab_between_local_z(heights, p).unwrap();
        let before = slab.export_step_mm(p).unwrap();
        let caps = slab.solid().shell.faces.len() - 2;
        let mid = heights[0] + (heights[1] - heights[0]) * 0.5;
        assert_eq!(
            slab.classify_point(Point3::new(16., 0., mid), p).unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            slab.classify_point(Point3::new(0., 0., mid), p).unwrap(),
            PointLocation::Outside
        );
        for (i, height) in heights.into_iter().enumerate() {
            let query = slab.point_query(Point3::new(16., 0., height), p).unwrap();
            assert_eq!(query.location, PointLocation::Boundary);
            let witness = query.boundary.unwrap();
            assert_eq!(witness.face, caps + i);
            assert!(if i == 0 {
                witness.normal.z < 0.
            } else {
                witness.normal.z > 0.
            });
            assert!(
                (slab.solid().shell.faces[witness.face]
                    .surface
                    .try_evaluate(witness.uv[0], witness.uv[1])
                    .unwrap()
                    - witness.point)
                    .norm()
                    < 1e-12
            );
            let radius = 16. + ((4. - height) * (4. + height)).sqrt();
            assert!(slab
                .point_query(Point3::new(radius, 0., height), p)
                .is_err());
        }
        let radius = 16. + ((4. - mid) * (4. + mid)).sqrt();
        let wall = slab.point_query(Point3::new(radius, 0., mid), p).unwrap();
        assert_eq!(wall.location, PointLocation::Boundary);
        assert!(wall.boundary.unwrap().face < caps);
        assert_eq!(
            slab.classify_point(Point3::new(16., 0., heights[1] + 0.05), p)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(before, slab.export_step_mm(p).unwrap());
    }
    let slab = source.slab_between_local_z([-1., 0.], p).unwrap();
    assert_eq!(
        slab.classify_point(Point3::new(20. + 0.8e-6, 0., 0.8e-6), p)
            .unwrap(),
        PointLocation::Outside
    );
    assert!(slab
        .classify_point(Point3::new(20. + 0.6e-6, 0., 0.6e-6), p)
        .is_err());
    assert!(slab.classify_point(Point3::new(1e16, 0., 0.), p).is_err());
}
