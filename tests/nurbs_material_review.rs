use hagane::{NurbsHoledFace, NurbsSurface, Point3, Surface, Tolerance};

fn isolated_singularity() -> NurbsSurface {
    let square = [0.25, -0.25, 0.25];
    let cubic = [-0.375, 0.375, -0.375, 0.375];
    let linear = [-1.5, -0.5, 0.5, 1.5];
    let points = (0..3)
        .flat_map(|i| {
            (0..4).map(move |j| Point3::new(i as f64 / 2., cubic[j] + square[i] * linear[j], 0.))
        })
        .collect();
    NurbsSurface::new(
        [2, 3],
        [
            vec![0., 0., 0., 1., 1., 1.],
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
        ],
        [3, 4],
        points,
        vec![1.; 12],
    )
    .unwrap()
}

fn plane(with_spans: bool) -> NurbsSurface {
    let axis = if with_spans {
        vec![0., 0., 0.5, 1., 1.]
    } else {
        vec![0., 0., 1., 1.]
    };
    let count = if with_spans { 3 } else { 2 };
    let points = (0..count)
        .flat_map(|i| {
            (0..count).map(move |j| {
                Point3::new(
                    i as f64 / (count - 1) as f64,
                    j as f64 / (count - 1) as f64,
                    0.,
                )
            })
        })
        .collect();
    NurbsSurface::new(
        [1, 1],
        [axis.clone(), axis],
        [count, count],
        points,
        vec![1.; count * count],
    )
    .unwrap()
}

#[test]
fn injective_polynomial_material_matches_analytic_geometry() {
    let source = isolated_singularity();
    assert!(source.normal(0.5, 0.5).is_err());
    assert!(source.tessellate_bounded(0.001, 4096).is_err());
    let t = Tolerance::default();
    let face = NurbsHoledFace::new(
        source.clone(),
        [[0., 1.], [0., 1.]],
        vec![[[0.25, 0.75], [0.25, 0.75]]],
        -1,
        t,
    )
    .unwrap();
    let Surface::Nurbs(retained) = &face.face.surface else {
        panic!()
    };
    // Refinement rounding can perturb the excluded tangent; geometry is tested here.
    assert_eq!(retained.domain(), [[0., 1.], [0., 1.]]);
    let result = face.tessellate_bounded(0.005, 4096, t).unwrap();
    for (i, uv) in result.vertex_uv.iter().enumerate() {
        assert!(!(uv[0] > 0.25 && uv[0] < 0.75 && uv[1] > 0.25 && uv[1] < 0.75));
        let actual = source.evaluate(uv[0], uv[1]).unwrap();
        assert!((actual - result.mesh.positions[i]).norm() < 1e-12);
        assert!((result.mesh.normals[i] + Point3::new(0., 0., 1.)).norm() < 1e-12);
    }
    for (cell, r) in result.uv_ranges.iter().enumerate() {
        assert!(r[0][1] <= 0.25 || r[0][0] >= 0.75 || r[1][1] <= 0.25 || r[1][0] >= 0.75);
        let a = result.mesh.triangles[cell * 2];
        let b = result.mesh.triangles[cell * 2 + 1];
        let c = [
            result.mesh.positions[a[0]],
            result.mesh.positions[a[2]],
            result.mesh.positions[a[1]],
            result.mesh.positions[b[1]],
        ];
        for i in 0..=12 {
            for j in 0..=12 {
                let u = i as f64 / 12.;
                let v = j as f64 / 12.;
                let uv = [
                    r[0][0] + (r[0][1] - r[0][0]) * u,
                    r[1][0] + (r[1][1] - r[1][0]) * v,
                ];
                let actual = source.evaluate(uv[0], uv[1]).unwrap();
                let expected = if v <= u {
                    c[0] * (1. - u) + c[1] * (u - v) + c[2] * v
                } else {
                    c[0] * (1. - v) + c[2] * u + c[3] * (v - u)
                };
                assert!((actual - expected).norm() <= result.error_bounds[cell] + 1e-12);
            }
        }
    }
    let unremoved = NurbsHoledFace::new(source, [[0., 1.], [0., 1.]], vec![], 1, t).unwrap();
    assert!(unremoved.tessellate_bounded(0.0001, 4096, t).is_err());
}

#[test]
fn retained_cells_determine_budget_and_empty_holes_keep_untrimmed_behavior() {
    let t = Tolerance::default();
    let face = NurbsHoledFace::new(
        plane(true),
        [[0., 1.], [0., 1.]],
        vec![[[0.2, 0.8], [0.2, 0.8]]],
        1,
        t,
    )
    .unwrap();
    let out = face.tessellate_bounded(0.01, 12, t).unwrap();
    assert_eq!(out.uv_ranges.len(), 12); // Full aligned source has sixteen cells.
    assert!(face.tessellate_bounded(0.01, 11, t).is_err());
    let area: f64 = out
        .uv_ranges
        .iter()
        .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
        .sum();
    assert!((area - 0.64).abs() < 1e-12);
    let source = plane(false);
    let ordinary = source.tessellate_bounded(0.01, 1).unwrap();
    let face = NurbsHoledFace::new(source, [[0., 1.], [0., 1.]], vec![], 1, t).unwrap();
    let holed = face.tessellate_bounded(0.01, 1, t).unwrap();
    assert_eq!(ordinary.mesh.positions, holed.mesh.positions);
    assert_eq!(ordinary.mesh.normals, holed.mesh.normals);
    assert_eq!(ordinary.mesh.triangles, holed.mesh.triangles);
    assert_eq!(ordinary.vertex_uv, holed.vertex_uv);
    assert_eq!(ordinary.vertex_nodes, holed.vertex_nodes);
    assert_eq!(ordinary.uv_ranges, holed.uv_ranges);
    assert_eq!(ordinary.error_bounds, holed.error_bounds);
}

#[test]
fn true_retained_singularity_inside_hole_is_not_sampled_but_material_is_checked() {
    let square = [0.25, -0.25, 0.25];
    let points = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                Point3::new(
                    square[i] - square[j],
                    2. * (i as f64 / 2. - 0.5) * (j as f64 / 2. - 0.5),
                    0.,
                )
            })
        })
        .collect();
    let k = vec![0., 0., 0., 1., 1., 1.];
    let source = NurbsSurface::new([2, 2], [k.clone(), k], [3, 3], points, vec![1.; 9])
        .unwrap()
        .insert_knot(0, 0.5, 1)
        .unwrap()
        .insert_knot(1, 0.5, 1)
        .unwrap();
    let t = Tolerance::default();
    assert!(source.normal(0.5, 0.5).is_err());
    let removed = NurbsHoledFace::new(
        source.clone(),
        [[0., 1.], [0., 1.]],
        vec![[[0.25, 0.75], [0.25, 0.75]]],
        1,
        t,
    )
    .unwrap();
    let Surface::Nurbs(retained) = &removed.face.surface else {
        panic!()
    };
    assert!(retained.normal(0.5, 0.5).is_err());
    assert!(retained.tessellate_bounded(0.01, 4096).is_err());
    let mesh = removed.tessellate_bounded(0.01, 4096, t).unwrap();
    assert!(mesh
        .vertex_uv
        .iter()
        .all(|p| !(p[0] > 0.25 && p[0] < 0.75 && p[1] > 0.25 && p[1] < 0.75)));
    let visible = NurbsHoledFace::new(
        source,
        [[0., 1.], [0., 1.]],
        vec![[[0.125, 0.25], [0.125, 0.25]]],
        1,
        t,
    )
    .unwrap();
    let Surface::Nurbs(retained) = &visible.face.surface else {
        panic!()
    };
    assert!(retained.normal(0.5, 0.5).is_err());
    assert!(visible.tessellate_bounded(0.01, 4096, t).is_err());
}
