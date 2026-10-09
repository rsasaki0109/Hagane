use hagane::*;
use std::collections::HashMap;

fn surface() -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 10., 0.),
            Point3::new(10., 0., 0.),
            Point3::new(10., 10., 4.),
        ],
        vec![1., 2., 3., 1.],
    )
    .unwrap()
}
fn exact(uv: [f64; 2]) -> Point3 {
    let [u, v] = uv;
    let b = [
        (1. - u) * (1. - v),
        2. * (1. - u) * v,
        3. * u * (1. - v),
        u * v,
    ];
    let w = b.iter().sum::<f64>();
    Point3::new(
        10. * (b[2] + b[3]) / w,
        10. * (b[1] + b[3]) / w,
        4. * b[3] / w,
    )
}
fn outer() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [0., 1.]]
}
fn hole() -> [[f64; 2]; 2] {
    [[0.2, 0.3], [0.2, 0.3]]
}
fn make() -> NurbsPolygonHoledFace {
    NurbsPolygonHoledFace::new(surface(), outer(), vec![hole()], 1, Tolerance::default()).unwrap()
}
#[test]
fn independent_rational_positions_bounds_material_area_and_boundary_incidence() {
    let face = make();
    face.validate(Tolerance::default()).unwrap();
    let display = face
        .tessellate_bounded(0.2, 65536, Tolerance::default())
        .unwrap();
    let mut nodes = HashMap::new();
    for (i, uv) in display.vertex_uv.iter().enumerate() {
        let p = display.mesh.positions[i];
        assert!((p - exact(*uv)).norm() < 1e-10);
        assert!((display.mesh.normals[i].norm() - 1.).abs() < 1e-10);
        if let Some((old_uv, old_p)) = nodes.insert(display.vertex_nodes[i], (*uv, p)) {
            assert_eq!(old_uv, *uv);
            assert_eq!(old_p, p);
        }
    }
    let mut area = 0.;
    let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    for (t, ids) in display.mesh.triangles.iter().enumerate() {
        let uv = ids.map(|i| display.vertex_uv[i]);
        let signed = ((uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1])
            - (uv[1][1] - uv[0][1]) * (uv[2][0] - uv[0][0]))
            / 2.;
        assert!(signed > 0.);
        area += signed;
        let bound = display.error_bounds[t];
        assert!(bound > 0. && bound <= 0.2);
        for a in 0..=8 {
            for b in 0..=8 - a {
                let f = [a as f64 / 8., b as f64 / 8., 1. - (a + b) as f64 / 8.];
                let at = [0, 1].map(|axis| (0..3).map(|i| f[i] * uv[i][axis]).sum());
                assert!(
                    !(at[0] > 0.2 + 1e-12
                        && at[0] < 0.3 - 1e-12
                        && at[1] > 0.2 + 1e-12
                        && at[1] < 0.3 - 1e-12)
                );
                let p = ids.map(|i| display.mesh.positions[i]);
                let linear = Point3::new(
                    (0..3).map(|i| f[i] * p[i].x).sum(),
                    (0..3).map(|i| f[i] * p[i].y).sum(),
                    (0..3).map(|i| f[i] * p[i].z).sum(),
                );
                assert!((exact(at) - linear).norm() <= bound + 1e-10);
            }
        }
        for i in 0..3 {
            let a = display.vertex_nodes[ids[i]];
            let b = display.vertex_nodes[ids[(i + 1) % 3]];
            let entry = edges.entry((a.min(b), a.max(b))).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!((area - 0.49).abs() < 1e-10);
    let mut hole_edges = 0;
    for ((a, b), (count, balance)) in edges {
        let ua = nodes[&a].0;
        let ub = nodes[&b].0;
        let outer_boundary = (ua[0] == 0. && ub[0] == 0.)
            || (ua[1] == 0. && ub[1] == 0.)
            || ((ua[0] + ua[1] - 1.).abs() < 1e-12 && (ub[0] + ub[1] - 1.).abs() < 1e-12);
        let inner = (0..2).any(|axis| {
            [0.2, 0.3]
                .into_iter()
                .any(|x| ua[axis] == x && ub[axis] == x)
                && [ua, ub]
                    .iter()
                    .all(|uv| uv[1 - axis] >= 0.2 && uv[1 - axis] <= 0.3)
        });
        if inner {
            hole_edges += 1;
        }
        assert_eq!(count, if outer_boundary || inner { 1 } else { 2 });
        assert_eq!(balance.abs(), if outer_boundary || inner { 1 } else { 0 });
    }
    assert!(hole_edges >= 4);
}
#[test]
fn retained_hole_wire_is_clockwise_and_corruption_is_rejected() {
    let face = make();
    assert_eq!(face.face.wires.len(), 2);
    assert_eq!(face.edges.len(), 7);
    assert_eq!(face.vertices.len(), 7);
    let mut area = 0.;
    for (i, c) in face.face.wires[1].coedges.iter().enumerate() {
        let next = &face.face.wires[1].coedges[(i + 1) % 4];
        let edge = &face.edges[c.edge];
        let e = edge.vertices;
        let en = face.edges[next.edge].vertices;
        assert_eq!(
            if c.forward { e[1] } else { e[0] },
            if next.forward { en[0] } else { en[1] }
        );
        let PCurve::Affine { origin, direction } = &c.pcurve else {
            panic!("affine boundary required")
        };
        area += origin[0] * (origin[1] + direction[1]) - origin[1] * (origin[0] + direction[0]);
    }
    assert!((area + 0.02).abs() < 1e-12);
    let mut dirty = face.clone();
    dirty.face.wires[1].coedges[0].edge = 999;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = face.clone();
    dirty.edges[3].vertices[0] = 999;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = face.clone();
    dirty.vertices[3].point.z += 1.;
    assert!(dirty.validate(Tolerance::default()).is_err());
}
#[test]
fn touching_near_contact_overlapping_and_invalid_holes_fail() {
    for holes in [
        vec![[[0.4, 0.6], [0.4, 0.6]]],
        vec![[[0.2, 0.3], [0.6, 0.7]]],
        vec![[[0.2, 0.3], [0.6, 0.7 - 1e-15]]],
        vec![[[0.2, 0.3], [0.2, 0.3]], [[0.3, 0.4], [0.2, 0.3]]],
        vec![[[0.2, 0.3], [0.2, 0.3]], [[0.3 + 1e-15, 0.4], [0.2, 0.3]]],
        vec![[[0.2, 0.3], [0.2, 0.3]], [[0.25, 0.35], [0.25, 0.35]]],
        vec![[[0.3, 0.2], [0.2, 0.3]]],
        vec![[[f64::NAN, 0.3], [0.2, 0.3]]],
    ] {
        assert!(
            NurbsPolygonHoledFace::new(surface(), outer(), holes, 1, Tolerance::default()).is_err()
        );
    }
    let face = make();
    assert!(face
        .tessellate_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
    assert!(face
        .tessellate_bounded(0.2, 1, Tolerance::default())
        .is_err());
}
#[test]
fn crossed_c0_seams_share_geometry_and_keep_distinct_normals() {
    let controls = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                Point3::new(
                    i as f64 * 10.,
                    j as f64 * 10.,
                    if i == 1 { 4. } else { 0. } + if j == 1 { 2. } else { 0. },
                )
            })
        })
        .collect();
    let roof = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 0.5, 1., 1.]],
        [3, 3],
        controls,
        vec![1.; 9],
    )
    .unwrap();
    let face = NurbsPolygonHoledFace::new(
        roof.clone(),
        vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        vec![hole()],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let display = face
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    let mut variants: HashMap<usize, Vec<Vec3>> = HashMap::new();
    let mut positions = HashMap::new();
    for (i, &node) in display.vertex_nodes.iter().enumerate() {
        if let Some(old) = positions.insert(node, display.mesh.positions[i]) {
            assert_eq!(old, display.mesh.positions[i]);
        }
        variants
            .entry(node)
            .or_default()
            .push(display.mesh.normals[i]);
    }
    assert!(variants
        .values()
        .any(|list| list.iter().any(|n| (*n - list[0]).norm() > 0.1)));
    let center_nodes: Vec<_> = display
        .vertex_uv
        .iter()
        .enumerate()
        .filter(|(_, uv)| **uv == [0.5, 0.5])
        .map(|(i, _)| display.vertex_nodes[i])
        .collect();
    assert!(!center_nodes.is_empty());
    assert!(center_nodes.iter().all(|id| *id == center_nodes[0]));
    assert_eq!(variants[&center_nodes[0]].len(), 4);
    for t in display.mesh.triangles {
        for axis in 0..2 {
            let values = t.map(|i| display.vertex_uv[i][axis]);
            assert!(values.iter().all(|v| *v <= 0.5) || values.iter().all(|v| *v >= 0.5));
        }
    }
    let aligned = NurbsPolygonHoledFace::new(
        roof,
        vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        vec![[[0.5, 0.7], [0.4, 0.6]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let m = aligned
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    let center: Vec<_> = m
        .vertex_uv
        .iter()
        .enumerate()
        .filter(|(_, uv)| **uv == [0.5, 0.5])
        .map(|(i, _)| i)
        .collect();
    assert_eq!(center.len(), 2);
    assert!(center
        .iter()
        .all(|i| m.normal_sides[*i][0] == KnotSide::Left));
    let mut area = 0.;
    let mut incidence: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    let mut uv_nodes = HashMap::new();
    for (i, &node) in m.vertex_nodes.iter().enumerate() {
        uv_nodes.insert(node, m.vertex_uv[i]);
    }
    for ids in &m.mesh.triangles {
        let uv = ids.map(|i| m.vertex_uv[i]);
        area += ((uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1])
            - (uv[1][1] - uv[0][1]) * (uv[2][0] - uv[0][0]))
            / 2.;
        let centroid = [0, 1].map(|axis| uv.iter().map(|p| p[axis]).sum::<f64>() / 3.);
        assert!(
            !(centroid[0] > 0.5 && centroid[0] < 0.7 && centroid[1] > 0.4 && centroid[1] < 0.6)
        );
        for i in 0..3 {
            let a = m.vertex_nodes[ids[i]];
            let b = m.vertex_nodes[ids[(i + 1) % 3]];
            let edge = incidence.entry((a.min(b), a.max(b))).or_default();
            edge.0 += 1;
            edge.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!((area - 0.96).abs() < 1e-10);
    let mut inner_count = 0;
    for ((a, b), (count, balance)) in incidence {
        let a = uv_nodes[&a];
        let b = uv_nodes[&b];
        let outer = (0..2).any(|axis| [0., 1.].iter().any(|x| a[axis] == *x && b[axis] == *x));
        let hole = (0..2).any(|axis| {
            [[0.5, 0.7], [0.4, 0.6]][axis]
                .iter()
                .any(|x| a[axis] == *x && b[axis] == *x)
                && [a, b].iter().all(|p| {
                    p[1 - axis] >= [[0.5, 0.7], [0.4, 0.6]][1 - axis][0]
                        && p[1 - axis] <= [[0.5, 0.7], [0.4, 0.6]][1 - axis][1]
                })
        });
        if hole {
            inner_count += 1;
        }
        assert_eq!(count, if outer || hole { 1 } else { 2 });
        assert_eq!(balance.abs(), if outer || hole { 1 } else { 0 });
    }
    assert!(inner_count >= 4);
}
