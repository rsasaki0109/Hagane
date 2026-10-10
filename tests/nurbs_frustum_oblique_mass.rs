use hagane::*;

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn frame(origin: Vec3, posed: bool) -> Frame3 {
    Transform::translation(origin)
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.7 } else { 0. }).unwrap())
        .unwrap()
}
fn plane(source: &NurbsFrustumSolid, c: f64, a: f64, b: f64, reverse: bool) -> Surface {
    let u = Vec3::new(1., 0., -a).normalized().unwrap();
    let n = Vec3::new(a, b, 1.).normalized().unwrap();
    let v = n.cross(u).normalized().unwrap() * if reverse { -1. } else { 1. };
    Surface::Plane {
        origin: source.frame().point(Point3::new(0., 0., c)),
        u: source.frame().vector(u),
        v: source.frame().vector(v),
    }
}
// Integrate positive cylindrical Jacobians independently of production cone
// moments. Three axial Gauss nodes integrate every degree <=5 exactly.
fn quadrature(r: [f64; 2], c: f64, a: f64, b: f64, lower: bool) -> (f64, Vec3, [[f64; 3]; 3]) {
    let nodes = [
        0.1834346424956498,
        0.525532409916329,
        0.7966664774136267,
        0.9602898564975363,
    ];
    let weights = [
        0.362683783378362,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    ];
    let s = (r[1] - r[0]) / 24.;
    let mut volume = 0.;
    let mut first = [0.; 3];
    let mut second = [[0.; 3]; 3];
    let dz = 0.6_f64.sqrt() / 2.;
    for i in 0..4 {
        for sign in [-1., 1.] {
            let rho = (1. + sign * nodes[i]) / 2.;
            for j in 0..512 {
                let theta = std::f64::consts::TAU * (j as f64 + 0.5) / 512.;
                let q = rho * (a * theta.cos() + b * theta.sin());
                let cut = (c - r[0] * q) / (1. + s * q);
                let (base, depth) = if lower { (0., cut) } else { (cut, 24. - cut) };
                for (t, w) in [(0.5 - dz, 5. / 18.), (0.5, 4. / 9.), (0.5 + dz, 5. / 18.)] {
                    let z = base + depth * t;
                    let radius = r[0] + s * z;
                    let xyz = [radius * rho * theta.cos(), radius * rho * theta.sin(), z];
                    let dv = weights[i] / 2. * std::f64::consts::TAU / 512.
                        * w
                        * depth
                        * radius
                        * radius
                        * rho;
                    volume += dv;
                    for k in 0..3 {
                        first[k] += dv * xyz[k];
                        for l in 0..3 {
                            second[k][l] += dv * xyz[k] * xyz[l];
                        }
                    }
                }
            }
        }
    }
    let center = Vec3::new(first[0] / volume, first[1] / volume, first[2] / volume);
    let components = [center.x, center.y, center.z];
    let covariance: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| second[i][j] - volume * components[i] * components[j])
    });
    let trace = covariance[0][0] + covariance[1][1] + covariance[2][2];
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| (if i == j { trace } else { 0. }) - covariance[i][j])
    });
    (volume, center, inertia)
}
fn rotated(local: [[f64; 3]; 3], frame: Frame3) -> [[f64; 3]; 3] {
    let axes = frame.axes().map(|a| [a.x, a.y, a.z]);
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .flat_map(|k| (0..3).map(move |l| axes[k][i] * local[k][l] * axes[l][j]))
                .sum()
        })
    })
}
fn close(a: f64, b: f64, scale: f64) {
    assert!((a - b).abs() < 2e-9 * scale, "{a} vs {b}; scale{scale}");
}

#[test]
fn independent_positive_moments_signed_tapers_cylinder_pose_and_reversal() {
    for r in [[16., 8.], [8., 16.], [12., 12.]] {
        for posed in [false, true] {
            for reverse in [false, true] {
                let source = NurbsFrustumSolid::new(
                    frame(Vec3::new(12., -3., 5.), posed),
                    r,
                    24.,
                    policy(1.),
                )
                .unwrap();
                let split = source
                    .split_by_plane(&plane(&source, 12., 0.1, -0.05, reverse), policy(1.))
                    .unwrap();
                for (child, lower) in [(&split.lower, true), (&split.upper, false)] {
                    let (v, c, local) = quadrature(r, 12., 0.1, -0.05, lower);
                    let mass = child.mass_properties(policy(1.)).unwrap();
                    let properties = child.inertia_properties(policy(1.)).unwrap();
                    close(mass.volume, v, v);
                    assert_eq!(mass.volume, child.volume(policy(1.)).unwrap());
                    assert!((mass.centroid - source.frame().point(c)).norm() < 1e-8);
                    assert!((properties.centroid - mass.centroid).norm() < 1e-10);
                    let expected = rotated(local, source.frame());
                    let scale = expected[0][0] + expected[1][1] + expected[2][2];
                    for (i, row) in expected.iter().enumerate() {
                        for (j, value) in row.iter().enumerate() {
                            close(properties.inertia[i][j], *value, scale);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn nonzero_offdiagonal_and_parallel_axis_conservation() {
    let source = NurbsFrustumSolid::new(
        frame(Vec3::new(1000., -2000., 1000.), true),
        [16., 8.],
        24.,
        policy(1.),
    )
    .unwrap();
    let split = source
        .split_by_plane(&plane(&source, 12., 0.1, -0.05, false), policy(1.))
        .unwrap();
    let parent = source.inertia_properties(policy(1.)).unwrap();
    let mut volume = 0.;
    let mut moment = Vec3::new(0., 0., 0.);
    let mut inertia = [[0.; 3]; 3];
    for child in [&split.lower, &split.upper] {
        let p = child.inertia_properties(policy(1.)).unwrap();
        volume += p.volume;
        moment = moment + p.centroid * p.volume;
        let d = p.centroid - parent.centroid;
        let xyz = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                inertia[i][j] += p.inertia[i][j]
                    + p.volume * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
            }
        }
    }
    close(volume, parent.volume, parent.volume);
    assert!((moment * (1. / volume) - parent.centroid).norm() < 1e-8);
    let scale = parent.inertia[0][0] + parent.inertia[1][1] + parent.inertia[2][2];
    for (i, row) in inertia.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            close(*value, parent.inertia[i][j], scale);
        }
    }
    let cylinder = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., policy(1.)).unwrap();
    let lower = cylinder
        .split_by_plane(&plane(&cylinder, 12., 0.1, -0.05, false), policy(1.))
        .unwrap()
        .lower;
    let p = lower.inertia_properties(policy(1.)).unwrap();
    assert!(p.inertia[0][1].abs() > 1.);
    assert!(p.inertia[0][2].abs() > 1.);
    assert!(p.inertia[1][2].abs() > 1.);
}

#[test]
fn normalized_scale_and_unresolved_nearuniform_metrics_preserve_geometry() {
    let reference = quadrature([16., 8.], 12., 0.1, -0.05, true);
    for scale in [1e-50, 1e50] {
        let p = policy(scale);
        let source =
            NurbsFrustumSolid::new(Frame3::IDENTITY, [16. * scale, 8. * scale], 24. * scale, p)
                .unwrap();
        let lower = source
            .split_by_plane(&plane(&source, 12. * scale, 0.1, -0.05, false), p)
            .unwrap()
            .lower;
        let mass = lower.mass_properties(p).unwrap();
        let inertia = lower.inertia_properties(p).unwrap();
        close(
            mass.volume / (scale * scale * scale),
            reference.0,
            reference.0,
        );
        assert!((mass.centroid * (1. / scale) - reference.1).norm() < 1e-8);
        let physical = scale.powi(5);
        for i in 0..3 {
            for j in 0..3 {
                close(
                    inertia.inertia[i][j] / physical,
                    reference.2[i][j],
                    reference.2[0][0] + reference.2[1][1] + reference.2[2][2],
                );
            }
        }
    }
    let source =
        NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12. + 1e-10], 24., policy(1.)).unwrap();
    let lower = source
        .split_by_plane(&plane(&source, 12., 0.1, -0.05, false), policy(1.))
        .unwrap()
        .lower;
    let before = lower.export_step_mm(policy(1.)).unwrap();
    assert!(matches!(
        lower.mass_properties(policy(1.)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        lower.inertia_properties(policy(1.)),
        Err(Error::Unsupported(_))
    ));
    assert!(lower.volume(policy(1.)).unwrap() > 0.);
    assert_eq!(lower.export_step_mm(policy(1.)).unwrap(), before);
    lower.validate(policy(1.)).unwrap();
}

#[test]
fn thin_covariance_and_physical_tensor_overflow_are_explicit() {
    let cylinder = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., policy(1.)).unwrap();
    let thin = cylinder
        .split_by_plane(&plane(&cylinder, 1.1e-5, 0., 0., false), policy(1.))
        .unwrap()
        .lower;
    assert!(thin.volume(policy(1.)).unwrap() > 0.);
    assert!(matches!(
        thin.mass_properties(policy(1.)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        thin.inertia_properties(policy(1.)),
        Err(Error::Unsupported(_))
    ));
    for c in [0.01, 23.99] {
        let split = cylinder
            .split_by_plane(&plane(&cylinder, c, 0., 0., false), policy(1.))
            .unwrap();
        for child in [&split.lower, &split.upper] {
            child.mass_properties(policy(1.)).unwrap();
            child.inertia_properties(policy(1.)).unwrap();
        }
    }
    for scale in [1e-80, 1e80] {
        let p = policy(scale);
        let source =
            NurbsFrustumSolid::new(Frame3::IDENTITY, [16. * scale, 8. * scale], 24. * scale, p)
                .unwrap();
        let child = source
            .split_by_plane(&plane(&source, 12. * scale, 0.1, -0.05, false), p)
            .unwrap()
            .lower;
        assert!(child.mass_properties(p).unwrap().volume.is_finite());
        assert!(matches!(
            child.inertia_properties(p),
            Err(Error::Unsupported(_))
        ));
        child.validate(p).unwrap();
    }
}
