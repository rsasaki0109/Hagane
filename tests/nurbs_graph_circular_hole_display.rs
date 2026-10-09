use hagane::*;
use std::collections::{BTreeMap, BTreeSet};
fn mesh_volume(mesh: &Mesh) -> f64 {
    let origin = mesh.positions[0];
    mesh.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.positions[i] - origin);
            a.dot(b.cross(c)) / 6.
        })
        .sum()
}
fn inspect(body: &NurbsGraphCircularHoledSolid, error: f64) -> NurbsGraphMesh {
    let display = body
        .tessellate_bounded(error, 65536, Tolerance::default())
        .unwrap();
    let source = body.source();
    let inverse = source.placement().inverse().unwrap();
    let [w, d, _] = source.dimensions();
    let trim = 7. * body.radius() / (8. * (display.subdivisions * display.subdivisions) as f64)
        * (1. + source.bulge().abs() * w.recip().hypot(d.recip()));
    let mut uses: BTreeMap<(usize, usize), (usize, i32)> = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let mut faces = BTreeSet::new();
    let mut circular_nodes = BTreeSet::new();
    for (i, &node) in display.vertex_nodes.iter().enumerate() {
        if let Some(previous) = nodes.insert(node, display.mesh.positions[i]) {
            assert_eq!(previous, display.mesh.positions[i]);
        }
        let fi = display.vertex_faces[i];
        faces.insert(fi);
        let uv = display.vertex_uv[i];
        let actual = body.brep().shell.faces[fi]
            .surface
            .try_evaluate(uv[0], uv[1])
            .unwrap();
        assert!((actual - display.mesh.positions[i]).norm() < 1e-6);
        if fi >= 6 {
            let local = inverse.point(display.mesh.positions[i]);
            let normal = inverse.vector(display.mesh.normals[i]);
            assert!(
                ((local.x - body.center()[0]).hypot(local.y - body.center()[1]) - body.radius())
                    .abs()
                    < 1e-8
            );
            assert!(
                normal.dot(Vec3::new(
                    local.x - body.center()[0],
                    local.y - body.center()[1],
                    0.
                )) < 0.
            );
            if uv[1] == 0. || uv[1] == 1. {
                circular_nodes.insert(node);
                let edge = if uv[1] == 0. {
                    12 + fi - 6
                } else {
                    16 + fi - 6
                };
                let exact = body.brep().edges[edge].curve.try_evaluate(uv[0]).unwrap();
                assert!((exact - display.mesh.positions[i]).norm() < 1e-8);
            }
        }
    }
    assert_eq!(faces.len(), 10);
    for (index, &triangle) in display.mesh.triangles.iter().enumerate() {
        let fi = display.mesh.face_ids[index];
        let face = &body.brep().shell.faces[fi];
        let bound = display.error_bounds[index];
        assert!(bound.is_finite() && bound > 0. && bound <= error);
        assert!(triangle.iter().all(|&i| display.vertex_faces[i] == fi));
        for k in 0..3 {
            let a = display.vertex_nodes[triangle[k]];
            let b = display.vertex_nodes[triangle[(k + 1) % 3]];
            let entry = uses.entry((a.min(b), a.max(b))).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
        for a in 0..=8 {
            for b in 0..=8 - a {
                let weights = [a as f64 / 8., b as f64 / 8., (8 - a - b) as f64 / 8.];
                let uv = std::array::from_fn::<_, 2, _>(|axis| {
                    (0..3)
                        .map(|j| weights[j] * display.vertex_uv[triangle[j]][axis])
                        .sum()
                });
                let point = (0..3).fold(Vec3::new(0., 0., 0.), |p, j| {
                    p + display.mesh.positions[triangle[j]] * weights[j]
                });
                let surface = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                assert!(
                    (point - surface).norm() <= bound,
                    "face{fi} tri{index}: {}>{bound}",
                    (point - surface).norm()
                );
                if fi < 2 {
                    let radial = (w * uv[0] - body.center()[0]).hypot(d * uv[1] - body.center()[1]);
                    assert!(
                        body.radius() - radial <= trim + 1e-8,
                        "unbounded chord intrusion"
                    );
                }
            }
        }
    }
    assert!(uses.values().all(|&(count, sign)| count == 2 && sign == 0));
    assert_eq!(
        nodes.len() as isize - uses.len() as isize + display.mesh.triangles.len() as isize,
        0
    );
    assert!(circular_nodes.iter().all(|&node| display
        .vertex_nodes
        .iter()
        .enumerate()
        .any(|(i, &n)| n == node && display.vertex_faces[i] < 2)));
    display
}
fn expected_volume(source: &NurbsGraphSolid, c: [f64; 2], r: f64) -> f64 {
    let [w, d, h] = source.dimensions();
    let b = source.bulge();
    let u = c[0] / w;
    let v = c[1] / d;
    let removed = std::f64::consts::PI
        * r
        * r
        * (h + 4. * b * u * (1. - u) * v * (1. - v)
            - b * r * r * (v * (1. - v) / (w * w) + u * (1. - u) / (d * d))
            + b * r.powi(4) / (6. * w * w * d * d));
    source.volume().unwrap() - removed
}
#[test]
fn actual_cap_and_wall_geometry_closed_mesh_with_bounded_circular_trim() {
    let t = Tolerance::default();
    for b in [-12., 0., 30.] {
        let source = NurbsGraphSolid::new([80., 60., 20.], b, t).unwrap();
        let body = source.through_xy_circle([35., 28.], 9., t).unwrap();
        let display = inspect(&body, 0.5);
        let expected = expected_volume(&source, [35., 28.], 9.);
        assert!((mesh_volume(&display.mesh) - expected).abs() < 0.5 * 80. * 60.);
    }
}
#[test]
fn shifted_trim_and_arbitrary_rigid_placement_preserve_actual_sampling() {
    let t = Tolerance::default();
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.47).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([80., 60., 20.], -8., t)
        .unwrap()
        .trimmed_uv([[0.12, 0.87], [0.18, 0.91]], t)
        .unwrap()
        .transformed(transform, t)
        .unwrap();
    let body = source.through_xy_circle([36., 31.], 8., t).unwrap();
    inspect(&body, 0.5);
}
#[test]
fn convergence_and_explicit_budget_precision_failures() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    let body = source.through_xy_circle([40., 30.], 10., t).unwrap();
    let coarse = inspect(&body, 1.);
    let fine = inspect(&body, 0.2);
    let expected = expected_volume(&source, [40., 30.], 10.);
    assert!(fine.mesh.triangles.len() > coarse.mesh.triangles.len());
    assert!(
        (mesh_volume(&fine.mesh) - expected).abs() < (mesh_volume(&coarse.mesh) - expected).abs()
    );
    for (error, budget) in [
        (f64::NAN, 65536),
        (0., 65536),
        (1., 31),
        (0.01, 32),
        (1e-14, 65536),
        (1e-6, 65536),
    ] {
        assert!(body.tessellate_bounded(error, budget, t).is_err());
    }
}
