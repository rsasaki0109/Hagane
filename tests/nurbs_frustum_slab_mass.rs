use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64, operand: f64) {
    assert!((a - b).abs() < 3e-9 * operand, "{a} != {b}");
}
fn plane(f: Frame3, s: f64, d: f64) -> Surface {
    let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
    let u = Vec3::new(-n.y, n.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| f.axes()[0] * p.x + f.axes()[1] * p.y + f.axes()[2] * p.z;
    Surface::Plane {
        origin: f.point(Point3::new(0., 0., d * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn translated(p: &Surface, f: Frame3, d: f64) -> Surface {
    let Surface::Plane { origin, u, v } = p else {
        panic!("plane")
    };
    Surface::Plane {
        origin: *origin + f.axes()[2] * d,
        u: *u,
        v: *v,
    }
}
fn oracle(f: Frame3, s: f64, hole: f64) -> NurbsGraphInertiaProperties {
    let r = 16. * s;
    let r0 = hole * s;
    let delta = 4. * s;
    let c = (r * r + r0 * r0) / 4.;
    let volume = std::f64::consts::PI * (r - r0) * (r + r0) * delta;
    let cov = [
        [c, 0., -0.1 * c],
        [0., c, -0.05 * c],
        [-0.1 * c, -0.05 * c, delta * delta / 12. + 0.0125 * c],
    ];
    let trace = cov[0][0] + cov[1][1] + cov[2][2];
    let local: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| volume * ((if i == j { trace } else { 0. }) - cov[i][j]))
    });
    let axes = f.axes().map(|p| [p.x, p.y, p.z]);
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
        centroid: f.point(Point3::new(0., 0., 10. * s)),
        inertia,
    }
}
fn compare(a: NurbsGraphInertiaProperties, b: NurbsGraphInertiaProperties, s: f64) {
    close(a.volume, b.volume, b.volume);
    assert!((a.centroid - b.centroid).norm() < 3e-9 * s);
    for i in 0..3 {
        for j in 0..3 {
            close(a.inertia[i][j], b.inertia[i][j], b.volume * s * s * 256.);
        }
    }
}
#[test]
fn cylindrical_slab_covariance_oracle_matches_canonical_and_bored_all_world_components() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let rot =
                Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
            let f =
                Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s))
                    .unwrap();
            let p = policy(s);
            let source = NurbsFrustumSolid::new(f, [16. * s, 16. * s], 24. * s, p).unwrap();
            let first_plane = plane(f, s, 8.);
            let second = translated(&first_plane, f, 4. * s);
            let third = translated(&first_plane, f, 8. * s);
            let first = source.split_by_plane(&first_plane, p).unwrap();
            let slab = first
                .upper
                .split_by_parallel_plane(&second, p)
                .unwrap()
                .lower;
            let before = format!("{:?}", slab.solid());
            let step = slab.export_step_mm(p).unwrap();
            compare(slab.inertia_properties(p).unwrap(), oracle(f, s, 0.), s);
            let mass = slab.mass_properties(p).unwrap();
            close(
                mass.volume,
                oracle(f, s, 0.).volume,
                oracle(f, s, 0.).volume,
            );
            assert!((mass.centroid - oracle(f, s, 0.).centroid).norm() < 3e-9 * s);
            let reversed = first
                .upper
                .split_by_parallel_plane(&third, p)
                .unwrap()
                .lower
                .split_by_parallel_plane(&second, p)
                .unwrap()
                .lower;
            compare(
                reversed.inertia_properties(p).unwrap(),
                slab.inertia_properties(p).unwrap(),
                s,
            );
            assert_eq!(before, format!("{:?}", slab.solid()));
            assert_eq!(step, slab.export_step_mm(p).unwrap());
            let bored = source.bore_coaxial(4. * s, p).unwrap();
            let first = bored.split_by_plane(&first_plane, p).unwrap();
            let slab = first
                .upper
                .split_by_parallel_plane(&second, p)
                .unwrap()
                .lower;
            compare(slab.inertia_properties(p).unwrap(), oracle(f, s, 4.), s);
            let mass = slab.mass_properties(p).unwrap();
            close(
                mass.volume,
                oracle(f, s, 4.).volume,
                oracle(f, s, 4.).volume,
            );
            assert!((mass.centroid - oracle(f, s, 4.).centroid).norm() < 3e-9 * s);
            let reversed = first
                .upper
                .split_by_parallel_plane(&third, p)
                .unwrap()
                .lower
                .split_by_parallel_plane(&second, p)
                .unwrap()
                .lower;
            compare(
                reversed.inertia_properties(p).unwrap(),
                slab.inertia_properties(p).unwrap(),
                s,
            );
        }
    }
}
fn conserve(parent: NurbsGraphInertiaProperties, parts: &[NurbsGraphInertiaProperties]) {
    let mut v = 0.;
    let mut m = Vec3::new(0., 0., 0.);
    let mut tensor = [[0.; 3]; 3];
    let mut operands = [[0.; 3]; 3];
    for part in parts {
        v += part.volume;
        m = m + part.centroid * part.volume;
        let d = part.centroid - parent.centroid;
        let xyz = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                let x = part.inertia[i][j]
                    + part.volume * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
                tensor[i][j] += x;
                operands[i][j] += x.abs();
            }
        }
    }
    close(v, parent.volume, parent.volume);
    assert!(
        (m - parent.centroid * parent.volume).norm()
            < 3e-9 * parent.volume * parent.centroid.norm()
    );
    for i in 0..3 {
        for j in 0..3 {
            close(
                tensor[i][j],
                parent.inertia[i][j],
                operands[i][j].max(parent.volume),
            );
        }
    }
}
#[test]
fn expanding_contracting_repeat_slabs_conserve_full_mass_and_inertia() {
    let p = policy(1.);
    let f = Transform::translation(Vec3::new(13., -7., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap())
        .unwrap();
    for radii in [[16., 8.], [8., 16.]] {
        let source = NurbsFrustumSolid::new(f, radii, 24., p).unwrap();
        let plane = plane(f, 1., 8.);
        let first = source.split_by_plane(&plane, p).unwrap();
        let repeated = first
            .upper
            .split_by_parallel_plane(&translated(&plane, f, 4.), p)
            .unwrap();
        conserve(
            source.inertia_properties(p).unwrap(),
            &[
                first.lower.inertia_properties(p).unwrap(),
                repeated.lower.inertia_properties(p).unwrap(),
                repeated.upper.inertia_properties(p).unwrap(),
            ],
        );
        let bored = source.bore_coaxial(4., p).unwrap();
        let first = bored.split_by_plane(&plane, p).unwrap();
        let repeated = first
            .upper
            .split_by_parallel_plane(&translated(&plane, f, 4.), p)
            .unwrap();
        conserve(
            bored.inertia_properties(p).unwrap(),
            &[
                first.lower.inertia_properties(p).unwrap(),
                repeated.lower.inertia_properties(p).unwrap(),
                repeated.upper.inertia_properties(p).unwrap(),
            ],
        );
    }
}
#[test]
fn unresolved_metric_conditioning_keeps_valid_actual_shape_and_step() {
    let p = policy(1.);
    for radii in [[16., 16. - 1e-12], [16., 16.]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., p).unwrap();
        let hole = if radii[0] == radii[1] { 16. - 2e-5 } else { 4. };
        let body = source.bore_coaxial(hole, p).unwrap();
        let plane = plane(Frame3::IDENTITY, 1., 8.);
        match body.split_by_plane(&plane, p) {
            Ok(first) => {
                let slab = first
                    .upper
                    .split_by_parallel_plane(&translated(&plane, Frame3::IDENTITY, 4.), p)
                    .unwrap()
                    .lower;
                let before = format!("{:?}", slab.solid());
                let step = slab.export_step_mm(p).unwrap();
                match slab.inertia_properties(p) {
                    Ok(props) => {
                        assert!(props.volume > 0.);
                        assert!(props.inertia.iter().flatten().all(|v| v.is_finite()));
                    }
                    Err(Error::Unsupported(_)) => {}
                    Err(e) => panic!("{e:?}"),
                };
                assert_eq!(before, format!("{:?}", slab.solid()));
                assert_eq!(step, slab.export_step_mm(p).unwrap());
            }
            Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
}
