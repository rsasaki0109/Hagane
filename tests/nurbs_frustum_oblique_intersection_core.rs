use hagane::*;
fn split() -> (NurbsFrustumPlaneSplit, GeometryTolerance) {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let n = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    (
        source
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
fn actual_cut_and_lateral_witnesses_partition_segment_material() {
    let (parts, p) = split();
    let start = Point3::new(-20., 2., 10.);
    let end = Point3::new(20., 2., 14.);
    let source = parts
        .lower
        .source()
        .intersect_segment(start, end, p)
        .unwrap();
    let lower = parts.lower.intersect_segment(start, end, p).unwrap();
    let upper = parts.upper.intersect_segment(start, end, p).unwrap();
    let l = lower.material_interval.unwrap();
    let u = upper.material_interval.unwrap();
    let original = source.material_interval.unwrap();
    assert!((l[0] - original[0]).abs() < 1e-12);
    assert!((u[1] - original[1]).abs() < 1e-12);
    assert!((l[1] - u[0]).abs() < 1e-12);
    for (body, result) in [(&parts.lower, &lower), (&parts.upper, &upper)] {
        assert_eq!(result.hits.len(), 2);
        for hit in &result.hits {
            for f in &hit.faces {
                let actual = body.solid().shell.faces[f.face_id]
                    .surface
                    .try_evaluate(f.uv[0], f.uv[1])
                    .unwrap();
                assert!((actual - hit.point).norm() < 1e-10);
            }
        }
        let reverse = body.intersect_segment(end, start, p).unwrap();
        for (a, b) in result.hits.iter().zip(reverse.hits.iter().rev()) {
            assert!((a.parameter + b.parameter - 1.).abs() < 1e-12);
            assert!((a.point - b.point).norm() < 1e-10);
        }
    }
}
#[test]
fn parallel_material_and_removed_segments_and_ambiguous_contact() {
    let (parts, p) = split();
    let start = Point3::new(-2., 0., 5.);
    let end = Point3::new(2., 0., 4.6);
    assert_eq!(
        parts
            .lower
            .intersect_segment(start, end, p)
            .unwrap()
            .material_interval,
        Some([0., 1.])
    );
    assert_eq!(
        parts
            .upper
            .intersect_segment(start, end, p)
            .unwrap()
            .material_interval,
        None
    );
    assert!(parts
        .lower
        .intersect_segment(Point3::new(-2., 0., 12.2), Point3::new(2., 0., 11.8), p)
        .is_err());
    assert!(parts
        .lower
        .intersect_segment(Point3::new(1e100, 0., 0.), Point3::new(-1e100, 0., 0.), p)
        .is_err());
}
