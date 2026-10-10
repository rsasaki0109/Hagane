use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64, operand: f64) {
    assert!(
        (a - b).abs() < 3e-9 * operand,
        "{a} != {b}; operand {operand}"
    );
}
fn plane(frame: Frame3, scale: f64, reversed: bool) -> Surface {
    let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap() * if reversed { -1. } else { 1. };
    let u = Vec3::new(-n.y, n.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 12. * scale)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn oracle(scale: f64, frame: Frame3) -> NurbsGraphInertiaProperties {
    let r = 16. * scale;
    let hole = 4. * scale;
    let d = 12. * scale;
    let a = 0.1;
    let b = 0.05;
    let area = std::f64::consts::PI * (r - hole) * (r + hole);
    let second = std::f64::consts::PI * (r - hole) * (r + hole) * (r * r + hole * hole) / 4.;
    let volume = area * d;
    let first = [
        -a * second,
        -b * second,
        area * d * d / 2. + (a * a + b * b) * second / 2.,
    ];
    let c = first.map(|v| v / volume);
    let raw = [
        [d * second, 0., -d * a * second],
        [0., d * second, -d * b * second],
        [
            -d * a * second,
            -d * b * second,
            area * d * d * d / 3. + d * (a * a + b * b) * second,
        ],
    ];
    let cov: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| raw[i][j] - volume * c[i] * c[j]));
    let trace = cov[0][0] + cov[1][1] + cov[2][2];
    let local: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                trace - cov[i][j]
            } else {
                -cov[i][j]
            }
        })
    });
    let axes = frame.axes().map(|v| [v.x, v.y, v.z]);
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|a| {
                    (0..3)
                        .map(|b| axes[a][i] * local[a][b] * axes[b][j])
                        .sum::<f64>()
                })
                .sum()
        })
    });
    NurbsGraphInertiaProperties {
        volume,
        centroid: frame.point(Point3::new(c[0], c[1], c[2])),
        inertia,
    }
}
#[test]
fn cylinder_annulus_oblique_metrics_match_independent_polynomial_moments() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let rotation =
                Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
            let frame = Frame3::new_with_tolerance(
                Point3::new(13., -7., 5.) * scale,
                rotation.axes(),
                policy(scale),
            )
            .unwrap();
            let body = NurbsFrustumSolid::new(
                frame,
                [16. * scale, 16. * scale],
                24. * scale,
                policy(scale),
            )
            .unwrap()
            .bore_coaxial(4. * scale, policy(scale))
            .unwrap();
            let before = format!("{:?}", body.solid());
            let expected = oracle(scale, frame);
            for reversed in [false, true] {
                let split = body
                    .split_by_plane(&plane(frame, scale, reversed), policy(scale))
                    .unwrap();
                let child = &split.lower;
                let actual = child.inertia_properties(policy(scale)).unwrap();
                let mass = child.mass_properties(policy(scale)).unwrap();
                close(actual.volume, expected.volume, expected.volume);
                close(mass.volume, expected.volume, expected.volume);
                assert!((actual.centroid - expected.centroid).norm() < 3e-9 * scale);
                assert!((mass.centroid - expected.centroid).norm() < 3e-9 * scale);
                for i in 0..3 {
                    for j in 0..3 {
                        close(
                            actual.inertia[i][j],
                            expected.inertia[i][j],
                            expected.volume * scale * scale * 256.,
                        );
                    }
                }
            }
            assert_eq!(before, format!("{:?}", body.solid()));
        }
    }
}
#[test]
fn tapered_children_conserve_volume_first_moment_and_full_world_inertia() {
    let scale = 1.;
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.), rotation.axes(), policy(scale))
            .unwrap();
    for radii in [[16., 8.], [8., 16.]] {
        let body = NurbsFrustumSolid::new(frame, radii, 24., policy(scale))
            .unwrap()
            .bore_coaxial(4., policy(scale))
            .unwrap();
        let before = format!("{:?}", body.solid());
        let split = body
            .split_by_plane(&plane(frame, scale, false), policy(scale))
            .unwrap();
        let parent = body.inertia_properties(policy(scale)).unwrap();
        let mut volume = 0.;
        let mut first = Vec3::new(0., 0., 0.);
        let mut tensor = [[0.; 3]; 3];
        let mut absolute = [[0.; 3]; 3];
        for child in [&split.lower, &split.upper] {
            let p = child.inertia_properties(policy(scale)).unwrap();
            let mass = child.mass_properties(policy(scale)).unwrap();
            close(mass.volume, p.volume, p.volume);
            assert_eq!(mass.centroid, p.centroid);
            volume += p.volume;
            first = first + p.centroid * p.volume;
            let d = p.centroid - parent.centroid;
            let xyz = [d.x, d.y, d.z];
            for i in 0..3 {
                for j in 0..3 {
                    let x = p.inertia[i][j]
                        + p.volume * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
                    tensor[i][j] += x;
                    absolute[i][j] += x.abs();
                }
            }
        }
        close(volume, parent.volume, parent.volume);
        assert!(
            (first - parent.centroid * parent.volume).norm()
                < 3e-9 * parent.volume * frame.origin().norm()
        );
        for i in 0..3 {
            for j in 0..3 {
                close(
                    tensor[i][j],
                    parent.inertia[i][j],
                    absolute[i][j].max(parent.volume),
                );
            }
        }
        assert_eq!(before, format!("{:?}", body.solid()));
    }
}

#[test]
fn unresolved_thin_metrics_preserve_actual_geometry_and_step_export() {
    let p = policy(1.);
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 16.], 24., p)
        .unwrap()
        .bore_coaxial(16. - 2e-5, p)
        .unwrap();
    let before = format!("{:?}", body.solid());
    match body.split_by_plane(&plane(Frame3::IDENTITY, 1., false), p) {
        Ok(split) => {
            for child in [&split.lower, &split.upper] {
                let actual = format!("{:?}", child.solid());
                let step = child.export_step_mm(p).unwrap();
                match child.inertia_properties(p) {
                    Ok(props) => {
                        assert!(props.volume.is_finite() && props.volume > 0.);
                        assert!(props.inertia.iter().flatten().all(|v| v.is_finite()));
                    }
                    Err(Error::Unsupported(_)) => {}
                    Err(error) => panic!("unexpected metric failure {error:?}"),
                }
                assert_eq!(actual, format!("{:?}", child.solid()));
                assert_eq!(step, child.export_step_mm(p).unwrap());
            }
        }
        Err(Error::Unsupported(_)) => {}
        Err(error) => panic!("unexpected geometry failure {error:?}"),
    }
    assert_eq!(before, format!("{:?}", body.solid()));
}
