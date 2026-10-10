use hagane::*;
use std::collections::BTreeMap;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn split() -> NurbsFrustumPlaneSplit {
    let shape = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy()).unwrap();
    let normal = Vec3::new(0.1, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., 12.),
        u,
        v: normal.cross(u),
    };
    shape.split_by_plane(&plane, policy()).unwrap()
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
fn dense_bound(child: &NurbsObliqueFrustumSolid, mesh: &Mesh, error: f64) {
    for face in 2..6 {
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
                        let tri = tris[2 * (i * n + j) + offset];
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
fn oblique_rational_generators_share_exact_mesh_nodes_and_actual_patch_bounds() {
    let result = split();
    for child in [&result.lower, &result.upper] {
        let mesh = child.tessellate(0.2, policy()).unwrap();
        exact_closed(&mesh);
        dense_bound(child, &mesh, 0.2);
        let Surface::Nurbs(surface) = &child.solid().shell.faces[2].surface else {
            panic!()
        };
        let w = surface.weights();
        assert_ne!(
            w[0], w[1],
            "fixture must exercise nonlinear generator mapping"
        );
        let midpoint = (child.solid().vertices[0].point + child.solid().vertices[4].point) * 0.5;
        assert!((surface.evaluate(0., 0.5).unwrap() - midpoint).norm() > 1e-4);
    }
}
#[test]
fn finer_bounded_oblique_display_converges_and_unresolved_requests_reject() {
    let result = split();
    for child in [&result.lower, &result.upper] {
        let coarse = child.tessellate(0.5, policy()).unwrap();
        let fine = child.tessellate(0.05, policy()).unwrap();
        exact_closed(&coarse);
        exact_closed(&fine);
        dense_bound(child, &fine, 0.05);
        assert!(fine.triangles.len() > coarse.triangles.len());
        let exact = child.volume(policy()).unwrap();
        assert!((fine.signed_volume() - exact).abs() < (coarse.signed_volume() - exact).abs());
        for error in [0., -1., f64::NAN, f64::INFINITY, 1e-14] {
            assert!(child.tessellate(error, policy()).is_err());
        }
        assert_eq!(
            child.tessellate(0.5, policy()).unwrap().triangles,
            coarse.triangles
        );
    }
}
