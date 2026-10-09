use hagane::*;
use std::collections::HashMap;
fn face(weighted: bool, orientation: i8) -> NurbsPolygonFace {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                i as f64 * 4.,
                j as f64 * 4.,
                if i == 1 { 3. } else { 0. } + if j == 1 { 2. } else { 0. },
            ));
            weights.push(if weighted {
                1. + 0.1 * (i + j) as f64
            } else {
                1.
            });
        }
    }
    let k = vec![0., 0., 0.5, 1., 1.];
    let s = NurbsSurface::new([1, 1], [k.clone(), k], [3, 3], points, weights).unwrap();
    NurbsPolygonFace::new(
        s,
        vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]],
        orientation,
        Tolerance::default(),
    )
    .unwrap()
}
fn independent(uv: [f64; 2], weighted: bool) -> Point3 {
    let i = usize::from(uv[0] >= 0.5);
    let j = usize::from(uv[1] >= 0.5);
    let u = uv[0] * 2. - i as f64;
    let v = uv[1] * 2. - j as f64;
    let mut numerator = Vec3::new(0., 0., 0.);
    let mut denominator = 0.;
    for a in 0..2 {
        for b in 0..2 {
            let x = i + a;
            let y = j + b;
            let w = (if a == 0 { 1. - u } else { u })
                * (if b == 0 { 1. - v } else { v })
                * (if weighted {
                    1. + 0.1 * (x + y) as f64
                } else {
                    1.
                });
            let point = Point3::new(
                x as f64 * 4.,
                y as f64 * 4.,
                if x == 1 { 3. } else { 0. } + if y == 1 { 2. } else { 0. },
            );
            numerator = numerator + point * w;
            denominator += w;
        }
    }
    numerator * (1. / denominator)
}
#[test]
fn crossed_creases_have_four_normals_and_one_geometric_node() {
    let f = face(false, 1);
    let display = f
        .tessellate_crease_bounded(0.01, 65536, Tolerance::default())
        .unwrap();
    let mut center = Vec::new();
    let mut nodes = HashMap::new();
    for i in 0..display.vertex_uv.len() {
        let uv = display.vertex_uv[i];
        let point = display.mesh.positions[i];
        assert!((point - independent(uv, false)).norm() < 1e-12);
        let sx = if uv[0] < 0.5 || (uv[0] == 0.5 && display.normal_sides[i][0] == KnotSide::Left) {
            6.
        } else {
            -6.
        };
        let sy = if uv[1] < 0.5 || (uv[1] == 0.5 && display.normal_sides[i][1] == KnotSide::Left) {
            4.
        } else {
            -4.
        };
        let normal = Vec3::new(-sx / 8., -sy / 8., 1.).normalized().unwrap();
        assert!((normal - display.mesh.normals[i]).norm() < 1e-12);
        if let Some(previous) = nodes.insert(display.vertex_nodes[i], point) {
            assert_eq!(previous, point);
        }
        if uv == [0.5, 0.5] {
            center.push(i);
        }
    }
    assert_eq!(center.len(), 4);
    for &i in &center {
        assert_eq!(display.vertex_nodes[i], display.vertex_nodes[center[0]]);
    }
    let mut uses = HashMap::new();
    let mut area = 0.;
    for t in &display.mesh.triangles {
        let [a, b, c] = t.map(|id| display.vertex_uv[id]);
        area += ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.;
        for axis in 0..2 {
            let lo = t
                .iter()
                .map(|id| display.vertex_uv[*id][axis])
                .fold(f64::INFINITY, f64::min);
            let hi = t
                .iter()
                .map(|id| display.vertex_uv[*id][axis])
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(lo >= 0.5 || hi <= 0.5);
        }
        let ids = t.map(|id| display.vertex_nodes[id]);
        for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            let key = if a < b { [a, b] } else { [b, a] };
            let entry = uses.entry(key).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!((area - 0.2734375).abs() < 1e-12);
    for (_, (count, sign)) in uses {
        assert!(count == 1 || (count == 2 && sign == 0));
    }
}
#[test]
fn weighted_crease_refinement_has_independent_bounds_and_orientation() {
    let a = face(true, 1)
        .tessellate_crease_bounded(0.02, 65536, Tolerance::default())
        .unwrap();
    let b = face(true, -1)
        .tessellate_crease_bounded(0.02, 65536, Tolerance::default())
        .unwrap();
    assert_eq!(a.vertex_uv, b.vertex_uv);
    assert_eq!(a.vertex_nodes, b.vertex_nodes);
    assert!(a.mesh.triangles.len() > 16);
    for (index, t) in a.mesh.triangles.iter().enumerate() {
        assert_eq!(b.mesh.triangles[index], [t[0], t[2], t[1]]);
        assert!(a.error_bounds[index] <= 0.02);
        for i in 0..=3 {
            for j in 0..=3 - i {
                let weights = [i as f64 / 3., j as f64 / 3., (3 - i - j) as f64 / 3.];
                let mut uv = [0.; 2];
                let mut chord = Vec3::new(0., 0., 0.);
                for k in 0..3 {
                    for (axis, value) in uv.iter_mut().enumerate() {
                        *value += a.vertex_uv[t[k]][axis] * weights[k];
                    }
                    chord = chord + a.mesh.positions[t[k]] * weights[k];
                }
                assert!((independent(uv, true) - chord).norm() <= a.error_bounds[index]);
            }
        }
    }
    for (x, y) in a.mesh.normals.iter().zip(b.mesh.normals) {
        assert!((*x + y).norm() < 1e-12);
    }
    assert!(face(true, 1)
        .tessellate_crease_bounded(0.02, 1, Tolerance::default())
        .is_err());
    assert!(face(true, 1)
        .tessellate_crease_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
}

#[test]
fn trim_boundary_on_crease_uses_only_its_retained_side() {
    let base = face(false, 1);
    let f = NurbsPolygonFace::new(
        base.boundary.surface,
        vec![[0.5, 0.125], [0.875, 0.25], [0.5, 0.875]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    let display = f
        .tessellate_crease_bounded(0.01, 65536, Tolerance::default())
        .unwrap();
    for (i, uv) in display.vertex_uv.iter().enumerate() {
        assert!(uv[0] >= 0.5);
        assert_eq!(display.normal_sides[i][0], KnotSide::Right);
        assert!((display.mesh.positions[i] - independent(*uv, false)).norm() < 1e-12);
    }
    let mut dirty = f;
    dirty.face.wires[0].coedges[0].edge = 99;
    assert!(dirty
        .tessellate_crease_bounded(0.01, 65536, Tolerance::default())
        .is_err());
}
