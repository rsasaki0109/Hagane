use hagane::*;
use std::collections::BTreeMap;

// Weld only to uniquely identified actual BRep vertices, within an explicit
// floating arithmetic allowance. This does not certify bitwise mesh welding.
fn review_mesh(s: &Solid) {
    let mesh = s.tessellate(0.01, Tolerance::new(1e-6).unwrap()).unwrap();
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

fn expected_material(p: Point3) -> bool {
    let nearest = [p.x.min(80. - p.x), p.y.min(60. - p.y), p.z.min(20. - p.z)];
    nearest.iter().all(|&d| d > 0.)
        && nearest[0] + nearest[1] > 3.
        && nearest[0] + nearest[2] > 3.
        && nearest[1] + nearest[2] > 3.
}

fn fixture(frame: Transform) {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tol.absolute(),
    )
    .unwrap()
    .transformed(frame, tol.absolute())
    .unwrap();
    let before = export_step_mm(&source, tol.absolute()).unwrap();
    let selections: Vec<_> = (0..12).map(|i| (i, 3.)).collect();
    assert!(chamfer_straight_convex_edges(&source, &selections, tol).is_err());
    let result =
        chamfer_straight_convex_edges_with_vertex_contacts(&source, &selections, tol).unwrap();
    assert_eq!(export_step_mm(&source, tol.absolute()).unwrap(), before);
    assert_eq!(
        (
            result.solid().vertices.len(),
            result.solid().edges.len(),
            result.solid().shell.faces.len()
        ),
        (32, 48, 18)
    );
    assert_eq!(result.bevels().len(), 12);
    assert!(result
        .bevels()
        .iter()
        .all(|p| p.rings.len() == 1 && p.rings[0].len() == 6));
    let patches = planar_face_patches(result.solid(), tol.absolute()).unwrap();
    assert_eq!(patches.iter().filter(|p| p.rings[0].len() == 4).count(), 6);
    assert!((result.solid().volume().unwrap() - 93282.).abs() < 1e-7);
    assert!(
        (result
            .removed()
            .iter()
            .map(|s| s.volume().unwrap())
            .sum::<f64>()
            - 2718.)
            .abs()
            < 1e-7
    );
    // Every cut-plane seam has equal area on the two sides of the full
    // retained/removed partition, including later subdivisions of old seams.
    for plane in result.bevel_planes() {
        let Surface::Plane { origin, u, v } = *plane else {
            unreachable!()
        };
        let normal = u.cross(v).normalized().unwrap();
        let mut signed_area = 0.0;
        let mut seam_faces = 0;
        for body in std::iter::once(result.solid()).chain(result.removed()) {
            for patch in planar_face_patches(body, tol.absolute()).unwrap() {
                let ring = &patch.rings[0];
                if !ring.iter().all(|p| (*p - origin).dot(normal).abs() < 1e-9) {
                    continue;
                }
                seam_faces += 1;
                let Surface::Plane { u: pu, v: pv, .. } = patch.surface else {
                    unreachable!()
                };
                let sign = (pu.cross(pv) * f64::from(patch.orientation))
                    .dot(normal)
                    .signum();
                signed_area += sign
                    * (1..ring.len() - 1)
                        .map(|i| (ring[i] - ring[0]).cross(ring[i + 1] - ring[0]).norm() * 0.5)
                        .sum::<f64>();
            }
        }
        assert!(seam_faces >= 2);
        assert!(signed_area.abs() < 1e-7);
    }
    for body in std::iter::once(result.solid()).chain(result.removed()) {
        review_mesh(body);
        let imported = import_step_mm(
            &export_step_mm(body, tol.absolute()).unwrap(),
            tol.absolute(),
        )
        .unwrap();
        review_mesh(&imported);
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-7);
        for face in &body.shell.faces {
            for coedge in &face.wires[0].coedges {
                let curve = &body.edges[coedge.edge].curve;
                let range = curve.range();
                for q in [0., 0.17, 0.53, 0.89, 1.] {
                    let t = range[0] + q * (range[1] - range[0]);
                    let uv = coedge.pcurve.try_evaluate(t).unwrap();
                    assert!(
                        (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                            - curve.try_evaluate(t).unwrap())
                        .norm()
                            < 1e-9
                    );
                }
            }
        }
    }
    for x in [0.4, 1.3, 2.7, 40., 77.3, 78.7, 79.6] {
        for y in [0.4, 1.3, 2.7, 30., 57.3, 58.7, 59.6] {
            for z in [0.4, 1.3, 2.7, 10., 17.3, 18.7, 19.6] {
                let p = Point3::new(x, y, z);
                let expected = expected_material(p);
                let world = frame.point(p);
                assert_eq!(
                    classify_point_in_solid(result.solid(), world, tol).unwrap(),
                    if expected {
                        PointLocation::Inside
                    } else {
                        PointLocation::Outside
                    }
                );
                let removed_inside = result
                    .removed()
                    .iter()
                    .filter(|s| {
                        classify_point_in_solid(s, world, tol).unwrap() == PointLocation::Inside
                    })
                    .count();
                assert_eq!(removed_inside, usize::from(!expected));
            }
        }
    }
    for request in [vec![(0, 3.), (8, 3. + 1e-7)], vec![(0, 3.), (8, 100.)]] {
        assert!(
            chamfer_straight_convex_edges_with_vertex_contacts(&source, &request, tol).is_err()
        );
        assert_eq!(export_step_mm(&source, tol.absolute()).unwrap(), before);
    }
}

#[test]
fn twelve_contact_cuts_have_actual_closed_display_and_independent_halfspaces() {
    fixture(Transform::IDENTITY);
    fixture(
        Transform::translation(Vec3::new(12., -3., 5.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
            .unwrap(),
    );
}
