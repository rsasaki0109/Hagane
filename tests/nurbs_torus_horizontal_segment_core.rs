use hagane::*;
#[test]
fn actual_annular_cap_clips_material_without_void_or_removed_hits() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let split = source.split_by_local_z(1., p).unwrap();
    let start = Point3::new(16., 0., -8.);
    let end = Point3::new(16., 0., 8.);
    for body in [split.lower, split.upper] {
        let before = body.export_step_mm(p).unwrap();
        let query = body.intersect_segment(start, end, p).unwrap();
        assert_eq!(query.material_intervals.len(), 1);
        assert_eq!(query.hits.len(), 2);
        let expected = if body.is_lower() {
            [0.25, 0.5625]
        } else {
            [0.5625, 0.75]
        };
        for (actual, expected) in query.material_intervals[0].into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-8);
        }
        for hit in &query.hits {
            let witness = &hit.boundary;
            let actual = body.solid().shell.faces[witness.face]
                .surface
                .try_evaluate(witness.uv[0], witness.uv[1])
                .unwrap();
            assert!((actual - witness.point).norm() < 1e-12);
            assert!(witness.distance < 1e-7);
        }
        let reverse = body.intersect_segment(end, start, p).unwrap();
        for (a, b) in query.hits.iter().zip(reverse.hits.iter().rev()) {
            assert!((a.parameter + b.parameter - 1.).abs() < 1e-8);
        }
        let empty = body
            .intersect_segment(Point3::new(-4., 0., 1.), Point3::new(4., 0., 1.), p)
            .unwrap();
        assert!(empty.hits.is_empty() && empty.material_intervals.is_empty());
        assert_eq!(before, body.export_step_mm(p).unwrap());
    }
}
