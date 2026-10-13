use hagane::*;
#[test]
fn both_caps_clip_source_intervals_without_provisional_or_void_hits() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let slab = source.slab_between_local_z([-1., 1.], p).unwrap();
    let before = slab.export_step_mm(p).unwrap();
    let start = Point3::new(16., 0., -8.);
    let end = Point3::new(16., 0., 8.);
    let query = slab.intersect_segment(start, end, p).unwrap();
    assert_eq!(query.material_intervals, vec![[0.4375, 0.5625]]);
    assert_eq!(query.hits.len(), 2);
    let caps = slab.solid().shell.faces.len() - 2;
    for (i, hit) in query.hits.iter().enumerate() {
        assert_eq!(hit.boundary.face, caps + i);
        let w = &hit.boundary;
        assert!(
            (slab.solid().shell.faces[w.face]
                .surface
                .try_evaluate(w.uv[0], w.uv[1])
                .unwrap()
                - w.point)
                .norm()
                < 1e-12
        );
    }
    let reverse = slab.intersect_segment(end, start, p).unwrap();
    for (a, b) in query.hits.iter().zip(reverse.hits.iter().rev()) {
        assert_eq!(a.parameter + b.parameter, 1.);
        assert_eq!(a.boundary.face, b.boundary.face);
    }
    let across = slab
        .intersect_segment(Point3::new(-24., 0., 0.), Point3::new(24., 0., 0.), p)
        .unwrap();
    assert_eq!(across.hits.len(), 4);
    assert_eq!(across.material_intervals.len(), 2);
    assert!(across.hits.iter().all(|h| h.boundary.face < caps));
    let empty = slab
        .intersect_segment(Point3::new(0., 0., -8.), Point3::new(0., 0., 8.), p)
        .unwrap();
    assert!(empty.hits.is_empty() && empty.material_intervals.is_empty());
    let inside = slab
        .intersect_segment(Point3::new(16., 0., -0.5), Point3::new(16., 0., 0.5), p)
        .unwrap();
    assert_eq!(inside.material_intervals, vec![[0., 1.]]);
    assert!(inside.hits.is_empty());
    assert!(slab
        .intersect_segment(Point3::new(15., 0., 1.), Point3::new(17., 0., 1.), p)
        .is_err());
    assert_eq!(before, slab.export_step_mm(p).unwrap());
}
