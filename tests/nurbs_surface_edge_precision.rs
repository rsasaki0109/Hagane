use hagane::{Error, NurbsSurface, Point3};

#[test]
fn recentered_homogeneous_underflow_is_rejected_before_composition() {
    let origin = 1e-300;
    let size = 1e-310;
    let knots = vec![0., 0., 1., 1.];
    // World homogeneous controls remain representable; subtracting the origin
    // makes the low-weight local displacement underflow instead.
    let surface = NurbsSurface::new(
        [1, 1],
        [knots.clone(), knots],
        [2, 2],
        vec![
            Point3::new(origin, origin, origin),
            Point3::new(origin, origin + size, origin),
            Point3::new(origin + size, origin, origin),
            Point3::new(origin + size, origin + size, origin),
        ],
        vec![1., 1e-20, 1., 1.],
    )
    .unwrap();
    assert!(matches!(
        surface.parameter_curve([0., 0.], [1., 1.]),
        Err(Error::InvalidInput(
            "surface parameter curve weighted local control underflows"
        ))
    ));
}

#[test]
fn excessive_crossing_count_is_rejected_before_patch_extraction() {
    let count = 4098;
    let mut u = vec![0., 0.];
    u.extend((1..count - 1).map(|i| i as f64 / (count - 1) as f64));
    u.extend([1., 1.]);
    let points = (0..count)
        .flat_map(|i| (0..2).map(move |j| Point3::new(i as f64, j as f64, 0.)))
        .collect();
    let surface = NurbsSurface::new(
        [1, 1],
        [u, vec![0., 0., 1., 1.]],
        [count, 2],
        points,
        vec![1.; count * 2],
    )
    .unwrap();
    assert!(matches!(
        surface.parameter_curve([0., 0.], [1., 1.]),
        Err(Error::Unsupported(
            "surface parameter curve exceeds span or control limit"
        ))
    ));
}
