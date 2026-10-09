use hagane::*;
#[test]
fn resolved_small_bore_volume_avoids_total_volume_subtraction() {
    let tol = Tolerance::new(5e-10).unwrap();
    let source = NurbsGraphSolid::new([1., 1., 1.], 0., tol).unwrap();
    let radius = 1e-8;
    let body = NurbsGraphCircularHoledSolid::new(&source, [0.5, 0.5], radius, tol).unwrap();
    let expected = std::f64::consts::PI * radius * radius;
    let actual = body.removed_volume(tol).unwrap();
    assert!((actual / expected - 1.).abs() < 1e-13);
    let subtracted = source.volume().unwrap() - body.volume().unwrap();
    assert!((subtracted / expected - 1.).abs() > 1e-3);
}
fn compare(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) {
    let scale = b.iter().flatten().copied().map(f64::abs).fold(0., f64::max);
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (a[i][j] - b[i][j]).abs() < 2e-11 * scale,
                "[{i}][{j}] {} != {}",
                a[i][j],
                b[i][j]
            );
        }
    }
}
fn shifted(mut a: [[f64; 3]; 3], m: f64, d: Point3) -> [[f64; 3]; 3] {
    let v = [d.x, d.y, d.z];
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] += m * (if i == j { d.dot(d) } else { 0. } - v[i] * v[j]);
        }
    }
    a
}
fn box_tensor(d: [f64; 3]) -> [[f64; 3]; 3] {
    let m = d.iter().product::<f64>();
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                m * (d[(i + 1) % 3].powi(2) + d[(i + 2) % 3].powi(2)) / 12.
            } else {
                0.
            }
        })
    })
}

fn disk_volume(source: &NurbsGraphSolid, c: [f64; 2], r: f64) -> f64 {
    let [w, d, h] = source.dimensions();
    let b = source.bulge();
    let u = c[0] / w;
    let v = c[1] / d;
    std::f64::consts::PI
        * r
        * r
        * (h + 4. * b * u * (1. - u) * v * (1. - v)
            - b * r * r * (v * (1. - v) / (w * w) + u * (1. - u) / (d * d))
            + b * r.powi(4) / (6. * w * w * d * d))
}
#[test]
fn flat_offcenter_circular_bore_matches_closed_box_cylinder_parallel_axis_oracle() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t).unwrap();
    let c = [2., 2.25];
    let r = 0.5;
    let body = NurbsGraphCircularHoledSolid::new(&source, c, r, t).unwrap();
    let p = body.inertia_properties(t).unwrap();
    let removed = std::f64::consts::PI * r * r * 2.;
    let volume = 96. - removed;
    let whole_center = Point3::new(4., 3., 1.);
    let cut_center = Point3::new(c[0], c[1], 1.);
    let center = (whole_center * 96. - cut_center * removed) * (1. / volume);
    let cylinder = [
        [removed * (3. * r * r + 4.) / 12., 0., 0.],
        [0., removed * (3. * r * r + 4.) / 12., 0.],
        [0., 0., removed * r * r / 2.],
    ];
    let full = shifted(box_tensor([8., 6., 2.]), 96., whole_center - center);
    let cut = shifted(cylinder, removed, cut_center - center);
    compare(
        p.inertia,
        std::array::from_fn(|i| std::array::from_fn(|j| full[i][j] - cut[i][j])),
    );
    assert!((p.volume - volume).abs() < 1e-11);
    assert!((p.centroid - center).norm() < 1e-12);
}
#[test]
fn signed_trimmed_graph_removed_volume_matches_independent_laplacian_formula() {
    let t = Tolerance::default();
    for b in [-2., 0., 20.] {
        let source = NurbsGraphSolid::new([20., 12., 3.], b, t)
            .unwrap()
            .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], t)
            .unwrap();
        let c = [9., 5.];
        let r = 1.25;
        let body = NurbsGraphCircularHoledSolid::new(&source, c, r, t).unwrap();
        let expected = source.volume().unwrap() - disk_volume(&source, c, r);
        assert!((body.volume().unwrap() - expected).abs() < expected * 2e-12);
    }
}
#[test]
fn placed_centroid_and_tensor_use_local_centering_and_world_rotation() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], 20., t).unwrap();
    let c = [9., 5.];
    let r = 1.25;
    let local = NurbsGraphCircularHoledSolid::new(&source, c, r, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap();
    let frame = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let placed_source = source.transformed(frame, t).unwrap();
    let p = NurbsGraphCircularHoledSolid::new(&placed_source, c, r, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    let axes = rotation.axes();
    let matrix: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| [axes[j].x, axes[j].y, axes[j].z][i]));
    let expected = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .flat_map(|k| {
                    (0..3).map(move |l| matrix[i][k] * local.inertia[k][l] * matrix[j][l])
                })
                .sum()
        })
    });
    compare(p.inertia, expected);
    assert_eq!(p.volume, local.volume);
    assert!((p.centroid - frame.point(local.centroid)).norm() < 1e-11);
}
#[test]
fn scaled_moments_and_overflow_or_mutation_failures_remain_explicit() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t).unwrap();
    let reference = NurbsGraphCircularHoledSolid::new(&source, [2., 2.25], 0.5, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    for scale in [1e-50, 1e-6, 1., 1e6, 1e50, 1e-80, 1e100] {
        let t = Tolerance::new(scale * 1e-7).unwrap();
        let source = NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], 0., t).unwrap();
        let mut body =
            NurbsGraphCircularHoledSolid::new(&source, [2. * scale, 2.25 * scale], 0.5 * scale, t)
                .unwrap();
        let result = body.inertia_properties(t);
        if scale == 1e-80 || scale == 1e100 {
            assert!(body.mass_properties(t).is_ok());
            assert!(result.is_err());
        } else {
            let actual = result.unwrap();
            compare(
                actual.inertia.map(|r| r.map(|v| v / scale.powi(5))),
                reference.inertia,
            );
        }
        body.solid.vertices[0].point.x += t.linear / 100.;
        assert!(body.mass_properties(t).is_err());
        assert!(body.inertia_properties(t).is_err());
    }
    assert!(NurbsGraphCircularHoledSolid::new(&source, [0.5, 3.], 0.5, t).is_err());
}

#[test]
fn curved_disk_first_and_second_moments_match_closed_monomial_integrals() {
    use std::collections::BTreeMap;
    type Polynomial = BTreeMap<(usize, usize), f64>;
    let factorial = |n: usize| (1..=n).map(|v| v as f64).product::<f64>();
    let choose = |n: usize, k: usize| factorial(n) / (factorial(k) * factorial(n - k));
    let center: [f64; 2] = [9., 5.];
    let radius: f64 = 1.25;
    let moment = |m: usize, n: usize| {
        let mut sum = 0.;
        for a in 0..=m / 2 {
            for b in 0..=n / 2 {
                let even = std::f64::consts::PI
                    * radius.powi((2 * a + 2 * b + 2) as i32)
                    * factorial(2 * a)
                    * factorial(2 * b)
                    / (4f64.powi((a + b) as i32)
                        * factorial(a)
                        * factorial(b)
                        * factorial(a + b + 1));
                sum += choose(m, 2 * a)
                    * choose(n, 2 * b)
                    * center[0].powi((m - 2 * a) as i32)
                    * center[1].powi((n - 2 * b) as i32)
                    * even;
            }
        }
        sum
    };
    let multiply = |a: &Polynomial, b: &Polynomial| {
        let mut out = Polynomial::new();
        for (&(i, j), x) in a {
            for (&(k, l), y) in b {
                *out.entry((i + k, j + l)).or_default() += x * y;
            }
        }
        out
    };
    let integrate = |p: &Polynomial, x: usize, y: usize| {
        p.iter()
            .map(|(&(i, j), v)| v * moment(i + x, j + y))
            .sum::<f64>()
    };
    let t = Tolerance::default();
    for bulge in [-2., 20.] {
        let source = NurbsGraphSolid::new([20., 12., 3.], bulge, t).unwrap();
        let whole = source.inertia_properties(t).unwrap();
        let h = Polynomial::from([
            ((0, 0), 3.),
            ((1, 1), 4. * bulge / (20. * 12.)),
            ((2, 1), -4. * bulge / (400. * 12.)),
            ((1, 2), -4. * bulge / (20. * 144.)),
            ((2, 2), 4. * bulge / (400. * 144.)),
        ]);
        let h2 = multiply(&h, &h);
        let h3 = multiply(&h2, &h);
        let removed = integrate(&h, 0, 0);
        let first = Point3::new(
            integrate(&h, 1, 0),
            integrate(&h, 0, 1),
            integrate(&h2, 0, 0) / 2.,
        );
        let volume = whole.volume - removed;
        let centroid = (whole.centroid * whole.volume - first) * (1. / volume);
        let zz = integrate(&h3, 0, 0) / 3.;
        let xx = integrate(&h, 2, 0);
        let yy = integrate(&h, 0, 2);
        let xy = -integrate(&h, 1, 1);
        let xz = -integrate(&h2, 1, 0) / 2.;
        let yz = -integrate(&h2, 0, 1) / 2.;
        let removed_raw = [[yy + zz, xy, xz], [xy, xx + zz, yz], [xz, yz, xx + yy]];
        let full_raw = shifted(whole.inertia, whole.volume, whole.centroid);
        let material_raw =
            std::array::from_fn(|i| std::array::from_fn(|j| full_raw[i][j] - removed_raw[i][j]));
        let expected = shifted(material_raw, -volume, centroid);
        let actual = NurbsGraphCircularHoledSolid::new(&source, center, radius, t)
            .unwrap()
            .inertia_properties(t)
            .unwrap();
        compare(actual.inertia, expected);
        assert!((actual.centroid - centroid).norm() < 1e-10);
    }
}
