use hagane::*;
use std::collections::BTreeMap;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn plane(z: f64, x: f64, y: f64) -> Surface {
    let n = Vec3::new(x, y, 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let u = (u - n * n.dot(u)).normalized().unwrap();
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: n.cross(u),
    }
}
fn closure(mesh: &Mesh) {
    let mut edges = BTreeMap::new();
    for tri in &mesh.triangles {
        for i in 0..3 {
            let key = |p: Vec3| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
            let a = key(mesh.positions[tri[i]]);
            let b = key(mesh.positions[tri[(i + 1) % 3]]);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let e = edges.entry(edge).or_insert((0, 0));
            e.0 += 1;
            e.1 += sign;
        }
    }
    assert!(edges.values().all(|v| *v == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
}
fn conserve(parent: NurbsGraphInertiaProperties, children: [NurbsGraphInertiaProperties; 2]) {
    let mut volume = 0.;
    let mut first = Vec3::new(0., 0., 0.);
    let mut tensor = [[0.; 3]; 3];
    for child in children {
        volume += child.volume;
        first = first + child.centroid * child.volume;
        let delta = child.centroid - parent.centroid;
        let d = [delta.x, delta.y, delta.z];
        for a in 0..3 {
            for b in 0..3 {
                tensor[a][b] += child.inertia[a][b]
                    + child.volume * ((if a == b { delta.dot(delta) } else { 0. }) - d[a] * d[b]);
            }
        }
    }
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-8 * a.abs().max(b.abs()).max(1.));
    close(volume, parent.volume);
    let centroid = first * (1. / volume);
    for (a, b) in [centroid.x, centroid.y, centroid.z].into_iter().zip([
        parent.centroid.x,
        parent.centroid.y,
        parent.centroid.z,
    ]) {
        close(a, b);
    }
    for (a, row) in tensor.iter().enumerate() {
        for (b, value) in row.iter().enumerate() {
            close(*value, parent.inertia[a][b]);
        }
    }
}
fn dense(solid: &Solid, mesh: &Mesh, error: f64) {
    for face in 2..solid.shell.faces.len() {
        let tris: Vec<_> = mesh
            .triangles
            .iter()
            .zip(&mesh.face_ids)
            .filter(|(_, id)| **id == face)
            .map(|(tri, _)| *tri)
            .collect();
        let n = ((tris.len() / 2) as f64).sqrt() as usize;
        assert_eq!(tris.len(), 2 * n * n);
        for i in 0..n {
            for j in 0..n {
                for x in [0.2, 0.6, 0.9] {
                    for y in [0.1, 0.5, 0.8] {
                        let (offset, w) = if y <= x {
                            (0, [1. - x, x - y, y])
                        } else {
                            (1, [1. - y, x, y - x])
                        };
                        let mut tri = tris[2 * (i * n + j) + offset];
                        if solid.shell.faces[face].orientation < 0 {
                            tri.swap(1, 2);
                        }
                        let chord = mesh.positions[tri[0]] * w[0]
                            + mesh.positions[tri[1]] * w[1]
                            + mesh.positions[tri[2]] * w[2];
                        let actual = solid.shell.faces[face]
                            .surface
                            .try_evaluate((i as f64 + x) / n as f64, (j as f64 + y) / n as f64)
                            .unwrap();
                        assert!((actual - chord).norm() <= error);
                    }
                }
            }
        }
    }
}
#[test]
fn ordered_skew_caps_display_actual_geometry_and_conserve_full_parent_moments() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy()).unwrap();
    let parent = source
        .split_by_plane(&plane(16., 0.2, 0.), policy())
        .unwrap()
        .lower;
    let before = format!("{:?}", parent.solid());
    let split = parent
        .split_by_interior_plane(&plane(8., 0., -0.1), policy())
        .unwrap();
    conserve(
        parent.inertia_properties(policy()).unwrap(),
        [
            split.lower.inertia_properties(policy()).unwrap(),
            split.upper.inertia_properties(policy()).unwrap(),
        ],
    );
    for child in [&split.lower, &split.upper] {
        let mesh = child.tessellate(0.2, policy()).unwrap();
        closure(&mesh);
        dense(child.solid(), &mesh, 0.2);
        assert!(
            (mesh.signed_volume() - child.volume(policy()).unwrap()).abs()
                < 0.02 * child.volume(policy()).unwrap()
        );
        assert!(child
            .export_step_mm(policy())
            .unwrap()
            .contains("RATIONAL_B_SPLINE_SURFACE"));
    }
    assert_eq!(before, format!("{:?}", parent.solid()));
}
#[test]
fn annular_skew_caps_preserve_hole_and_unresolved_optional_metrics_leave_geometry_available() {
    for radii in [[16., 8.], [16., 16. + 1e-8]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., policy())
            .unwrap()
            .bore_coaxial(4., policy())
            .unwrap();
        let parent = source
            .split_by_plane(&plane(16., 0.2, 0.), policy())
            .unwrap()
            .lower;
        let split = parent
            .split_by_interior_plane(&plane(8., 0., -0.1), policy())
            .unwrap();
        if radii[1] == 8. {
            conserve(
                parent.inertia_properties(policy()).unwrap(),
                [
                    split.lower.inertia_properties(policy()).unwrap(),
                    split.upper.inertia_properties(policy()).unwrap(),
                ],
            );
        }
        for child in [&split.lower, &split.upper] {
            let mesh = child.tessellate(0.2, policy()).unwrap();
            closure(&mesh);
            dense(child.solid(), &mesh, 0.2);
            for cap in 0..2 {
                assert_eq!(child.solid().shell.faces[cap].wires.len(), 2);
            }
            assert!(!child.export_step_mm(policy()).unwrap().is_empty());
            let before = format!("{:?}", child.solid());
            if radii[1] != 8. {
                assert!(matches!(
                    child.inertia_properties(policy()),
                    Err(Error::Unsupported(_))
                ));
            }
            assert_eq!(before, format!("{:?}", child.solid()));
            assert!(child.tessellate(1e-12, policy()).is_err());
        }
    }
}
