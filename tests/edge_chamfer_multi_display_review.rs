use hagane::*;
use std::collections::BTreeMap;

// Weld only to uniquely identified actual BRep vertices, within an explicit
// floating arithmetic allowance. This does not certify bitwise mesh welding.
fn review_mesh(s: &Solid) {
    let mesh = s.tessellate(0.01, Tolerance::default()).unwrap();
    let world = s.vertices.iter().fold(1_f64, |a, v| {
        a.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let guard = 4096.0 * f64::EPSILON * world;
    let ids: Vec<_> = mesh
        .positions
        .iter()
        .map(|p| {
            let candidates: Vec<_> = s
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| (v.point - *p).norm() <= guard)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(
                candidates.len(),
                1,
                "mesh point must identify one actual vertex"
            );
            candidates[0]
        })
        .collect();
    let mut incidence = BTreeMap::new();
    let mut volume = 0.0;
    for (tri, &face_id) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        let face = &s.shell.faces[face_id];
        let Surface::Plane { origin, u, v } = face.surface else {
            panic!("planar chamfer")
        };
        let normal = u.cross(v).normalized().unwrap() * f64::from(face.orientation);
        for point in p {
            assert!((point - origin).dot(normal).abs() <= guard);
        }
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(normal) > 0.0);
        for i in 0..3 {
            let a = ids[tri[i]];
            let b = ids[tri[(i + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let entry = incidence.entry(key).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += sign;
        }
        volume += (p[0] - Point3::new(0., 0., 0.))
            .dot((p[1] - Point3::new(0., 0., 0.)).cross(p[2] - Point3::new(0., 0., 0.)))
            / 6.0;
    }
    assert!(incidence
        .values()
        .all(|&(n, direction)| n == 2 && direction == 0));
    assert!((volume - s.volume().unwrap()).abs() < 1e-7);
}

fn area(ring: &[Point3]) -> f64 {
    let origin = ring[0];
    (1..ring.len() - 1)
        .map(|i| (ring[i] - origin).cross(ring[i + 1] - origin).norm() * 0.5)
        .sum()
}

fn plane_area(body: &Solid, family: usize) -> f64 {
    planar_face_patches(body, Tolerance::default())
        .unwrap()
        .iter()
        .filter(|patch| {
            patch.rings[0].iter().all(|p| {
                let residual = if family == 0 {
                    p.y + p.z - 3.
                } else {
                    p.x + p.y - 5.
                };
                residual.abs() < 1e-10
            })
        })
        .map(|patch| area(&patch.rings[0]))
        .sum()
}

#[test]
fn interacting_bevel_meshes_have_final_faces_and_disjoint_analytic_partition() {
    let tol = GeometryTolerance::default();
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tol.absolute(),
    )
    .unwrap();
    let result = chamfer_straight_convex_edges(&source, &[(0, 3.), (8, 5.)], tol).unwrap();
    assert_eq!(result.selections(), &[(0, 3.), (8, 5.)]);
    assert!((result.removed()[0].volume().unwrap() - 360.).abs() < 1e-7);
    assert!((result.removed()[1].volume().unwrap() - 232.).abs() < 1e-7);
    assert!((result.solid().volume().unwrap() - 95408.).abs() < 1e-7);
    for (i, expected) in [229.5 * 2_f64.sqrt(), 95.5 * 2_f64.sqrt()]
        .into_iter()
        .enumerate()
    {
        assert!((area(&result.bevels()[i].rings[0]) - expected).abs() < 1e-8);
        assert!((plane_area(result.solid(), i) - expected).abs() < 1e-8);
    }
    // The first removed piece keeps the whole original cut patch. Its seam
    // is partitioned between the final retained bevel and the second removal.
    assert!((plane_area(&result.removed()[0], 0) - 240. * 2_f64.sqrt()).abs() < 1e-8);
    assert!(
        (plane_area(result.solid(), 0) + plane_area(&result.removed()[1], 0)
            - plane_area(&result.removed()[0], 0))
        .abs()
            < 1e-8
    );
    assert!((plane_area(result.solid(), 1) - plane_area(&result.removed()[1], 1)).abs() < 1e-8);
    for body in std::iter::once(result.solid()).chain(result.removed()) {
        review_mesh(body);
        let imported = import_step_mm(
            &export_step_mm(body, tol.absolute()).unwrap(),
            tol.absolute(),
        )
        .unwrap();
        review_mesh(&imported);
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-7);
    }
    for x in [0.25, 2., 6., 40.] {
        for y in [0.25, 1., 4., 30.] {
            for z in [0.25, 1., 4., 10.] {
                let p = Point3::new(x, y, z);
                let expected = [
                    y + z > 3. && x + y > 5.,
                    y + z < 3.,
                    y + z > 3. && x + y < 5.,
                ];
                assert_eq!(expected.iter().filter(|&&inside| inside).count(), 1);
                for (body, inside) in std::iter::once(result.solid())
                    .chain(result.removed())
                    .zip(expected)
                {
                    assert_eq!(
                        classify_point_in_solid(body, p, tol).unwrap(),
                        if inside {
                            PointLocation::Inside
                        } else {
                            PointLocation::Outside
                        }
                    );
                }
            }
        }
    }
    for body in [result.solid(), &result.removed()[1]] {
        assert_eq!(
            classify_point_in_solid(body, Point3::new(4., 1., 8.), tol).unwrap(),
            PointLocation::Boundary
        );
    }
}
