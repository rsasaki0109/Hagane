use hagane::*;
fn parts() -> (NurbsBoredFrustumPlaneSplit, GeometryTolerance) {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let n = Vec3::new(0.2, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    (
        NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p)
            .unwrap()
            .bore_coaxial(4., p)
            .unwrap()
            .split_by_plane(
                &Surface::Plane {
                    origin: Point3::new(0., 0., 12.),
                    u,
                    v: n.cross(u),
                },
                p,
            )
            .unwrap(),
        p,
    )
}
#[test]
fn actual_inner_and_outer_walls_remain_two_material_intervals() {
    let (parts, p) = parts();
    let start = Point3::new(-20., 1., 6.);
    let end = Point3::new(20., 1., 6.);
    let r = parts.lower.intersect_segment(start, end, p).unwrap();
    assert_eq!(r.hits.len(), 4);
    assert_eq!(r.material_intervals.len(), 2);
    for hit in &r.hits {
        for f in &hit.faces {
            let q = parts.lower.solid().shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((q - hit.point).norm() < 1e-10);
        }
    }
    let reverse = parts.lower.intersect_segment(end, start, p).unwrap();
    for (a, b) in r.hits.iter().zip(reverse.hits.iter().rev()) {
        assert!((a.parameter + b.parameter - 1.).abs() < 1e-12);
    }
    let empty = parts
        .lower
        .intersect_segment(Point3::new(0., 0., -5.), Point3::new(0., 0., 29.), p)
        .unwrap();
    assert!(empty.hits.is_empty());
    assert!(empty.material_intervals.is_empty());
}
#[test]
fn cut_root_in_void_has_no_filled_disk_hit_and_material_cut_has_true_cap() {
    let (parts, p) = parts();
    let start = Point3::new(-20., 0., 12.);
    let end = Point3::new(20., 0., 12.);
    let a = parts.lower.intersect_segment(start, end, p).unwrap();
    let b = parts.upper.intersect_segment(start, end, p).unwrap();
    assert_eq!(a.material_intervals.len(), 1);
    assert_eq!(b.material_intervals.len(), 1);
    assert!(a
        .hits
        .iter()
        .chain(&b.hits)
        .all(|h| h.faces.iter().all(|f| f.face_id >= 2)));
    let start = Point3::new(8., 0., 3.);
    let end = Point3::new(8., 0., 18.);
    let cut = parts.lower.intersect_segment(start, end, p).unwrap();
    assert_eq!(cut.hits.len(), 1);
    assert_eq!(cut.hits[0].faces[0].face_id, 1);
    assert_eq!(cut.material_intervals[0][0], 0.);
    assert!(parts
        .lower
        .intersect_segment(Point3::new(4., 0., 6.), Point3::new(8., 0., 6.), p)
        .is_err());
}
