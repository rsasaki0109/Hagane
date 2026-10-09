use hagane::{KnotSide, NurbsHoledFace, NurbsSurface, Point3, Surface, Tolerance};
use std::collections::BTreeMap;

fn roof() -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [vec![-2., -2., 1., 5., 5.], vec![3., 3., 9., 9.]],
        [3, 2],
        vec![
            Point3::new(-2., 0., 0.),
            Point3::new(-2., 2., 0.),
            Point3::new(0., 0., 2.),
            Point3::new(0., 2., 2.),
            Point3::new(4., 0., 0.),
            Point3::new(4., 2., 0.),
        ],
        vec![1.; 6],
    )
    .unwrap()
}

#[test]
fn rectangle_holes_reject_contacts_and_use_uv_not_length_tolerance() {
    let outer = [[-2., 5.], [3., 9.]];
    let valid = [[-0.2, 2.], [4., 5.]];
    for holes in [
        vec![[[-2., 0.], [4., 5.]]],
        vec![[[-2. + 1e-14, 0.], [4., 5.]]],
        vec![[[-3., 0.], [4., 5.]]],
        vec![[[1., 1.], [4., 5.]]],
        vec![[[2., 1.], [4., 5.]]],
        vec![[[f64::NAN, 2.], [4., 5.]]],
        vec![valid, [[2., 3.], [4., 5.]]],
        vec![valid, [[2. + 1e-14, 3.], [4., 5.]]],
        vec![valid, [[1., 3.], [4.5, 6.]]],
        vec![valid; 17],
    ] {
        assert!(NurbsHoledFace::new(roof(), outer, holes, 1, Tolerance::default()).is_err());
    }
    // Physical boundary matching tolerance does not become a UV clearance threshold.
    let tolerance = Tolerance::new(1e-3).unwrap();
    let face = NurbsHoledFace::new(
        roof(),
        outer,
        vec![[[-2. + 1e-6, 0.], [4., 5.]]],
        1,
        tolerance,
    )
    .unwrap();
    face.validate_boundary(tolerance).unwrap();
}

#[test]
fn exact_clockwise_hole_boundaries_match_retained_surface() {
    let t = Tolerance::default();
    let holes = vec![[[-0.2, 2.], [4., 5.]], [[3., 4.], [6., 8.]]];
    let part = NurbsHoledFace::new(roof(), [[-2., 5.], [3., 9.]], holes, -1, t).unwrap();
    part.validate_boundary(t).unwrap();
    let Surface::Nurbs(surface) = &part.face.surface else {
        panic!()
    };
    assert_eq!(part.face.orientation, -1);
    assert_eq!(part.face.wires.len(), 3);
    assert_eq!(part.vertices.len(), 12);
    assert_eq!(part.edges.len(), 12);
    for (wi, wire) in part.face.wires.iter().enumerate() {
        let mut area = 0.;
        for (ci, coedge) in wire.coedges.iter().enumerate() {
            let edge = &part.edges[coedge.edge];
            let [a, b] = edge.curve.range();
            let start = if coedge.forward { a } else { b };
            let end = if coedge.forward { b } else { a };
            let p = coedge.pcurve.evaluate(start);
            let q = coedge.pcurve.evaluate(end);
            area += p[0] * q[1] - p[1] * q[0];
            let next = &wire.coedges[(ci + 1) % wire.coedges.len()];
            let next_edge = &part.edges[next.edge];
            let tail = edge.vertices[usize::from(coedge.forward)];
            let head = next_edge.vertices[usize::from(!next.forward)];
            assert_eq!(tail, head);
            for k in 0..=20 {
                let parameter = a + (b - a) * k as f64 / 20.;
                let uv = coedge.pcurve.evaluate(parameter);
                assert!(
                    (surface.evaluate(uv[0], uv[1]).unwrap()
                        - edge.curve.try_evaluate(parameter).unwrap())
                    .norm()
                        < 1e-12
                );
            }
        }
        assert!(if wi == 0 { area > 0. } else { area < 0. });
    }
}

#[test]
fn holed_c0_mesh_welds_and_covers_only_exact_material_domain() {
    let t = Tolerance::default();
    let original = roof();
    let holes = vec![[[-0.2, 2.], [4., 5.]], [[3., 4.], [6., 8.]]];
    let face = NurbsHoledFace::new(
        original.clone(),
        [[-2., 5.], [3., 9.]],
        holes.clone(),
        -1,
        t,
    )
    .unwrap();
    let out = face.tessellate_bounded(0.05, 4096, t).unwrap();
    assert_eq!(out.mesh.positions.len(), out.vertex_uv.len());
    assert_eq!(out.mesh.positions.len(), out.vertex_nodes.len());
    assert_eq!(out.mesh.positions.len(), out.normal_sides.len());
    let mut positions = BTreeMap::new();
    for (i, &node) in out.vertex_nodes.iter().enumerate() {
        let p = out.mesh.positions[i];
        let bits = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
        if let Some(existing) = positions.insert(node, bits) {
            assert_eq!(existing, bits);
        }
        let uv = out.vertex_uv[i];
        let e = original
            .evaluate_with_partials(uv[0], uv[1], out.normal_sides[i])
            .unwrap();
        assert!((p - e.point).norm() < 1e-12);
        assert!((out.mesh.normals[i] + e.normal().unwrap()).norm() < 1e-12);
    }
    let mut incidence = BTreeMap::new();
    for tri in &out.mesh.triangles {
        for i in 0..3 {
            let a = out.vertex_nodes[tri[i]];
            let b = out.vertex_nodes[tri[(i + 1) % 3]];
            let key = if a < b { (a, b) } else { (b, a) };
            let entry = incidence
                .entry(key)
                .or_insert((0usize, 0i32, tri[i], tri[(i + 1) % 3]));
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    for (count, balance, a, b) in incidence.values() {
        assert!(*count == 1 || *count == 2);
        if *count == 2 {
            assert_eq!(*balance, 0);
            continue;
        }
        let uv = out.vertex_uv[*a];
        let vv = out.vertex_uv[*b];
        let on = |r: [[f64; 2]; 2]| {
            (uv[0] == vv[0]
                && (uv[0] == r[0][0] || uv[0] == r[0][1])
                && uv[1] >= r[1][0]
                && uv[1] <= r[1][1]
                && vv[1] >= r[1][0]
                && vv[1] <= r[1][1])
                || (uv[1] == vv[1]
                    && (uv[1] == r[1][0] || uv[1] == r[1][1])
                    && uv[0] >= r[0][0]
                    && uv[0] <= r[0][1]
                    && vv[0] >= r[0][0]
                    && vv[0] <= r[0][1])
        };
        assert!(on([[-2., 5.], [3., 9.]]) || holes.iter().any(|r| on(*r)));
    }
    let mut area = 0.;
    for (cell, r) in out.uv_ranges.iter().enumerate() {
        area += (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]);
        assert!(holes.iter().all(|h| r[0][1] <= h[0][0]
            || r[0][0] >= h[0][1]
            || r[1][1] <= h[1][0]
            || r[1][0] >= h[1][1]));
        let a = out.mesh.triangles[2 * cell];
        let b = out.mesh.triangles[2 * cell + 1];
        let corners = [
            out.mesh.positions[a[0]],
            out.mesh.positions[a[2]],
            out.mesh.positions[a[1]],
            out.mesh.positions[b[1]],
        ];
        for i in 0..=8 {
            for j in 0..=8 {
                let u = i as f64 / 8.;
                let v = j as f64 / 8.;
                let uv = [
                    r[0][0] + (r[0][1] - r[0][0]) * u,
                    r[1][0] + (r[1][1] - r[1][0]) * v,
                ];
                let p = if v <= u {
                    corners[0] * (1. - u) + corners[1] * (u - v) + corners[2] * v
                } else {
                    corners[0] * (1. - v) + corners[2] * u + corners[3] * (v - u)
                };
                assert!(
                    (original.evaluate(uv[0], uv[1]).unwrap() - p).norm()
                        <= out.error_bounds[cell] + 1e-12
                );
            }
        }
    }
    assert!((area - 37.8).abs() < 1e-12);
    assert!(out.normal_sides.iter().any(|s| s[0] == KnotSide::Left));
}
