use hagane::*;
use std::f64::consts::PI;
fn entry(face: BoxFace, b: BoxSpec) -> (Point3, Vec3) {
    let mut p = b.min + b.size * 0.5;
    let n = match face {
        BoxFace::MaxZ => {
            p.z = b.min.z + b.size.z;
            Vec3::new(0., 0., 1.)
        }
        BoxFace::MinZ => {
            p.z = b.min.z;
            Vec3::new(0., 0., -1.)
        }
        BoxFace::MaxX => {
            p.x = b.min.x + b.size.x;
            Vec3::new(1., 0., 0.)
        }
        BoxFace::MinX => {
            p.x = b.min.x;
            Vec3::new(-1., 0., 0.)
        }
        BoxFace::MaxY => {
            p.y = b.min.y + b.size.y;
            Vec3::new(0., 1., 0.)
        }
        BoxFace::MinY => {
            p.y = b.min.y;
            Vec3::new(0., -1., 0.)
        }
    };
    (p, n)
}
#[test]
fn six_entry_faces_preserve_bounds_volume_material_floor_normals_and_closed_meshes() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = Tolerance::new(epsilon).unwrap();
        let policy = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        let b = BoxSpec {
            min: Point3::new(3., -2., 4.) * scale,
            size: Vec3::new(8., 6., 4.) * scale,
        };
        for id in 0..6 {
            let face = BoxFace::try_from(id).unwrap();
            let (center, n) = entry(face, b);
            let bore = FaceBlindBore {
                center,
                radius: 0.8 * scale,
                depth: 1.5 * scale,
            };
            let solid = subtract_blind_bores_from_face(b, face, &[bore], t).unwrap();
            solid.validate(t).unwrap();
            assert_eq!((solid.shell.faces.len(), solid.edges.len()), (8, 15));
            assert!((solid.bounds().min - b.min).norm() < epsilon);
            assert!((solid.bounds().max - (b.min + b.size)).norm() < epsilon);
            let tangent = if n.x.abs() == 1. {
                Vec3::new(0., 1., 0.)
            } else {
                Vec3::new(1., 0., 0.)
            };
            let bores = [
                FaceBlindBore {
                    center: center - tangent * scale,
                    radius: 0.3 * scale,
                    depth: 0.7 * scale,
                },
                FaceBlindBore {
                    center: center + tangent * scale,
                    radius: 0.4 * scale,
                    depth: 1.1 * scale,
                },
            ];
            let multiple = subtract_blind_bores_from_face(b, face, &bores, t).unwrap();
            multiple.validate(t).unwrap();
            assert_eq!((multiple.shell.faces.len(), multiple.edges.len()), (10, 18));
            let multi_volume = (192. - PI * (0.09 * 0.7 + 0.16 * 1.1)) * scale.powi(3);
            assert!((multiple.volume().unwrap() - multi_volume).abs() < 1e-10 * scale.powi(3));
            let volume = (192. - PI * 0.64 * 1.5) * scale.powi(3);
            assert!((solid.volume().unwrap() - volume).abs() < 1e-10 * scale.powi(3));
            let floor = center - n * bore.depth;
            for angle in [0., 0.37] {
                let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
                let placed = solid.transformed(transform, t).unwrap();
                for (p, expected) in [
                    (center, PointLocation::Outside),
                    (center - n * scale, PointLocation::Outside),
                    (floor, PointLocation::Boundary),
                    (floor - n * 0.25 * scale, PointLocation::Inside),
                    (floor + n * (2. * epsilon), PointLocation::Outside),
                    (floor - n * (2. * epsilon), PointLocation::Inside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&placed, transform.point(p), policy).unwrap(),
                        expected
                    );
                }
            }
            let mesh = solid.tessellate(0.002 * scale, t).unwrap();
            assert!((mesh.signed_volume() - volume).abs() < 0.02 * scale.powi(3));
            let key = |p: Point3| {
                [
                    (p.x / scale * 1e8).round() as i64,
                    (p.y / scale * 1e8).round() as i64,
                    (p.z / scale * 1e8).round() as i64,
                ]
            };
            let mut uses = std::collections::BTreeMap::new();
            let mut floors = 0;
            for (tri, face_id) in mesh.triangles.iter().zip(&mesh.face_ids) {
                let p = tri.map(|i| mesh.positions[i]);
                if *face_id == 7 {
                    floors += 1;
                    assert!(p.iter().all(|p| ((*p - floor).dot(n)).abs() < epsilon));
                    assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(n) > 0.);
                }
                for i in 0..3 {
                    let a = key(p[i]);
                    let b = key(p[(i + 1) % 3]);
                    let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                    let v = uses.entry(edge).or_insert((0, 0));
                    v.0 += 1;
                    v.1 += sign;
                }
            }
            assert!(floors > 8);
            assert!(uses.values().all(|&(count, sign)| count == 2 && sign == 0));
        }
    }
}
#[test]
fn all_faces_reject_wrong_mouth_breakthrough_invalid_dimensions_and_overlapping_tools() {
    let t = Tolerance::default();
    let b = BoxSpec {
        min: Point3::new(-4., -3., -2.),
        size: Vec3::new(8., 6., 4.),
    };
    for id in 0..6 {
        let face = BoxFace::try_from(id).unwrap();
        let (center, n) = entry(face, b);
        let bore = FaceBlindBore {
            center,
            radius: 0.5,
            depth: 1.,
        };
        let thickness = match face {
            BoxFace::MaxX | BoxFace::MinX => 8.,
            BoxFace::MaxY | BoxFace::MinY => 6.,
            _ => 4.,
        };
        for depth in [
            0.,
            -1.,
            thickness,
            thickness - 1e-9,
            f64::NAN,
            f64::INFINITY,
        ] {
            assert!(
                subtract_blind_bores_from_face(b, face, &[FaceBlindBore { depth, ..bore }], t)
                    .is_err()
            );
        }
        for center in [center + n * 1e-6, Point3::new(f64::NAN, 0., 0.)] {
            assert!(subtract_blind_bores_from_face(
                b,
                face,
                &[FaceBlindBore { center, ..bore }],
                t
            )
            .is_err());
        }
        assert!(subtract_blind_bores_from_face(b, face, &[bore, bore], t).is_err());
        assert!(subtract_blind_bores_from_face(
            b,
            face,
            &[FaceBlindBore { radius: 0., ..bore }],
            t
        )
        .is_err());
        let near = FaceBlindBore {
            center: center + n * (0.5 * t.linear),
            ..bore
        };
        subtract_blind_bores_from_face(b, face, &[near], t).unwrap();
        let empty = subtract_blind_bores_from_face(b, face, &[], t).unwrap();
        assert_eq!(
            empty.mesh_json(0.02, t).unwrap(),
            make_box(b, t).unwrap().mesh_json(0.02, t).unwrap()
        );
    }
    assert!(BoxFace::try_from(6).is_err());
}
