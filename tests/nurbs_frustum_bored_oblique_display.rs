use hagane::*;
use std::collections::BTreeMap;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn key(p: Point3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}
fn exact_closed(mesh: &Mesh) {
    let mut edges: BTreeMap<([u64; 3], [u64; 3]), (usize, i32)> = BTreeMap::new();
    for triangle in &mesh.triangles {
        for i in 0..3 {
            let a = key(mesh.positions[triangle[i]]);
            let b = key(mesh.positions[triangle[(i + 1) % 3]]);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let entry = edges.entry(edge).or_default();
            entry.0 += 1;
            entry.1 += sign;
        }
    }
    assert!(edges.values().all(|entry| *entry == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
}
fn dense_bound(child: &NurbsBoredObliqueFrustumSolid, mesh: &Mesh, error: f64) {
    for face in 2..10 {
        let tris: Vec<_> = mesh
            .triangles
            .iter()
            .zip(&mesh.face_ids)
            .filter_map(|(tri, id)| (*id == face).then_some(*tri))
            .collect();
        let n = ((tris.len() / 2) as f64).sqrt() as usize;
        assert_eq!(tris.len(), 2 * n * n);
        assert!(n.is_power_of_two());
        let Surface::Nurbs(surface) = &child.solid().shell.faces[face].surface else {
            panic!("missing actual ruled surface")
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
                        let mut tri = tris[2 * (i * n + j) + offset];
                        if child.solid().shell.faces[face].orientation < 0 {
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
                            "actual patch {face} cell{i},{j} error{} >{error}",
                            (exact - chord).norm()
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn actual_eight_walls_annular_caps_and_rational_generators_are_closed_and_bounded() {
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., policy())
            .unwrap()
            .bore_coaxial(4., policy())
            .unwrap();
        let normal = Vec3::new(0.2, 0.1, 1.).normalized().unwrap();
        let u = Vec3::new(0., 1., 0.);
        let u = (u - normal * normal.dot(u)).normalized().unwrap();
        let plane = Surface::Plane {
            origin: Point3::new(0., 0., 12.),
            u,
            v: normal.cross(u),
        };
        let split = source.split_by_plane(&plane, policy()).unwrap();
        for child in [&split.lower, &split.upper] {
            let mesh = child.tessellate(0.2, policy()).unwrap();
            exact_closed(&mesh);
            dense_bound(child, &mesh, 0.2);
            assert!(mesh.face_ids.iter().all(|id| *id < 10));
            for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                if *face >= 6 {
                    let p = mesh.positions[tri[0]];
                    let n = mesh.normals[tri[0]];
                    assert!(p.x * n.x + p.y * n.y < 0.);
                }
            }
            for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                if *face < 2 {
                    let Surface::Plane { origin, u, v } = &child.solid().shell.faces[*face].surface
                    else {
                        panic!("cap must retain actual plane")
                    };
                    let normal = u.cross(*v).normalized().unwrap();
                    for a in 0..=8 {
                        for b in 0..=8 - a {
                            let wa = a as f64 / 8.;
                            let wb = b as f64 / 8.;
                            let p = mesh.positions[tri[0]] * wa
                                + mesh.positions[tri[1]] * wb
                                + mesh.positions[tri[2]] * (1. - wa - wb);
                            assert!((p - *origin).dot(normal).abs() < 1e-9);
                            assert!(p.x.hypot(p.y) >= 4. - 0.2);
                            let outer_radius = radii[0] + (radii[1] - radii[0]) * p.z / 24.;
                            assert!(p.x.hypot(p.y) <= outer_radius + 1e-9);
                        }
                    }
                }
            }
            let volume = child.volume(policy()).unwrap();
            assert!((mesh.signed_volume() - volume).abs() < 0.02 * volume);
        }
    }
}
#[test]
fn posed_child_is_bit_closed_and_invalid_chord_requests_are_explicit() {
    let (sin, cos) = 0.37_f64.sin_cos();
    let frame = Frame3::new(
        Point3::new(12., -5., 8.),
        [
            Vec3::new(cos, 0., -sin),
            Vec3::new(0., 1., 0.),
            Vec3::new(sin, 0., cos),
        ],
        Tolerance::default(),
    )
    .unwrap();
    let source = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy())
        .unwrap()
        .bore_coaxial(4., policy())
        .unwrap();
    let axes = frame.axes();
    let normal = (axes[2] + axes[0] * 0.2).normalized().unwrap();
    let plane = Surface::Plane {
        origin: frame.origin() + axes[2] * 12.,
        u: axes[1],
        v: normal.cross(axes[1]),
    };
    let split = source.split_by_plane(&plane, policy()).unwrap();
    for child in [&split.lower, &split.upper] {
        let mesh = child.tessellate(0.2, policy()).unwrap();
        exact_closed(&mesh);
        dense_bound(child, &mesh, 0.2);
        for error in [0., -1., f64::NAN, 1e-12] {
            assert!(child.tessellate(error, policy()).is_err());
        }
    }
}
