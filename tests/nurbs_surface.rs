use hagane::*;
fn near(a: Vec3, b: Vec3) {
    assert!((a - b).norm() < 1e-9, "{a:?} != {b:?}");
}
fn clamp(p: usize) -> Vec<f64> {
    let mut k = vec![0.0; p + 1];
    k.extend(vec![1.0; p + 1]);
    k
}
fn saddle() -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [clamp(1), clamp(1)],
        [2, 2],
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 20.0, 0.0),
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0, 20.0, 4.0),
        ],
        vec![1.0; 4],
    )
    .unwrap()
}
#[test]
fn bilinear_patch_and_analytic_partials() {
    let s = saddle();
    for u in [0.0, 0.2, 0.7, 1.0] {
        for v in [0.0, 0.3, 0.8, 1.0] {
            let e = s.partials(u, v).unwrap();
            near(e.point, Point3::new(10.0 * u, 20.0 * v, 4.0 * u * v));
            near(e.du, Vec3::new(10.0, 0.0, 4.0 * v));
            near(e.dv, Vec3::new(0.0, 20.0, 4.0 * u));
            let n = e.normal().unwrap();
            assert!((n.norm() - 1.0).abs() < 1e-14);
            assert!(n.dot(e.du).abs() < 1e-13 && n.dot(e.dv).abs() < 1e-13);
            assert!(n.z > 0.0);
        }
    }
}
#[test]
fn nonunit_domains_preserve_derivative_units() {
    let s = NurbsSurface::new(
        [1, 1],
        [vec![2.0, 2.0, 6.0, 6.0], vec![-3.0, -3.0, 7.0, 7.0]],
        [2, 2],
        saddle().control_points().to_vec(),
        vec![1.0; 4],
    )
    .unwrap();
    let e = s.partials(4.0, 2.0).unwrap();
    near(e.point, Point3::new(5.0, 10.0, 1.0));
    near(e.du, Vec3::new(2.5, 0.0, 0.5));
    near(e.dv, Vec3::new(0.0, 2.0, 0.2));
    assert_eq!(s.domain(), [[2.0, 6.0], [-3.0, 7.0]]);
}
#[test]
fn rational_quarter_cylinder_is_exact() {
    let mut pts = Vec::new();
    let mut weights = Vec::new();
    for (x, y, w) in [
        (2.0, 0.0, 1.0),
        (2.0, 2.0, std::f64::consts::FRAC_1_SQRT_2),
        (0.0, 2.0, 1.0),
    ] {
        for z in [0.0, 5.0] {
            pts.push(Point3::new(x, y, z));
            weights.push(w);
        }
    }
    let s = NurbsSurface::new([2, 1], [clamp(2), clamp(1)], [3, 2], pts, weights).unwrap();
    for i in 0..=20 {
        for j in 0..=10 {
            let u = i as f64 / 20.0;
            let v = j as f64 / 10.0;
            let e = s.partials(u, v).unwrap();
            assert!((e.point.x * e.point.x + e.point.y * e.point.y - 4.0).abs() < 1e-13);
            assert!((e.point.z - 5.0 * v).abs() < 1e-14);
            near(e.dv, Vec3::new(0.0, 0.0, 5.0));
            near(
                e.normal().unwrap(),
                Vec3::new(e.point.x / 2.0, e.point.y / 2.0, 0.0),
            );
        }
    }
}
#[test]
fn rational_tensor_patch_matches_independent_bernstein_sum() {
    let pts: Vec<_> = (0..3)
        .flat_map(|i| (0..3).map(move |j| Point3::new(i as f64, j as f64, (i * j) as f64)))
        .collect();
    let weights = vec![1.0, 2.0, 1.0, 0.5, 3.0, 0.75, 1.0, 1.5, 1.0];
    let s = NurbsSurface::new(
        [2, 2],
        [clamp(2), clamp(2)],
        [3, 3],
        pts.clone(),
        weights.clone(),
    )
    .unwrap();
    for u in [0.0, 0.2, 0.5, 0.8, 1.0] {
        for v in [0.0, 0.3, 0.7, 1.0] {
            let basis = |t: f64| [(1.0 - t).powi(2), 2.0 * t * (1.0 - t), t * t];
            let bu = basis(u);
            let bv = basis(v);
            let mut p = Vec3::new(0.0, 0.0, 0.0);
            let mut d = 0.0;
            for i in 0..3 {
                for j in 0..3 {
                    let w = bu[i] * bv[j] * weights[i * 3 + j];
                    p = p + pts[i * 3 + j] * w;
                    d += w;
                }
            }
            near(s.evaluate(u, v).unwrap(), p * (1.0 / d));
        }
    }
}
#[test]
fn analytic_partials_match_finite_differences() {
    let points: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| Point3::new(i as f64, j as f64, (i * i + j * j) as f64)))
        .collect();
    let s = NurbsSurface::new(
        [2, 2],
        [vec![0.0, 0.0, 0.0, 0.3, 1.0, 1.0, 1.0], clamp(2)],
        [4, 3],
        points,
        vec![1.0, 2.0, 1.0, 1.5, 3.0, 1.0, 2.0, 0.75, 1.0, 1.0, 2.0, 1.0],
    )
    .unwrap();
    for u in [0.1, 0.29, 0.3, 0.31, 0.8] {
        for v in [0.2, 0.5, 0.8] {
            let h = 1e-6;
            let e = s.partials(u, v).unwrap();
            let du = (s.evaluate(u + h, v).unwrap() - s.evaluate(u - h, v).unwrap()) * (0.5 / h);
            let dv = (s.evaluate(u, v + h).unwrap() - s.evaluate(u, v - h).unwrap()) * (0.5 / h);
            assert!((e.du - du).norm() < 1e-4);
            assert!((e.dv - dv).norm() < 1e-5);
        }
    }
}
#[test]
fn c0_crease_has_explicit_left_and_right_normals() {
    let pts = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(2.0, 0.0, 1.0),
        Point3::new(2.0, 1.0, 1.0),
    ];
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0.0, 0.0, 0.5, 1.0, 1.0], clamp(1)],
        [3, 2],
        pts,
        vec![1.0; 6],
    )
    .unwrap();
    assert!(matches!(s.partials(0.5, 0.3), Err(Error::Unsupported(_))));
    assert!(s.normal(0.5, 0.3).is_err());
    let l = s
        .evaluate_with_partials(0.5, 0.3, [KnotSide::Left, KnotSide::Right])
        .unwrap();
    let r = s
        .evaluate_with_partials(0.5, 0.3, [KnotSide::Right; 2])
        .unwrap();
    near(l.point, r.point);
    near(l.du, Vec3::new(2.0, 0.0, 0.0));
    near(r.du, Vec3::new(2.0, 0.0, 2.0));
    near(l.normal().unwrap(), Vec3::new(0.0, 0.0, 1.0));
    near(
        r.normal().unwrap(),
        Vec3::new(
            -std::f64::consts::FRAC_1_SQRT_2,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
        ),
    );
}
#[test]
fn singular_patch_has_points_but_no_invented_normals() {
    let s = NurbsSurface::new(
        [1, 1],
        [clamp(1), clamp(1)],
        [2, 2],
        vec![Point3::new(2.0, 3.0, 4.0); 4],
        vec![1.0; 4],
    )
    .unwrap();
    near(s.evaluate(0.5, 0.5).unwrap(), Point3::new(2.0, 3.0, 4.0));
    assert!(s.normal(0.5, 0.5).is_err());
    assert!(SurfaceEvaluation {
        point: Point3::new(0.0, 0.0, 0.0),
        du: Vec3::new(1.0, 0.0, 0.0),
        dv: Vec3::new(1.0, 1e-16, 0.0)
    }
    .normal()
    .is_err());
}
#[test]
fn normals_handle_tiny_and_large_regular_tangents() {
    for scale in [1e-250, 1e250] {
        let e = SurfaceEvaluation {
            point: Point3::new(0.0, 0.0, 0.0),
            du: Vec3::new(scale, 0.0, 0.0),
            dv: Vec3::new(0.0, scale, 0.0),
        };
        near(e.normal().unwrap(), Vec3::new(0.0, 0.0, 1.0));
    }
}
#[test]
fn invalid_grid_knots_weights_and_parameters_are_rejected() {
    let s = saddle();
    for counts in [[0, 2], [2, 3], [usize::MAX, 2]] {
        assert!(NurbsSurface::new(
            [1, 1],
            [clamp(1), clamp(1)],
            counts,
            s.control_points().to_vec(),
            vec![1.0; 4]
        )
        .is_err());
    }
    for weights in [
        vec![],
        vec![1.0, 0.0, 1.0, 1.0],
        vec![1.0, f64::NAN, 1.0, 1.0],
    ] {
        assert!(NurbsSurface::new(
            [1, 1],
            [clamp(1), clamp(1)],
            [2, 2],
            s.control_points().to_vec(),
            weights
        )
        .is_err());
    }
    assert!(NurbsSurface::new(
        [1, 0],
        [clamp(1), clamp(1)],
        [2, 2],
        s.control_points().to_vec(),
        vec![1.0; 4]
    )
    .is_err());
    assert!(NurbsSurface::new(
        [1, 1],
        [clamp(1), vec![0.0, 0.1, 0.9, 1.0]],
        [2, 2],
        s.control_points().to_vec(),
        vec![1.0; 4]
    )
    .is_err());
    for (u, v) in [
        (f64::NAN, 0.5),
        (0.5, f64::INFINITY),
        (-1e-12, 0.5),
        (0.5, 1.0 + 1e-12),
    ] {
        assert!(s.evaluate(u, v).is_err());
        assert!(s.partials(u, v).is_err());
    }
    assert!(s.knots(2).is_err());
}
#[test]
fn common_surface_weight_scaling_is_invariant() {
    let s = saddle();
    for scale in [1e-200, 1e200] {
        let c = NurbsSurface::new(
            s.degrees(),
            [s.knots(0).unwrap().to_vec(), s.knots(1).unwrap().to_vec()],
            s.control_counts(),
            s.control_points().to_vec(),
            s.weights().iter().map(|w| w * scale).collect(),
        )
        .unwrap();
        let a = s.partials(0.4, 0.7).unwrap();
        let b = c.partials(0.4, 0.7).unwrap();
        near(a.point, b.point);
        near(a.du, b.du);
        near(a.dv, b.dv);
    }
}

#[test]
fn display_grid_is_an_open_patch_with_consistent_winding() {
    let s = saddle();
    let mesh = s.sample_grid([3, 5]).unwrap();
    assert_eq!(mesh.triangles.len(), 30);
    assert_eq!(mesh.positions.len(), 90);
    assert!(mesh.face_ids.iter().all(|i| *i == 0));
    for (tri, normals) in mesh.triangles.iter().zip(mesh.normals.as_chunks::<3>().0) {
        let p = tri.map(|i| mesh.positions[i]);
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        for n in normals {
            assert!(cross.dot(*n) > 0.0);
            assert!((n.norm() - 1.0).abs() < 1e-14);
        }
    }
}
#[test]
fn display_sampling_rejects_bad_resolution_and_unsplit_c0_lines() {
    for cells in [[0, 1], [1, 0], [257, 1]] {
        assert!(saddle().sample_grid(cells).is_err());
    }
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0.0, 0.0, 0.5, 1.0, 1.0], clamp(1)],
        [3, 2],
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 0.0, 1.0),
            Point3::new(2.0, 1.0, 1.0),
        ],
        vec![1.0; 6],
    )
    .unwrap();
    assert!(matches!(s.sample_grid([4, 4]), Err(Error::Unsupported(_))));
}
#[test]
fn v_axis_crease_side_selection() {
    let s = NurbsSurface::new(
        [1, 1],
        [clamp(1), vec![0.0, 0.0, 0.5, 1.0, 1.0]],
        [2, 3],
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 2.0, 1.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 1.0),
        ],
        vec![1.0; 6],
    )
    .unwrap();
    assert!(s.partials(0.3, 0.5).is_err());
    let left = s
        .evaluate_with_partials(0.3, 0.5, [KnotSide::Right, KnotSide::Left])
        .unwrap();
    let right = s
        .evaluate_with_partials(0.3, 0.5, [KnotSide::Right; 2])
        .unwrap();
    near(left.point, right.point);
    near(left.dv, Vec3::new(0.0, 2.0, 0.0));
    near(right.dv, Vec3::new(0.0, 2.0, 2.0));
    near(
        right.normal().unwrap(),
        Vec3::new(
            0.0,
            -std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ),
    );
}
