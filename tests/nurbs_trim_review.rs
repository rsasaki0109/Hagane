use hagane::{KnotSide, NurbsFace, NurbsSurface, Point3, Tolerance};

fn source(scale: f64) -> NurbsSurface {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..5 {
        for j in 0..5 {
            points.push(Point3::new(i as f64, j as f64, (i * j + i * i) as f64 * 0.15) * scale);
            weights.push((1. + ((i * 7 + j * 3) % 11) as f64 * 0.12) * 1e200);
        }
    }
    NurbsSurface::new(
        [2, 2],
        [
            vec![2., 2., 2., 4., 4., 6., 6., 6.],
            vec![-3., -3., -3., 0., 1., 5., 5., 5.],
        ],
        [5, 5],
        points,
        weights,
    )
    .unwrap()
}

#[test]
fn restriction_retains_nonunit_parameters_and_source_derivative_limits() {
    for scale in [1., 1e-100] {
        let original = source(scale);
        for ranges in [
            [[3., 4.], [-1., 2.]],
            [[4., 5.5], [0., 5.]],
            [[2., 6.], [-3., 5.]],
        ] {
            let trimmed = original.restricted(ranges).unwrap();
            assert_eq!(trimmed.domain(), ranges);
            for i in 0..=20 {
                for j in 0..=20 {
                    let uv = [
                        ranges[0][0] + (ranges[0][1] - ranges[0][0]) * i as f64 / 20.,
                        ranges[1][0] + (ranges[1][1] - ranges[1][0]) * j as f64 / 20.,
                    ];
                    for side in [KnotSide::Left, KnotSide::Right] {
                        let mut sides = [side; 2];
                        for axis in 0..2 {
                            if uv[axis] == ranges[axis][0] {
                                sides[axis] = KnotSide::Right;
                            }
                            if uv[axis] == ranges[axis][1] {
                                sides[axis] = KnotSide::Left;
                            }
                        }
                        let a = original
                            .evaluate_with_partials(uv[0], uv[1], sides)
                            .unwrap();
                        let b = trimmed.evaluate_with_partials(uv[0], uv[1], sides).unwrap();
                        assert!((a.point - b.point).norm() < scale * 3e-12);
                        assert!((a.du - b.du).norm() < scale * 3e-12);
                        assert!((a.dv - b.dv).norm() < scale * 3e-12);
                    }
                }
            }
        }
    }
}

#[test]
fn trimmed_open_face_bounds_actual_source_and_keeps_negative_orientation() {
    let t = Tolerance::default();
    let original = source(1.);
    let face = NurbsFace::new(original.clone(), -1, t).unwrap();
    let ranges = [[3., 5.], [-1., 3.]];
    let trimmed = face.trimmed(ranges, t).unwrap();
    trimmed.validate_boundary(t).unwrap();
    assert_eq!(trimmed.face.orientation, -1);
    let mesh = trimmed.tessellate_bounded(0.06, 4096, t).unwrap();
    for (i, uv) in mesh.vertex_uv.iter().enumerate() {
        assert!((mesh.mesh.positions[i] - original.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-12);
    }
    // Negative face orientation reverses triangles, so recover UV corner order.
    for (cell, range) in mesh.uv_ranges.iter().enumerate() {
        let first = mesh.mesh.triangles[cell * 2];
        let second = mesh.mesh.triangles[cell * 2 + 1];
        let corners = [
            mesh.mesh.positions[first[0]],
            mesh.mesh.positions[first[2]],
            mesh.mesh.positions[first[1]],
            mesh.mesh.positions[second[1]],
        ];
        for i in 0..=10 {
            for j in 0..=10 {
                let s = i as f64 / 10.;
                let v = j as f64 / 10.;
                let uv = [
                    range[0][0] + (range[0][1] - range[0][0]) * s,
                    range[1][0] + (range[1][1] - range[1][0]) * v,
                ];
                let p = if v <= s {
                    corners[0] * (1. - s) + corners[1] * (s - v) + corners[2] * v
                } else {
                    corners[0] * (1. - v) + corners[2] * s + corners[3] * (v - s)
                };
                assert!(
                    (original.evaluate(uv[0], uv[1]).unwrap() - p).norm()
                        <= mesh.error_bounds[cell] + 1e-11
                );
            }
        }
    }
}

#[test]
fn narrow_far_parameter_restriction_preserves_original_evaluation() {
    let a = 1e16;
    let b = a + 6.;
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(i as f64, j as f64, (i * j) as f64 * 0.2));
            weights.push(1. + (i * 3 + j) as f64 * 0.11);
        }
    }
    let original = NurbsSurface::new(
        [2, 2],
        [vec![a, a, a, b, b, b], vec![2., 2., 2., 6., 6., 6.]],
        [3, 3],
        points,
        weights,
    )
    .unwrap();
    let restricted = original.restricted([[a + 2., a + 4.], [3., 5.]]).unwrap();
    assert_eq!(restricted.domain(), [[a + 2., a + 4.], [3., 5.]]);
    for u in [a + 2., a + 4.] {
        for i in 0..=20 {
            let v = 3. + i as f64 / 10.;
            let a = original
                .evaluate_with_partials(u, v, [KnotSide::Right; 2])
                .unwrap();
            let b = restricted
                .evaluate_with_partials(u, v, [KnotSide::Right; 2])
                .unwrap();
            assert!((a.point - b.point).norm() < 2e-13);
            assert!((a.du - b.du).norm() < 2e-13);
            assert!((a.dv - b.dv).norm() < 2e-13);
        }
    }
}
