use hagane::*;
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

fn rect(a: [f64; 2], b: [f64; 2]) -> Vec<[f64; 2]> {
    vec![a, [b[0], a[1]], b, [a[0], b[1]]]
}
fn holes() -> Vec<Vec<[f64; 2]>> {
    vec![
        rect([0.125, 0.25], [0.375, 0.5]),
        rect([0.625, 0.125], [0.875, 0.375]),
        rect([0.5, 0.625], [0.75, 0.875]),
    ]
}
fn outer(source: &NurbsGraphSolid, t: Tolerance) -> NurbsGraphPolygonSolid {
    NurbsGraphPolygonSolid::new(source, rect([0., 0.], [1., 1.]), t).unwrap()
}
#[test]
fn two_and_three_offcenter_openings_match_analytic_box_moments() {
    let t = Tolerance::default();
    let d = [8., 6., 2.];
    let source = NurbsGraphSolid::new(d, 0., t).unwrap();
    for count in [2, 3] {
        let h = holes()[..count].to_vec();
        let body = NurbsGraphPolygonMultiHoledSolid::new(&outer(&source, t), h.clone(), t).unwrap();
        let mut volume = 96.;
        let mut first = Point3::new(4., 3., 1.) * volume;
        let mut raw = shifted(box_tensor(d), volume, Point3::new(4., 3., 1.));
        for p in h {
            let hd = [8. * (p[2][0] - p[0][0]), 6. * (p[2][1] - p[0][1]), 2.];
            let v = hd.iter().product::<f64>();
            let c = Point3::new(4. * (p[0][0] + p[2][0]), 3. * (p[0][1] + p[2][1]), 1.);
            let r = shifted(box_tensor(hd), v, c);
            volume -= v;
            first = first - c * v;
            for i in 0..3 {
                for j in 0..3 {
                    raw[i][j] -= r[i][j];
                }
            }
        }
        let c = first * (1. / volume);
        let expected = shifted(raw, -volume, c);
        let actual = body.inertia_properties(t).unwrap();
        assert!((actual.volume - volume).abs() < 1e-11);
        assert!((actual.centroid - c).norm() < 1e-12);
        compare(actual.inertia, expected);
    }
}
#[test]
fn curved_positive_material_agrees_with_well_conditioned_subtraction_oracle_and_rotation() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 4., t).unwrap();
    let h = holes()[..2].to_vec();
    let body = NurbsGraphPolygonMultiHoledSolid::new(&outer(&source, t), h.clone(), t).unwrap();
    let whole = source.inertia_properties(t).unwrap();
    let removed: Vec<_> = h
        .iter()
        .map(|p| {
            NurbsGraphPolygonSolid::new(&source, p.clone(), t)
                .unwrap()
                .inertia_properties(t)
                .unwrap()
        })
        .collect();
    let v = whole.volume - removed.iter().map(|p| p.volume).sum::<f64>();
    let c = (whole.centroid * whole.volume
        - removed
            .iter()
            .fold(Point3::new(0., 0., 0.), |s, p| s + p.centroid * p.volume))
        * (1. / v);
    let mut expected = shifted(whole.inertia, whole.volume, whole.centroid - c);
    for p in removed {
        let cut = shifted(p.inertia, p.volume, p.centroid - c);
        for i in 0..3 {
            for j in 0..3 {
                expected[i][j] -= cut[i][j];
            }
        }
    }
    let actual = body.inertia_properties(t).unwrap();
    assert!((actual.centroid - c).norm() < 1e-12);
    compare(actual.inertia, expected);
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap();
    let frame = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let placed_source = source.transformed(frame, t).unwrap();
    let placed = NurbsGraphPolygonMultiHoledSolid::new(&outer(&placed_source, t), h, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    let axes = rotation.axes();
    let r: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| [axes[j].x, axes[j].y, axes[j].z][i]));
    let rotated = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .flat_map(|k| (0..3).map(move |l| r[i][k] * actual.inertia[k][l] * r[j][l]))
                .sum()
        })
    });
    compare(placed.inertia, rotated);
    assert!((placed.centroid - frame.point(actual.centroid)).norm() < 1e-11);
}
#[test]
fn multi_hole_scaling_overflow_and_strict_mutation_guards() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t).unwrap();
    let reference =
        NurbsGraphPolygonMultiHoledSolid::new(&outer(&source, t), holes()[..2].to_vec(), t)
            .unwrap()
            .inertia_properties(t)
            .unwrap();
    for scale in [1e-50, 1., 1e50, 1e-80, 1e100] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], 0., t).unwrap();
        let mut body =
            NurbsGraphPolygonMultiHoledSolid::new(&outer(&source, t), holes()[..2].to_vec(), t)
                .unwrap();
        let result = body.inertia_properties(t);
        if scale == 1e-80 || scale == 1e100 {
            assert!(result.is_err());
        } else {
            let p = result.unwrap();
            compare(
                p.inertia.map(|r| r.map(|x| x / scale.powi(5))),
                reference.inertia,
            );
        }
        body.solid.vertices[0].point.x += t.linear / 100.;
        assert!(body.mass_properties(t).is_err());
        assert!(body.inertia_properties(t).is_err());
    }
}
#[test]
fn dyadic_thin_frame_mass_and_centered_inertia_stay_positive() {
    let t = Tolerance::new(1e-10).unwrap();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let d = 2f64.powi(-26);
    let body = NurbsGraphPolygonMultiHoledSolid::new(
        &outer(&source, t),
        vec![rect([d, d], [1. - d, 1. - d])],
        t,
    )
    .unwrap();
    let p = body.inertia_properties(t).unwrap();
    let area = 4. * d * (1. - d);
    assert!((p.volume - area).abs() < area * 2e-13);
    assert!((p.centroid - Point3::new(0.5, 0.5, 0.5)).norm() < 2e-13);
    // Stable 1-(1-2d)^4 factorization avoids nearly equal stock tensors.
    let s = 1. - 2. * d;
    let xy = 2. * d * (1. + s) * (1. + s * s) / 12.;
    let expected = [
        [xy + area / 12., 0., 0.],
        [0., xy + area / 12., 0.],
        [0., 0., 2. * xy],
    ];
    compare(p.inertia, expected);
}
