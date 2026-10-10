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
    assert!((volume - s.volume().unwrap()).abs() < 1e-9);
}

fn review_partition(source: &Solid, edge: usize, setback: f64, removed_volume: f64) {
    let result =
        chamfer_straight_convex_edge(source, edge, setback, GeometryTolerance::default()).unwrap();
    assert!((result.removed().volume().unwrap() - removed_volume).abs() < 1e-9);
    assert!(
        (result.solid().volume().unwrap() + removed_volume - source.volume().unwrap()).abs() < 1e-9
    );
    for body in [result.solid(), result.removed()] {
        review_mesh(body);
        let imported = import_step_mm(
            &export_step_mm(body, Tolerance::default()).unwrap(),
            Tolerance::default(),
        )
        .unwrap();
        review_mesh(&imported);
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-9);
        let patches = planar_face_patches(body, Tolerance::default()).unwrap();
        let seam = patches
            .iter()
            .find(|patch| {
                patch.rings[0].len() == result.bevel().rings[0].len()
                    && result.bevel().rings[0]
                        .iter()
                        .all(|p| patch.rings[0].iter().any(|q| (*p - *q).norm() < 1e-11))
            })
            .expect("both actual BReps retain the same finite cut patch");
        assert_eq!(seam.rings.len(), 1);
        let center = result.bevel().rings[0]
            .iter()
            .fold(Vec3::new(0., 0., 0.), |a, p| {
                a + (*p - Point3::new(0., 0., 0.))
            })
            * (1.0 / result.bevel().rings[0].len() as f64);
        assert_eq!(
            classify_point_in_solid(
                body,
                Point3::new(center.x, center.y, center.z),
                GeometryTolerance::default()
            )
            .unwrap(),
            PointLocation::Boundary
        );
    }
}

#[test]
fn box_bevel_actual_mesh_partition_and_native_step() {
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 5., 6.),
        },
        Tolerance::default(),
    )
    .unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|&i| source.vertices[i].point.x == 8. && source.vertices[i].point.y == 0.)
        })
        .unwrap();
    review_partition(&source, edge, 1., 3.);
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    review_partition(
        &source.transformed(transform, Tolerance::default()).unwrap(),
        edge,
        1.,
        3.,
    );
}

#[test]
fn nonorthogonal_faces_have_equal_physical_setback() {
    let source = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [8., 0.], [2., 5.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 6.),
        Tolerance::default(),
    )
    .unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|&i| source.vertices[i].point.x == 0. && source.vertices[i].point.y == 0.)
        })
        .unwrap();
    review_partition(&source, edge, 0.5, 3. * 0.25 * 5. / 29_f64.sqrt());
}
