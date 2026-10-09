use hagane::*;
use std::collections::HashMap;

fn roof() -> NurbsSurface {
    let points = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                Point3::new(
                    i as f64 * 5.,
                    j as f64 * 5.,
                    if i == 1 { 4. } else { 0. } + if j == 1 { 2. } else { 0. },
                )
            })
        })
        .collect();
    let knots = vec![0., 0., 0.5, 1., 1.];
    NurbsSurface::new([1, 1], [knots.clone(), knots], [3, 3], points, vec![1.; 9]).unwrap()
}
fn outer() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
fn area(display: &NurbsPolygonMesh) -> f64 {
    display
        .mesh
        .triangles
        .iter()
        .map(|ids| {
            let [a, b, c] = ids.map(|id| display.vertex_uv[id]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.
        })
        .sum()
}
#[test]
fn central_hole_removes_crossing_and_keeps_material_crease_normals() {
    let face = NurbsPolygonHoledFace::new(
        roof(),
        outer(),
        vec![[[0.4, 0.6], [0.4, 0.6]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let display = face
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    assert!((area(&display) - 0.96).abs() < 1e-12);
    let mut nodes: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, uv) in display.vertex_uv.iter().enumerate() {
        assert!(!(uv[0] > 0.4 && uv[0] < 0.6 && uv[1] > 0.4 && uv[1] < 0.6));
        assert_ne!(*uv, [0.5, 0.5]);
        nodes.entry(display.vertex_nodes[i]).or_default().push(i);
    }
    let mut split = 0;
    for ids in nodes.values() {
        if ids.len() > 1 {
            split += 1;
            for &i in ids {
                assert_eq!(display.mesh.positions[i], display.mesh.positions[ids[0]]);
            }
            assert!(ids
                .iter()
                .any(|i| display.mesh.normals[*i] != display.mesh.normals[ids[0]]));
        }
    }
    assert!(split >= 4);
    for triangle in &display.mesh.triangles {
        for axis in 0..2 {
            assert!(
                !(triangle.iter().any(|i| display.vertex_uv[*i][axis] < 0.5)
                    && triangle.iter().any(|i| display.vertex_uv[*i][axis] > 0.5))
            );
        }
    }
}
#[test]
fn separated_holes_preserve_uv_area_and_reverse_orientation() {
    let holes = vec![
        [[0.125, 0.25], [0.125, 0.375]],
        [[0.625, 0.875], [0.625, 0.875]],
    ];
    for orientation in [1, -1] {
        let face = NurbsPolygonHoledFace::new(
            roof(),
            outer(),
            holes.clone(),
            orientation,
            Tolerance::default(),
        )
        .unwrap();
        let display = face
            .tessellate_bounded(0.1, 65536, Tolerance::default())
            .unwrap();
        assert!((area(&display) - orientation as f64 * 0.90625).abs() < 1e-12);
        assert!(display.error_bounds.iter().all(|b| *b <= 0.1));
    }
}
#[test]
fn hole_line_clipping_respects_intermediate_triangle_budget() {
    let face = NurbsPolygonHoledFace::new(
        roof(),
        outer(),
        vec![[[0.4, 0.6], [0.4, 0.6]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    assert!(face
        .tessellate_bounded(0.1, 3, Tolerance::default())
        .is_err());
    assert!(face
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .is_ok());
}

#[test]
fn separated_decimal_holes_preserve_material_area() {
    let holes = vec![[[0.1, 0.2], [0.1, 0.3]], [[0.7, 0.9], [0.65, 0.8]]];
    let face = NurbsPolygonHoledFace::new(roof(), outer(), holes, 1, Tolerance::default()).unwrap();
    let display = face
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    assert!((area(&display) - 0.95).abs() < 1e-12);
    assert!(display.error_bounds.iter().all(|bound| *bound <= 0.1));
}

#[test]
fn excluded_isolated_singularity_is_not_evaluated_for_display_normals() {
    let coordinates = [0.25, -0.25, 0.25];
    let linear = [-0.5, 0., 0.5];
    let controls = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| Point3::new(coordinates[i], coordinates[j], linear[i] * linear[j]))
        })
        .collect();
    let knots = vec![0., 0., 0., 1., 1., 1.];
    let surface = NurbsSurface::new(
        [2, 2],
        [knots.clone(), knots],
        [3, 3],
        controls,
        vec![1.; 9],
    )
    .unwrap();
    assert!(
        surface
            .evaluate_with_partials(0.5, 0.5, [KnotSide::Right; 2])
            .unwrap()
            .du
            .cross(
                surface
                    .evaluate_with_partials(0.5, 0.5, [KnotSide::Right; 2])
                    .unwrap()
                    .dv
            )
            .norm()
            == 0.
    );
    let face = NurbsPolygonHoledFace::new(
        surface,
        outer(),
        vec![[[0.375, 0.625], [0.375, 0.625]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let display = face
        .tessellate_bounded(0.005, 65536, Tolerance::default())
        .unwrap();
    assert!(display
        .vertex_uv
        .iter()
        .all(|uv| !(uv[0] > 0.375 && uv[0] < 0.625 && uv[1] > 0.375 && uv[1] < 0.625)));
    assert!((area(&display) - 0.9375).abs() < 1e-12);
    assert!(display
        .mesh
        .normals
        .iter()
        .all(|n| (n.norm() - 1.).abs() < 1e-12));
}

#[test]
fn hole_side_on_c0_crease_has_resolved_retained_triangles() {
    let face = NurbsPolygonHoledFace::new(
        roof(),
        outer(),
        vec![[[0.5, 0.7], [0.4, 0.6]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let display = face
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    assert!((area(&display) - 0.96).abs() < 1e-12);
    assert!(display.vertex_uv.contains(&[0.5, 0.5]));
    assert!(display
        .vertex_uv
        .iter()
        .all(|uv| !(uv[0] > 0.5 && uv[0] < 0.7 && uv[1] > 0.4 && uv[1] < 0.6)));
}
