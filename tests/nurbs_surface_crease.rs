use hagane::{KnotSide, NurbsSurface, NurbsSurfaceMesh, Point3};
fn roof(height: f64, weight: f64) -> NurbsSurface {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (x, z, w) in [(-40., 0., 1.), (0., height, weight), (40., 0., 1.)] {
        for y in [-30., 30.] {
            points.push(Point3::new(x, y, z));
            weights.push(w);
        }
    }
    NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        points,
        weights,
    )
    .unwrap()
}
fn geometric_topology(mesh: &NurbsSurfaceMesh) {
    assert_eq!(mesh.vertex_uv.len(), mesh.mesh.positions.len());
    assert_eq!(mesh.vertex_nodes.len(), mesh.mesh.positions.len());
    assert_eq!(mesh.normal_sides.len(), mesh.mesh.positions.len());
    let mut nodes = std::collections::BTreeMap::new();
    for (i, node) in mesh.vertex_nodes.iter().enumerate() {
        if let Some(j) = nodes.insert(*node, i) {
            assert_eq!(mesh.vertex_uv[i], mesh.vertex_uv[j]);
            assert_eq!(
                mesh.mesh.positions[i].x.to_bits(),
                mesh.mesh.positions[j].x.to_bits()
            );
            assert_eq!(
                mesh.mesh.positions[i].y.to_bits(),
                mesh.mesh.positions[j].y.to_bits()
            );
            assert_eq!(
                mesh.mesh.positions[i].z.to_bits(),
                mesh.mesh.positions[j].z.to_bits()
            );
        }
    }
    let mut edges = std::collections::BTreeMap::new();
    for tri in &mesh.mesh.triangles {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let a = mesh.vertex_nodes[a];
            let b = mesh.vertex_nodes[b];
            assert_ne!(a, b);
            let key = if a < b { (a, b) } else { (b, a) };
            *edges.entry(key).or_insert(0) += 1;
        }
    }
    assert!(edges.values().all(|n| *n == 1 || *n == 2));
    let mut us = mesh.vertex_uv.iter().map(|uv| uv[0]).collect::<Vec<_>>();
    let mut vs = mesh.vertex_uv.iter().map(|uv| uv[1]).collect::<Vec<_>>();
    us.sort_by(f64::total_cmp);
    vs.sort_by(f64::total_cmp);
    us.dedup();
    vs.dedup();
    assert_eq!(nodes.len(), us.len() * vs.len());
    assert_eq!(
        edges.values().filter(|n| **n == 1).count(),
        2 * (us.len() + vs.len() - 2)
    );
}
fn dense_bounds(surface: &NurbsSurface, mesh: &NurbsSurfaceMesh, error: f64) {
    for (cell, [[a, b], [c, d]]) in mesh.uv_ranges.iter().copied().enumerate() {
        let t0 = mesh.mesh.triangles[cell * 2];
        let t1 = mesh.mesh.triangles[cell * 2 + 1];
        let corners = [
            mesh.mesh.positions[t0[0]],
            mesh.mesh.positions[t0[1]],
            mesh.mesh.positions[t0[2]],
            mesh.mesh.positions[t1[2]],
        ];
        assert!(mesh.error_bounds[cell] <= error);
        for i in 0..=8 {
            for j in 0..=8 {
                let s = i as f64 / 8.;
                let t = j as f64 / 8.;
                let point = surface.evaluate(a + (b - a) * s, c + (d - c) * t).unwrap();
                let triangle = if t <= s {
                    corners[0] * (1. - s) + corners[1] * (s - t) + corners[2] * t
                } else {
                    corners[0] * (1. - t) + corners[2] * s + corners[3] * (t - s)
                };
                assert!((point - triangle).norm() <= mesh.error_bounds[cell] + 1e-12);
            }
        }
    }
}
#[test]
fn roof_crease_has_exact_coincident_positions_and_distinct_analytic_normals() {
    for weight in [1., 2.5] {
        let surface = roof(18., weight);
        assert!(surface.partials(0.5, 0.4).is_err());
        let mesh = surface.tessellate_bounded(0.1, 65536).unwrap();
        geometric_topology(&mesh);
        dense_bounds(&surface, &mesh, 0.1);
        let left = Point3::new(-18. / 40., 0., 1.).normalized().unwrap();
        let right = Point3::new(18. / 40., 0., 1.).normalized().unwrap();
        let mut crease_count = 0;
        for (i, [u, _v]) in mesh.vertex_uv.iter().copied().enumerate() {
            assert!(mesh.mesh.normals[i].finite());
            if u == 0.5 {
                crease_count += 1;
                let expected = if mesh.normal_sides[i][0] == KnotSide::Left {
                    left
                } else {
                    right
                };
                assert!((mesh.mesh.normals[i] - expected).norm() < 1e-12);
            }
        }
        assert!(crease_count >= 4);
    }
}
#[test]
fn crossing_c0_lines_retain_all_four_one_sided_normal_pairs() {
    let k = vec![0., 0., 0.5, 1., 1.];
    let mut points = Vec::new();
    for (x, zx) in [(-40., 0.), (0., 18.), (40., 0.)] {
        for (y, zy) in [(-30., 0.), (0., 12.), (30., 0.)] {
            points.push(Point3::new(x, y, zx + zy));
        }
    }
    let surface = NurbsSurface::new([1, 1], [k.clone(), k], [3, 3], points, vec![1.; 9]).unwrap();
    let mesh = surface.tessellate_bounded(0.01, 64).unwrap();
    geometric_topology(&mesh);
    dense_bounds(&surface, &mesh, 0.01);
    let ids = mesh
        .vertex_uv
        .iter()
        .enumerate()
        .filter(|(_, uv)| **uv == [0.5, 0.5])
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 4);
    let node = mesh.vertex_nodes[ids[0]];
    let mut side_pairs = std::collections::BTreeSet::new();
    for i in ids {
        assert_eq!(mesh.vertex_nodes[i], node);
        let sides = mesh.normal_sides[i];
        side_pairs.insert((sides[0] == KnotSide::Left, sides[1] == KnotSide::Left));
        let expected = Point3::new(
            if sides[0] == KnotSide::Left {
                -18. / 40.
            } else {
                18. / 40.
            },
            if sides[1] == KnotSide::Left {
                -12. / 30.
            } else {
                12. / 30.
            },
            1.,
        )
        .normalized()
        .unwrap();
        assert!((mesh.mesh.normals[i] - expected).norm() < 1e-12);
    }
    assert_eq!(side_pairs.len(), 4);
}
#[test]
fn nominal_c0_smooth_refinement_succeeds_but_singular_side_fails() {
    let surface = NurbsSurface::new(
        [2, 1],
        [vec![0., 0., 0., 1., 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(0.5, 0., 0.),
            Point3::new(0.5, 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
        ],
        vec![1.; 6],
    )
    .unwrap()
    .insert_knot(0, 0.5, 2)
    .unwrap();
    let mesh = surface.tessellate_bounded(0.01, 16).unwrap();
    geometric_topology(&mesh);
    for normal in &mesh.mesh.normals {
        assert_eq!(*normal, Point3::new(0., 0., 1.));
    }
    let singular = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
        ],
        vec![1.; 6],
    )
    .unwrap();
    assert!(singular.tessellate_bounded(0.1, 64).is_err());
}
#[test]
fn coincident_positions_at_distinct_uv_are_not_geometric_node_welded() {
    let profile = [
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (-1., 1.),
        (-1., 0.),
        (-1., -1.),
        (0., -1.),
        (1., -1.),
        (1., 0.),
    ];
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (i, (x, y)) in profile.into_iter().enumerate() {
        for z in [0., 1.] {
            points.push(Point3::new(x, y, z));
            weights.push(if i % 2 == 0 {
                1.
            } else {
                std::f64::consts::FRAC_1_SQRT_2
            });
        }
    }
    let surface = NurbsSurface::new(
        [2, 1],
        [
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            vec![0., 0., 1., 1.],
        ],
        [9, 2],
        points,
        weights,
    )
    .unwrap();
    let mesh = surface.tessellate_bounded(0.1, 4096).unwrap();
    geometric_topology(&mesh);
    let start = mesh
        .vertex_uv
        .iter()
        .position(|uv| *uv == [0., 0.])
        .unwrap();
    let end = mesh
        .vertex_uv
        .iter()
        .position(|uv| *uv == [4., 0.])
        .unwrap();
    assert_eq!(mesh.mesh.positions[start], mesh.mesh.positions[end]);
    assert_ne!(mesh.vertex_nodes[start], mesh.vertex_nodes[end]);
}
#[test]
fn mixed_signed_zero_knots_share_one_geometric_crease() {
    let mut points = Vec::new();
    for (x, z) in [(-1., 0.), (-0.5, 0.5), (0., 1.), (0.5, 0.5), (1., 0.)] {
        for y in [0., 1.] {
            points.push(Point3::new(x, y, z));
        }
    }
    let surface = NurbsSurface::new(
        [2, 1],
        [
            vec![-1., -1., -1., -0.0, 0.0, 1., 1., 1.],
            vec![0., 0., 1., 1.],
        ],
        [5, 2],
        points,
        vec![1.; 10],
    )
    .unwrap();
    let mesh = surface.tessellate_bounded(0.01, 16).unwrap();
    assert_eq!(mesh.uv_ranges.len(), 2);
    geometric_topology(&mesh);
    let nodes = mesh
        .vertex_nodes
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(nodes.len(), 6);
    let crease = mesh
        .vertex_uv
        .iter()
        .enumerate()
        .filter(|(_, uv)| uv[0] == 0.)
        .collect::<Vec<_>>();
    assert_eq!(crease.len(), 4);
    for (i, uv) in crease {
        assert_eq!(uv[0].to_bits(), 0f64.to_bits());
        let expected = Point3::new(
            if mesh.normal_sides[i][0] == KnotSide::Left {
                -1.
            } else {
                1.
            },
            0.,
            1.,
        )
        .normalized()
        .unwrap();
        assert!((mesh.mesh.normals[i] - expected).norm() < 1e-12);
    }
}
