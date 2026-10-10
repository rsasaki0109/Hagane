use hagane::*;
fn setup() -> (NurbsFrustumSolid, GeometryTolerance) {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    (
        NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap(),
        p,
    )
}
#[test]
fn finite_material_intervals_actual_caps_seams_inside_and_empty() {
    let (body, p) = setup();
    let side = body
        .intersect_segment(Point3::new(-20., 0., 12.), Point3::new(20., 0., 12.), p)
        .unwrap();
    assert_eq!(side.hits.len(), 2);
    let interval = side.material_interval.unwrap();
    assert!((interval[0] - 0.2).abs() < 1e-12);
    assert!((interval[1] - 0.8).abs() < 1e-12);
    assert!(side.hits.iter().all(|h| h.faces.len() == 2));
    let caps = body
        .intersect_segment(Point3::new(1., 1., -4.), Point3::new(1., 1., 28.), p)
        .unwrap();
    assert_eq!(
        caps.hits
            .iter()
            .map(|h| h.faces[0].face_id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    for hit in side.hits.iter().chain(&caps.hits) {
        for f in &hit.faces {
            let q = body.solid().shell.faces[f.face_id]
                .surface
                .try_evaluate(f.uv[0], f.uv[1])
                .unwrap();
            assert!((q - hit.point).norm() < p.linear());
        }
    }
    assert_eq!(
        body.intersect_segment(Point3::new(0., 0., 8.), Point3::new(1., 1., 10.), p)
            .unwrap()
            .material_interval,
        Some([0., 1.])
    );
    assert!(body
        .intersect_segment(Point3::new(20., 20., 8.), Point3::new(21., 20., 10.), p)
        .unwrap()
        .material_interval
        .is_none());
    let inside = body
        .intersect_segment(Point3::new(0., 0., 12.), Point3::new(20., 0., 12.), p)
        .unwrap();
    assert_eq!(inside.material_interval.unwrap()[0], 0.);
}
#[test]
fn posed_reverse_and_unsupported_contacts_are_checked() {
    let (_, p) = setup();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), rotation.axes(), p).unwrap();
    let body = NurbsFrustumSolid::new(frame, [16., 8.], 24., p).unwrap();
    let start = frame.point(Point3::new(-20., 2., 12.));
    let end = frame.point(Point3::new(20., 2., 12.));
    let a = body.intersect_segment(start, end, p).unwrap();
    let b = body.intersect_segment(end, start, p).unwrap();
    assert!((a.hits[0].parameter - (1. - b.hits[1].parameter)).abs() < 1e-12);
    let (body, p) = setup();
    for (a, b) in [
        (Point3::new(16., 0., 0.), Point3::new(8., 0., 24.)),
        (Point3::new(-20., 12., 12.), Point3::new(20., 12., 12.)),
        (Point3::new(0., 0., 0.), Point3::new(0., 0., 20.)),
    ] {
        assert!(body.intersect_segment(a, b, p).is_err());
    }
    assert!(body
        .intersect_segment(Point3::new(0., 0., 1.), Point3::new(0., 0., 1.), p)
        .is_err());
}
