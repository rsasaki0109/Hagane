use hagane::*;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn stock() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-10., -8., 0.),
            size: Vec3::new(20., 16., 5.),
        },
        policy().absolute(),
    )
    .unwrap()
}
#[test]
fn explicit_axis_resolves_box_height_and_preserves_old_domains() {
    let s = stock();
    let original = format!("{s:?}");
    for (axis, height) in [
        (Vec3::new(1., 0., 0.), 20.),
        (Vec3::new(0., 1., 0.), 16.),
        (Vec3::new(0., 0., 1.), 5.),
    ] {
        let result = bore_normal_prism(&s, Point3::new(0., 0., 2.5), 1., axis, policy()).unwrap();
        let expected = std::f64::consts::PI * height;
        assert!((result.direct_removed_volume() - expected).abs() < 1e-12);
        assert!((result.removed().volume().unwrap() - expected).abs() < 1e-10);
        assert!((result.kept().volume().unwrap() + expected - s.volume().unwrap()).abs() < 1e-9);
        result.kept().validate(policy().absolute()).unwrap();
        let opposite =
            bore_normal_prism(&s, Point3::new(0., 0., 2.5), 1., axis * -1e100, policy()).unwrap();
        assert_eq!(
            format!("{:?}", result.kept()),
            format!("{:?}", opposite.kept())
        );
    }
    assert!(bore_normal_arc_line_prism(&s, Point3::new(0., 0., 0.), 1., policy()).is_err());
    assert_eq!(original, format!("{s:?}"));
}
#[test]
fn plain_partition_can_continue_with_bore_and_another_partition() {
    let s = stock();
    let axis = Vec3::new(0., 0., 1.);
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: axis,
        v: Vec3::new(0., -1., 0.),
    };
    let split = split_normal_prism_by_plane_components(&s, &plane, axis, policy()).unwrap();
    assert_eq!((split.negative().len(), split.positive().len()), (1, 1));
    assert!((split.positive()[0].volume().unwrap() - 800.).abs() < 1e-10);
    let bored = bore_normal_prism(
        &split.positive()[0],
        Point3::new(5., 0., 37.),
        1.,
        axis,
        policy(),
    )
    .unwrap();
    let cut = Surface::Plane {
        origin: Point3::new(7., 0., 0.),
        u: axis,
        v: Vec3::new(0., -1., 0.),
    };
    let again = split_normal_prism_by_plane_components(bored.kept(), &cut, axis, policy()).unwrap();
    let sum: f64 = again
        .negative()
        .iter()
        .chain(again.positive())
        .map(|s| s.volume().unwrap())
        .sum();
    assert!((sum - bored.kept().volume().unwrap()).abs() < 1e-9);
    assert!(split_normal_arc_line_prism_by_plane_components(&s, &plane, policy()).is_err());
}
#[test]
fn invalid_axis_and_unresolved_origin_never_mutate_source() {
    let s = stock();
    let original = format!("{s:?}");
    for axis in [
        Vec3::new(0., 0., 0.),
        Vec3::new(f64::NAN, 0., 1.),
        Vec3::new(1., 1., 1.),
    ] {
        assert!(bore_normal_prism(&s, Point3::new(0., 0., 0.), 1., axis, policy()).is_err());
    }
    assert!(bore_normal_prism(
        &s,
        Point3::new(0., 0., 1e12),
        1.,
        Vec3::new(0., 0., 1.),
        policy()
    )
    .is_err());
    assert_eq!(original, format!("{s:?}"));
}
