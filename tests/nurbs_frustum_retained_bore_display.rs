use hagane::*;
use std::collections::BTreeMap;
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn plane(z: f64, x: f64, y: f64) -> Surface {
    let normal = Vec3::new(x, y, 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    let u = (u - normal * normal.dot(u)).normalized().unwrap();
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: normal.cross(u),
    }
}
fn verify(body: &NurbsBoredFrustumPlaneBoundedSolid, radius: f64) {
    let mesh = body.tessellate(0.2, policy()).unwrap();
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
    assert!(edges.values().all(|entry| *entry == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
    assert!(
        (mesh.signed_volume() - body.volume(policy()).unwrap()).abs()
            < 0.025 * body.volume(policy()).unwrap()
    );
    for (tri, face) in mesh.triangles.iter().zip(&mesh.face_ids) {
        if *face < 2 {
            for a in 0..=8 {
                for b in 0..=8 - a {
                    let x = a as f64 / 8.;
                    let y = b as f64 / 8.;
                    let p = mesh.positions[tri[0]] * x
                        + mesh.positions[tri[1]] * y
                        + mesh.positions[tri[2]] * (1. - x - y);
                    assert!(p.x.hypot(p.y) >= radius - 0.2);
                    let Surface::Plane { origin, u, v } = &body.solid().shell.faces[*face].surface
                    else {
                        panic!("actual annular plane")
                    };
                    assert!((p - *origin).dot(u.cross(*v).normalized().unwrap()).abs() < 1e-9);
                }
            }
        } else if *face >= 6 {
            let p = mesh.positions[tri[0]];
            let n = mesh.normals[tri[0]];
            assert!(p.x * n.x + p.y * n.y < 0.);
        }
    }
}
#[test]
fn drilled_and_enlarged_actual_kept_and_removed_annuli_are_closed_without_filling_holes() {
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy()).unwrap();
    let parent = source
        .split_by_plane(&plane(16., 0.2, 0.), policy())
        .unwrap()
        .lower;
    let retained = parent
        .split_by_interior_plane(&plane(8., 0., -0.1), policy())
        .unwrap()
        .upper;
    let before = format!("{:?}", retained.solid());
    let drilled = retained.partition_coaxial_bore(3., policy()).unwrap();
    verify(&drilled.kept, 3.);
    let enlargement = drilled
        .kept
        .partition_coaxial_bore_enlargement(5., policy())
        .unwrap();
    verify(&enlargement.kept, 5.);
    verify(&enlargement.removed, 3.);
    assert_eq!(before, format!("{:?}", retained.solid()));
    for body in [&drilled.kept, &enlargement.kept, &enlargement.removed] {
        let mesh = body.tessellate(0.2, policy()).unwrap();
        assert_eq!(
            format!("{:?}", mesh),
            format!("{:?}", body.tessellate(0.2, policy()).unwrap())
        );
        assert!(body.tessellate(1e-12, policy()).is_err());
    }
}
