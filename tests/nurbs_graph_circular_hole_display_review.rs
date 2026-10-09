use hagane::*;
use std::collections::BTreeMap;
#[test]
fn dense_actual_face_and_true_trim_distance_stay_inside_reported_triangle_bounds() {
    let t = Tolerance::default();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([20., 12., 3.], -2., t)
        .unwrap()
        .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], t)
        .unwrap()
        .transformed(frame, t)
        .unwrap();
    let center = [9., 5.];
    let radius = 1.25;
    let body = NurbsGraphCircularHoledSolid::new(&source, center, radius, t).unwrap();
    for error in [0.5, 0.1] {
        let display = body.tessellate_bounded(error, 65536, t).unwrap();
        let mut uses: BTreeMap<(usize, usize), (usize, i32)> = BTreeMap::new();
        for (i, tri) in display.mesh.triangles.iter().enumerate() {
            let face = display.mesh.face_ids[i];
            let bound = display.error_bounds[i];
            assert!(bound <= error);
            for j in 0..3 {
                let a = display.vertex_nodes[tri[j]];
                let b = display.vertex_nodes[tri[(j + 1) % 3]];
                let entry = uses.entry((a.min(b), a.max(b))).or_default();
                entry.0 += 1;
                entry.1 += if a < b { 1 } else { -1 };
            }
            for a in 0..=4 {
                for b in 0..=4 - a {
                    let factors = [a as f64 / 4., b as f64 / 4., (4 - a - b) as f64 / 4.];
                    let uv: [f64; 2] = std::array::from_fn(|axis| {
                        (0..3)
                            .map(|k| factors[k] * display.vertex_uv[tri[k]][axis])
                            .sum()
                    });
                    let point = (0..3).fold(Point3::new(0., 0., 0.), |s, k| {
                        s + display.mesh.positions[tri[k]] * factors[k]
                    });
                    let exact = body.brep().shell.faces[face]
                        .surface
                        .try_evaluate(uv[0], uv[1])
                        .unwrap();
                    assert!((point - exact).norm() <= bound + 1e-9);
                    if face < 2 {
                        let mut x = 20. * uv[0];
                        let mut y = 12. * uv[1];
                        let dx = x - center[0];
                        let dy = y - center[1];
                        let distance = dx.hypot(dy);
                        if distance < radius {
                            assert!(distance > 0.);
                            x = center[0] + radius * dx / distance;
                            y = center[1] + radius * dy / distance;
                        }
                        let u = x / 20.;
                        let v = y / 12.;
                        let h = if face == 0 {
                            0.
                        } else {
                            3. - 8. * u * (1. - u) * v * (1. - v)
                        };
                        // A cap chord may enter the void, but the reported trim
                        // allowance must bound distance to an actual material point.
                        let material = frame.point(Point3::new(x, y, h));
                        assert!((point - material).norm() <= bound + 1e-9);
                    }
                }
            }
        }
        assert!(uses
            .values()
            .all(|&(count, balance)| count == 2 && balance == 0));
    }
}
#[test]
fn precision_budget_invalid_error_and_shared_geometry_mutation_reject() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], 4., t).unwrap();
    let mut body = NurbsGraphCircularHoledSolid::new(&source, [9., 5.], 1.25, t).unwrap();
    for error in [0., -1., f64::NAN, 1e-14] {
        assert!(body.tessellate_bounded(error, 65536, t).is_err());
    }
    assert!(body.tessellate_bounded(0.001, 32, t).is_err());
    body.solid.vertices[0].point.x += t.linear / 100.;
    assert!(body.tessellate_bounded(0.5, 65536, t).is_err());
}
