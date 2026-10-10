use hagane::*;
use std::collections::{BTreeMap, BTreeSet};
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn key(p: Point3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}
fn closed_genus_one(mesh: &Mesh) {
    let mut nodes = BTreeSet::new();
    let mut edges: BTreeMap<([u64; 3], [u64; 3]), (usize, i32)> = BTreeMap::new();
    for tri in &mesh.triangles {
        for i in 0..3 {
            let a = key(mesh.positions[tri[i]]);
            let b = key(mesh.positions[tri[(i + 1) % 3]]);
            nodes.insert(a);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let count = edges.entry(edge).or_default();
            count.0 += 1;
            count.1 += sign;
        }
    }
    assert!(edges.values().all(|value| *value == (2, 0)));
    assert_eq!(
        nodes.len() as isize - edges.len() as isize + mesh.triangles.len() as isize,
        0
    );
    assert!(mesh.signed_volume() > 0.);
}
fn actual_wall_bounds(body: &NurbsBoredFrustumSolid, mesh: &Mesh, error: f64) {
    let mut common = None;
    for face in 2..10 {
        let triangles: Vec<_> = mesh
            .triangles
            .iter()
            .zip(&mesh.face_ids)
            .filter_map(|(tri, id)| (*id == face).then_some(*tri))
            .collect();
        let n = ((triangles.len() / 2) as f64).sqrt() as usize;
        assert_eq!(triangles.len(), 2 * n * n);
        assert!(n.is_power_of_two());
        if let Some(common) = common {
            assert_eq!(n, common)
        } else {
            common = Some(n)
        };
        let retained = &body.solid().shell.faces[face];
        let Surface::Nurbs(surface) = &retained.surface else {
            panic!()
        };
        for i in 0..n {
            for j in 0..n {
                for a in 0..=4 {
                    for b in 0..=4 {
                        let x = a as f64 / 4.;
                        let y = b as f64 / 4.;
                        let (offset, weights) = if y <= x {
                            (0, [1. - x, x - y, y])
                        } else {
                            (1, [1. - y, x, y - x])
                        };
                        let mut tri = triangles[2 * (i * n + j) + offset];
                        if retained.orientation < 0 {
                            tri.swap(1, 2);
                        }
                        let chord = mesh.positions[tri[0]] * weights[0]
                            + mesh.positions[tri[1]] * weights[1]
                            + mesh.positions[tri[2]] * weights[2];
                        let exact = surface
                            .evaluate((i as f64 + x) / n as f64, (j as f64 + y) / n as f64)
                            .unwrap();
                        assert!(
                            (exact - chord).norm() <= error,
                            "face{face} cell{i},{j} error{}",
                            (exact - chord).norm()
                        );
                    }
                }
            }
        }
    }
}
fn cap_opening(body: &NurbsBoredFrustumSolid, mesh: &Mesh) {
    let frame = body.source().frame();
    let radius = body.radius();
    for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
        if *face >= 2 {
            continue;
        }
        let points = tri.map(|id| frame.local_point(mesh.positions[id]));
        for p in points {
            assert!(p.x.hypot(p.y) >= radius * (1. - 1e-12));
        }
        let cross = |a: Point3, b: Point3| (b.x - a.x) * (-a.y) - (b.y - a.y) * (-a.x);
        let signs = [
            cross(points[0], points[1]),
            cross(points[1], points[2]),
            cross(points[2], points[0]),
        ];
        assert!(
            !(signs.iter().all(|v| *v >= 0.) || signs.iter().all(|v| *v <= 0.)),
            "cap triangle filled bore center"
        );
    }
}
#[test]
fn bounded_actual_annular_display_is_closed_genus_one_and_preserves_hole() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy(1.)).unwrap();
    let body = source.bore_coaxial(4., policy(1.)).unwrap();
    let coarse = body.tessellate(0.5, policy(1.)).unwrap();
    let fine = body.tessellate(0.1, policy(1.)).unwrap();
    for mesh in [&coarse, &fine] {
        closed_genus_one(mesh);
        cap_opening(&body, mesh);
    }
    actual_wall_bounds(&body, &fine, 0.1);
    assert!(fine.triangles.len() > coarse.triangles.len());
    let exact = body.volume(policy(1.)).unwrap();
    assert!((fine.signed_volume() - exact).abs() < (coarse.signed_volume() - exact).abs());
    for (tri, face) in fine.triangles.iter().zip(&fine.face_ids) {
        if *face < 6 {
            continue;
        }
        let p = fine.positions[tri[0]];
        let n = fine.normals[tri[0]];
        assert!(p.x * n.x + p.y * n.y < 0.);
    }
}
#[test]
fn posed_scaled_display_uses_actual_nets_and_rejects_unresolved_error_or_wall() {
    for scale in [1e-4, 1., 10.] {
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let frame = Frame3::new_with_tolerance(
            Point3::new(13. * scale, -7. * scale, 5. * scale),
            rotation.axes(),
            policy(scale),
        )
        .unwrap();
        let source =
            NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
                .unwrap();
        let body = source.bore_coaxial(4. * scale, policy(scale)).unwrap();
        let mesh = body.tessellate(0.2 * scale, policy(scale)).unwrap();
        closed_genus_one(&mesh);
        cap_opening(&body, &mesh);
        actual_wall_bounds(&body, &mesh, 0.2 * scale);
        for error in [0., -1., f64::NAN, f64::INFINITY, 1e-14 * scale] {
            assert!(body.tessellate(error, policy(scale)).is_err());
        }
        assert!(source
            .bore_coaxial(8. * scale - 1e-7 * scale, policy(scale))
            .is_err());
        assert_eq!(
            body.tessellate(0.2 * scale, policy(scale))
                .unwrap()
                .triangles,
            mesh.triangles
        );
    }
}
