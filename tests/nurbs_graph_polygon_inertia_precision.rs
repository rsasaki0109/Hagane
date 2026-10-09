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
#[test]
fn asymmetric_triangle_matches_independent_simplex_covariance() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t).unwrap();
    let uv = vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]];
    let p: Vec<_> = uv.iter().map(|p| [8. * p[0], 6. * p[1]]).collect();
    let c = [
        p.iter().map(|p| p[0]).sum::<f64>() / 3.,
        p.iter().map(|p| p[1]).sum::<f64>() / 3.,
    ];
    let v = ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]))
        .abs();
    let covariance: [[f64; 2]; 2] = std::array::from_fn(|i| {
        std::array::from_fn(|j| p.iter().map(|p| (p[i] - c[i]) * (p[j] - c[j])).sum::<f64>() / 12.)
    });
    let want = [
        [v * (covariance[1][1] + 1. / 3.), -v * covariance[0][1], 0.],
        [-v * covariance[0][1], v * (covariance[0][0] + 1. / 3.), 0.],
        [0., 0., v * (covariance[0][0] + covariance[1][1])],
    ];
    let actual = NurbsGraphPolygonSolid::new(&source, uv, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    assert!((actual.volume - v).abs() < 1e-12);
    assert!((actual.centroid - Point3::new(c[0], c[1], 1.)).norm() < 1e-12);
    assert!(want[0][1].abs() > 0.1);
    compare(actual.inertia, want);
}
#[test]
fn offcenter_polygon_opening_matches_analytic_box_subtraction() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 0., t).unwrap();
    let body = source
        .through_uv_polygon(
            vec![[0.125, 0.25], [0.375, 0.25], [0.375, 0.625], [0.125, 0.625]],
            t,
        )
        .unwrap();
    let whole = 96.;
    let removed = 9.;
    let volume = whole - removed;
    let cw = Point3::new(4., 3., 1.);
    let cr = Point3::new(2., 2.625, 1.);
    let c = (cw * whole - cr * removed) * (1. / volume);
    let a = shifted(box_tensor([8., 6., 2.]), whole, cw - c);
    let b = shifted(box_tensor([2., 2.25, 2.]), removed, cr - c);
    let expected = std::array::from_fn(|i| std::array::from_fn(|j| a[i][j] - b[i][j]));
    let actual = body.inertia_properties(t).unwrap();
    assert!((actual.centroid - c).norm() < 1e-12);
    compare(actual.inertia, expected);
}
#[test]
fn curved_partition_obeys_parallel_axis_and_rigid_tensor_rotation() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 4., t).unwrap();
    let poly = vec![
        [0.125, 0.125],
        [0.875, 0.125],
        [0.875, 0.875],
        [0.125, 0.875],
    ];
    let body = NurbsGraphPolygonSolid::new(&source, poly.clone(), t).unwrap();
    let full = body.inertia_properties(t).unwrap();
    let split = body.split_uv_line([0.5, 0.], [0.5, 1.], t).unwrap();
    let a = split.negative.inertia_properties(t).unwrap();
    let b = split.positive.inertia_properties(t).unwrap();
    let ia = shifted(a.inertia, a.volume, a.centroid - full.centroid);
    let ib = shifted(b.inertia, b.volume, b.centroid - full.centroid);
    compare(
        std::array::from_fn(|i| std::array::from_fn(|j| ia[i][j] + ib[i][j])),
        full.inertia,
    );
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap();
    let frame = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let placed = NurbsGraphPolygonSolid::new(&source.transformed(frame, t).unwrap(), poly, t)
        .unwrap()
        .inertia_properties(t)
        .unwrap();
    let axes = rotation.axes();
    let r: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| [axes[j].x, axes[j].y, axes[j].z][i]));
    let expected = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .flat_map(|k| (0..3).map(move |l| r[i][k] * full.inertia[k][l] * r[j][l]))
                .sum()
        })
    });
    compare(placed.inertia, expected);
    assert!((placed.centroid - frame.point(full.centroid)).norm() < 1e-11);
}
#[test]
fn scaling_mutation_and_unrepresentable_inertia_are_explicit() {
    for scale in [1e-50, 1., 1e50, 1e-80, 1e100] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], 0., t).unwrap();
        let mut body =
            NurbsGraphPolygonSolid::new(&source, vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], t)
                .unwrap();
        let result = body.inertia_properties(t);
        if scale == 1e-80 || scale == 1e100 {
            assert!(result.is_err());
        } else {
            let p = result.unwrap();
            compare(
                p.inertia.map(|r| r.map(|x| x / scale.powi(5))),
                box_tensor([8., 6., 2.]),
            );
        }
        body.solid.vertices[0].point.x += t.linear / 100.;
        assert!(body.inertia_properties(t).is_err());
    }
}
