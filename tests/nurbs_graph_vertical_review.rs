use hagane::*;
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn placement() -> Transform {
    Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap()
}
fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], -2., Tolerance::default())
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], Tolerance::default())
        .unwrap()
        .transformed(placement(), Tolerance::default())
        .unwrap()
}
fn check(section: &NurbsGraphVerticalSection, uv: [f64; 2]) {
    let [u, v] = uv;
    let roof = 3. - 8. * u * (1. - u) * v * (1. - v);
    assert_eq!(section.uv, uv);
    assert_eq!(section.intervals.len(), 1);
    assert!((section.intervals[0][0]).abs() < 1e-12);
    assert!((section.intervals[0][1] - roof).abs() < 1e-12);
    let origin = placement().point(Point3::new(20. * u, 12. * v, 0.));
    let direction = placement().vector(Vec3::new(0., 0., 1.));
    assert!((section.origin - origin).norm() < 1e-12);
    assert!((section.direction - direction).norm() < 1e-12);
    assert_eq!(section.events.len(), 2);
    assert_eq!(section.events[0].face, 0);
    assert_eq!(section.events[1].face, 1);
    assert!(section.events[0].entering);
    assert!(!section.events[1].entering);
    assert_eq!(section.segments.len(), 1);
    for (i, event) in section.events.iter().enumerate() {
        let height = if i == 0 { 0. } else { roof };
        assert!((event.parameter - height).abs() < 1e-12);
        assert!((event.point - (origin + direction * height)).norm() < 1e-10);
        let expected = if i == 0 {
            Vec3::new(0., 0., -1.)
        } else {
            Vec3::new(
                8. * (1. - 2. * u) * v * (1. - v) / 20.,
                8. * u * (1. - u) * (1. - 2. * v) / 12.,
                1.,
            )
            .normalized()
            .unwrap()
        };
        assert!((event.normal - placement().vector(expected)).norm() < 1e-10);
    }
    for i in 0..=10 {
        let fraction = i as f64 / 10.;
        assert!(
            (section.segments[0].try_evaluate(fraction).unwrap()
                - (origin + direction * (roof * fraction)))
                .norm()
                < 1e-10
        );
    }
}
#[test]
fn physical_height_intervals_cap_events_normals_and_segments_match_original_uv() {
    let s = source();
    for uv in [[0.3, 0.2], [0.5, 0.4], [0.7, 0.6]] {
        check(&s.vertical_section(uv, tolerance()).unwrap(), uv);
    }
    let h =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.5], [0.25, 0.4]], Tolerance::default()).unwrap();
    for uv in [[0.3, 0.2], [0.6, 0.5]] {
        check(&h.vertical_section(uv, tolerance()).unwrap(), uv);
    }
}
#[test]
fn outside_and_hole_void_are_empty_but_wall_contacts_are_explicit_errors() {
    let s = source();
    let h =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.5], [0.25, 0.4]], Tolerance::default()).unwrap();
    for uv in [[0.1, 0.4], [0.9, 0.4], [0.5, 0.], [0.5, 0.8]] {
        let section = s.vertical_section(uv, tolerance()).unwrap();
        assert!(
            section.intervals.is_empty()
                && section.events.is_empty()
                && section.segments.is_empty()
        );
        assert!((section.direction.norm() - 1.).abs() < 1e-12);
    }
    let empty = h.vertical_section([0.425, 0.325], tolerance()).unwrap();
    assert!(empty.intervals.is_empty() && empty.events.is_empty() && empty.segments.is_empty());
    for uv in [
        [0.2, 0.4],
        [0.2 + 0.5e-6 / 20., 0.4],
        [0.8, 0.4],
        [0.5, 0.1],
        [0.5, 0.7],
    ] {
        assert!(matches!(
            s.vertical_section(uv, tolerance()),
            Err(Error::Unsupported(_))
        ));
    }
    for uv in [
        [0.35, 0.325],
        [0.5, 0.325],
        [0.425, 0.25],
        [0.425, 0.4],
        [0.35 - 0.5e-6 / 20., 0.325],
    ] {
        assert!(matches!(
            h.vertical_section(uv, tolerance()),
            Err(Error::Unsupported(_))
        ));
    }
    let clear = s
        .vertical_section([0.2 + 2e-6 / 20., 0.4], tolerance())
        .unwrap();
    assert_eq!(clear.intervals.len(), 1);
}
#[test]
fn tiny_finite_sections_and_public_corruption_are_checked() {
    let tol = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let tiny = NurbsGraphSolid::new([1e-9; 3], 1e-9, tol.absolute()).unwrap();
    let section = tiny.vertical_section([0.5, 0.5], tol).unwrap();
    assert_eq!(section.intervals.len(), 1);
    assert!((section.intervals[0][1] - 1.25e-9).abs() < 1e-23);
    assert!((section.events[1].point.z - 1.25e-9).abs() < 1e-23);
    let s = source();
    for uv in [[f64::NAN, 0.4], [0.5, f64::INFINITY]] {
        assert!(s.vertical_section(uv, tolerance()).is_err());
    }
    let mut dirty = s.clone();
    dirty.solid.vertices[0].point.x += 0.01;
    assert!(dirty.vertical_section([0.6, 0.5], tolerance()).is_err());
    let mut dirty =
        NurbsGraphHoledSolid::new(&s, [[0.35, 0.5], [0.25, 0.4]], Tolerance::default()).unwrap();
    dirty.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(dirty.vertical_section([0.6, 0.5], tolerance()).is_err());
}
#[test]
fn rectangular_wall_contact_bands_use_euclidean_corner_distance() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([1.; 3], 0., Tolerance::default()).unwrap();
    let hole =
        NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.3, 0.7]], Tolerance::default()).unwrap();
    for distance in [0.8e-6, 1e-6, 2e-6] {
        let outside = source
            .vertical_section([-distance, -distance], tol)
            .unwrap();
        assert!(outside.intervals.is_empty());
        let material = hole
            .vertical_section([0.3 - distance, 0.3 - distance], tol)
            .unwrap();
        assert_eq!(material.intervals, vec![[0., 1.]]);
    }
    for distance in [0.1e-6, 0.5e-6, 0.6e-6] {
        assert!(matches!(
            source.vertical_section([-distance, -distance], tol),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            hole.vertical_section([0.3 - distance, 0.3 - distance], tol),
            Err(Error::Unsupported(_))
        ));
    }
}
