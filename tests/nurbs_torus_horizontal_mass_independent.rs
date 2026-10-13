use hagane::*;
use std::f64::consts::PI;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn source(s: f64) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(frame, [16. * s, 4. * s], policy(s)).unwrap()
}
// Independent positive disk-annular slice integration, z=r*sin(theta).
// This integrates physical density and raw moments, not the production primitives.
fn oracle(c: f64, lower: bool, s: f64) -> (f64, f64, [f64; 3]) {
    let t = (c / (4. * s)).asin();
    let (lo, hi) = if lower { (-PI / 2., t) } else { (t, PI / 2.) };
    let n = 8192;
    let h = (hi - lo) / n as f64;
    let mut sums = [0.; 4];
    for i in 0..=n {
        let theta = lo + h * i as f64;
        let a = 4. * s * theta.cos();
        let z = 4. * s * theta.sin();
        let jac = 4. * s * theta.cos();
        let area = 4. * PI * 16. * s * a;
        let values = [
            area * jac,
            area * z * jac,
            2. * PI * 16. * s * a * ((16. * s).powi(2) + a * a) * jac,
            area * z * z * jac,
        ];
        let w = if i == 0 || i == n {
            1.
        } else if i % 2 == 0 {
            2.
        } else {
            4.
        };
        for (sum, v) in sums.iter_mut().zip(values) {
            *sum += w * v * h / 3.;
        }
    }
    let [v, mz, qxx, qzz] = sums;
    let z = mz / v;
    (
        v,
        z,
        [qxx + qzz - v * z * z, qxx + qzz - v * z * z, 2. * qxx],
    )
}
fn rotated(frame: Frame3, diagonal: [f64; 3]) -> [[f64; 3]; 3] {
    let a = frame.axes();
    let rows = [
        [a[0].x, a[1].x, a[2].x],
        [a[0].y, a[1].y, a[2].y],
        [a[0].z, a[1].z, a[2].z],
    ];
    let mut result = [[0.; 3]; 3];
    for (i, row) in result.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = (0..3).map(|k| rows[i][k] * diagonal[k] * rows[j][k]).sum();
        }
    }
    result
}
fn check(child: &NurbsTorusHorizontalSolid, s: f64) -> (f64, Point3, [[f64; 3]; 3]) {
    let p = policy(s);
    let before = format!("{:?}", child.solid());
    let step = child.export_step_mm(p).unwrap();
    let (v, z, diag) = oracle(child.cut_height(), child.is_lower(), s);
    let mass = child.mass_properties(p).unwrap();
    let inertia = child.inertia_properties(p).unwrap();
    let expected = child.source().frame().point(Point3::new(0., 0., z));
    assert!((mass.volume - v).abs() < v * 2e-9);
    assert!((mass.centroid - expected).norm() < 2e-8 * s);
    assert!((inertia.centroid - mass.centroid).norm() < 1e-10 * s);
    let tensor = rotated(child.source().frame(), diag);
    let envelope = diag.into_iter().fold(0., f64::max);
    for (actual, want) in inertia
        .inertia
        .iter()
        .flatten()
        .zip(tensor.iter().flatten())
    {
        assert!(
            (actual - want).abs() < envelope * 2e-9,
            "{actual} != {want}"
        );
    }
    assert_eq!(before, format!("{:?}", child.solid()));
    assert_eq!(step, child.export_step_mm(p).unwrap());
    (mass.volume, mass.centroid, inertia.inertia)
}
#[test]
fn exact_half_closed_form_and_signed_slice_integrals_match_all_world_tensor_entries() {
    for s in [1e-4, 1., 10.] {
        let body = source(s);
        let before = format!("{:?}", body.solid());
        let step = body.export_step_mm(policy(s)).unwrap();
        for c in [0., 1.25, -1.25, 3.2, -3.2] {
            let split = body.split_by_local_z(c * s, policy(s)).unwrap();
            for child in [&split.lower, &split.upper] {
                check(child, s);
            }
            if c == 0. {
                let expected = 4. * 4. * s / (3. * PI);
                let half_v = body.volume(policy(s)).unwrap() / 2.;
                let transverse = half_v * ((128. + 10.) * s * s - expected * expected);
                let axial = half_v * (256. + 12.) * s * s;
                let closed_tensor = rotated(body.frame(), [transverse, transverse, axial]);
                for child in [&split.lower, &split.upper] {
                    let inertia = child.inertia_properties(policy(s)).unwrap();
                    for (a, b) in inertia
                        .inertia
                        .iter()
                        .flatten()
                        .zip(closed_tensor.iter().flatten())
                    {
                        assert!((a - b).abs() < axial * 1e-11);
                    }
                }

                for child in [&split.lower, &split.upper] {
                    let centroid = body
                        .frame()
                        .local_point(child.mass_properties(policy(s)).unwrap().centroid);
                    assert!(
                        (centroid.z
                            - (if child.is_lower() {
                                -expected
                            } else {
                                expected
                            }))
                        .abs()
                            < 1e-10 * s
                    );
                    assert!(
                        (child.mass_properties(policy(s)).unwrap().volume
                            - body.volume(policy(s)).unwrap() / 2.)
                            .abs()
                            < body.volume(policy(s)).unwrap() * 1e-12
                    );
                }
            }
        }
        assert_eq!(before, format!("{:?}", body.solid()));
        assert_eq!(step, body.export_step_mm(policy(s)).unwrap());
    }
}
#[test]
fn child_first_moments_and_parallel_axis_tensor_recover_parent_including_thin_caps() {
    for s in [1e-4, 1., 10.] {
        let body = source(s);
        let p = policy(s);
        let parent = body.inertia_properties(p).unwrap();
        for c in [0., 1.25, -1.25, 4. - 0.001, -4. + 0.001] {
            let split = body.split_by_local_z(c * s, p).unwrap();
            let metrics = [check(&split.lower, s), check(&split.upper, s)];
            let volume = metrics.iter().map(|m| m.0).sum::<f64>();
            assert!((volume - parent.volume).abs() < parent.volume * 2e-9);
            let center = metrics.iter().fold(Vec3::new(0., 0., 0.), |a, m| {
                a + (m.1 - parent.centroid) * m.0
            });
            assert!(center.norm() < parent.volume * s * 2e-8);
            let mut combined = [[0.; 3]; 3];
            for (v, c, tensor) in metrics {
                let d = c - parent.centroid;
                let xyz = [d.x, d.y, d.z];
                for (i, row) in combined.iter_mut().enumerate() {
                    for (j, value) in row.iter_mut().enumerate() {
                        *value += tensor[i][j]
                            + v * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
                    }
                }
            }
            let envelope = parent
                .inertia
                .iter()
                .flatten()
                .map(|x| x.abs())
                .fold(0., f64::max);
            for (actual, want) in combined
                .iter()
                .flatten()
                .zip(parent.inertia.iter().flatten())
            {
                assert!((actual - want).abs() < envelope * 3e-9);
            }
        }
    }
}
#[test]
fn stricter_unresolved_policy_rejects_metrics_without_changing_geometry_and_recovers() {
    let body = source(1.);
    let split = body.split_by_local_z(3.2, policy(1.)).unwrap();
    let broad = GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap();
    for child in [&split.lower, &split.upper] {
        let before = format!("{:?}", child.solid());
        let step = child.export_step_mm(policy(1.)).unwrap();
        assert!(child.mass_properties(broad).is_err());
        assert!(child.inertia_properties(broad).is_err());
        check(child, 1.);
        assert_eq!(before, format!("{:?}", child.solid()));
        assert_eq!(step, child.export_step_mm(policy(1.)).unwrap());
    }
}

#[test]
fn representable_geometry_and_volume_do_not_turn_inertia_overflow_into_success() {
    let scale = 1e62;
    let p = policy(scale);
    let body = source(scale);
    let split = body.split_by_local_z(0., p).unwrap();
    let child = &split.lower;
    let before = format!("{:?}", child.solid());
    child.validate(p).unwrap();
    assert!(child.volume(p).unwrap().is_finite());
    assert!(child.mass_properties(p).unwrap().volume.is_finite());
    assert!(matches!(
        child.inertia_properties(p),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(before, format!("{:?}", child.solid()));
}
