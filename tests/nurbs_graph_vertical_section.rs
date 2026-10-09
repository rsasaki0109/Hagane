use hagane::*;
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-4, 1e-10, 1e-12).unwrap()
}
#[test]
fn placed_trimmed_section_has_physical_height_events_and_actual_cap_normals() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([8., 6., 2.], 12., tol.absolute())
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol.absolute())
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(12., -7., 4.))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
                .unwrap(),
            tol.absolute(),
        )
        .unwrap();
    let uv = [0.3, 0.4];
    let section = source.vertical_section(uv, tol).unwrap();
    section.validate(tol).unwrap();
    let h = 2. + 48. * 0.3 * 0.7 * 0.4 * 0.6;
    assert_eq!(section.intervals, vec![[0., h]]);
    assert_eq!(section.events.len(), 2);
    assert_eq!(section.events[0].face, 0);
    assert_eq!(section.events[1].face, 1);
    assert!(section.events[0].entering);
    assert!(!section.events[1].entering);
    assert!((section.direction.norm() - 1.).abs() < 1e-14);
    for event in &section.events {
        assert!((event.point - section.evaluate(event.parameter).unwrap()).norm() < 1e-12);
        let Surface::Nurbs(surface) = &source.brep().shell.faces[event.face].surface else {
            panic!()
        };
        assert!((event.point - surface.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-12);
    }
    assert!(section.events[0].normal.dot(section.direction) < 0.);
    assert!(section.events[1].normal.dot(section.direction) > 0.);
    assert_eq!(section.segments[0].range(), [0., 1.]);
    assert_eq!(
        section.segments[0].try_evaluate(0.).unwrap(),
        section.events[0].point
    );
    let mut dirty = section.clone();
    dirty.intervals[0][1] += 1e-12;
    assert!(dirty.validate(tol).is_err());
    assert!(dirty.evaluate(1.).is_err());
}
#[test]
fn opening_and_exterior_are_empty_but_wall_contact_is_explicit() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol.absolute()).unwrap();
    let body =
        NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.2, 0.6]], tol.absolute()).unwrap();
    for uv in [[0.5, 0.4], [-0.2, 0.5], [1.2, 0.5]] {
        let section = body.vertical_section(uv, tol).unwrap();
        assert!(section.intervals.is_empty());
        assert!(section.events.is_empty());
        assert!(section.segments.is_empty());
        section.validate(tol).unwrap();
    }
    let material = body.vertical_section([0.1, 0.1], tol).unwrap();
    assert_eq!(material.intervals.len(), 1);
    for uv in [[0., 0.5], [0.3, 0.4], [0.5, 0.6], [0.7, 0.2]] {
        assert!(body.vertical_section(uv, tol).is_err());
    }
    let mut dirty = body.clone();
    dirty.solid.vertices[0].point.x += 1e-12;
    assert!(dirty.vertical_section([-2., 0.5], tol).is_err());
    assert!(body.vertical_section([f64::NAN, 0.5], tol).is_err());
    assert!(body.vertical_section([1e200, 0.5], tol).is_err());
}
#[test]
fn corner_contact_uses_euclidean_distance_and_tiny_bands_fail_explicitly() {
    let tol = tolerance();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., tol.absolute()).unwrap();
    let d = tol.linear();
    assert!(source
        .vertical_section([-0.8 * d / 8., -0.8 * d / 6.], tol)
        .unwrap()
        .intervals
        .is_empty());
    assert!(source
        .vertical_section([-0.6 * d / 8., -0.6 * d / 6.], tol)
        .is_err());
    assert!(source
        .vertical_section(
            [0.5, 0.5],
            GeometryTolerance::new(1e-20, 1e-10, 0.).unwrap()
        )
        .is_err());
}
