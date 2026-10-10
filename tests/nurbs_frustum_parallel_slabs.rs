use hagane::*;
use std::collections::BTreeMap;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn geometry(s: &Solid, holes: bool, scale: f64) {
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        if holes { (16, 24, 10) } else { (8, 12, 6) }
    );
    let mut uses = vec![(0, 0); s.edges.len()];
    for face in &s.shell.faces {
        for c in face.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].0 += 1;
            uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
            for i in 0..=16 {
                let t = i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                let p = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let q = s.edges[c.edge].curve.try_evaluate(t).unwrap();
                assert!((p - q).norm() < 1e-8 * scale);
            }
        }
    }
    assert!(uses.iter().all(|v| *v == (2, 0)));
    assert_eq!(s.shell.faces[0].wires.len(), if holes { 2 } else { 1 });
    assert_eq!(s.shell.faces[1].wires.len(), if holes { 2 } else { 1 });
}
fn moved(plane: &Surface, axis: Vec3, distance: f64) -> Surface {
    let Surface::Plane { origin, u, v } = plane else {
        panic!("plane")
    };
    Surface::Plane {
        origin: *origin + axis * distance,
        u: *u,
        v: *v,
    }
}
fn frame(scale: f64) -> Frame3 {
    let r = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * scale, r.axes(), policy(scale)).unwrap()
}
fn first_plane(frame: Frame3, scale: f64) -> Surface {
    let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
    let u = Vec3::new(-n.y, n.x, 0.).normalized().unwrap();
    let vector = |p: Vec3| frame.axes()[0] * p.x + frame.axes()[1] * p.y + frame.axes()[2] * p.z;
    Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 8. * scale)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-9 * b.abs(), "{a} != {b}");
}
#[test]
fn repeated_canonical_slabs_match_cylinder_oracle_cut_order_pose_and_scale() {
    for scale in [1e-4, 1., 10.] {
        let f = frame(scale);
        let p = policy(scale);
        let source = NurbsFrustumSolid::new(f, [16. * scale, 16. * scale], 24. * scale, p).unwrap();
        let before = format!("{:?}", source.solid());
        let plane = first_plane(f, scale);
        let initial = source.split_by_plane(&plane, p).unwrap();
        let second = moved(&plane, f.axes()[2], 4. * scale);
        let third = moved(&plane, f.axes()[2], 8. * scale);
        let split = initial.upper.split_by_parallel_plane(&second, p).unwrap();
        let again = split.upper.split_by_parallel_plane(&third, p).unwrap();
        let earlier = initial
            .lower
            .split_by_parallel_plane(&moved(&plane, f.axes()[2], -4. * scale), p)
            .unwrap();
        for (part, height) in [
            (&split.lower, 4.),
            (&again.lower, 4.),
            (&again.upper, 8.),
            (&earlier.lower, 4.),
            (&earlier.upper, 4.),
        ] {
            part.validate(p).unwrap();
            geometry(part.solid(), false, scale);
            close(
                part.volume(p).unwrap(),
                std::f64::consts::PI * 256. * height * scale.powi(3),
            );
            let step = part.export_step_mm(p).unwrap();
            assert!(step.contains("MANIFOLD_SOLID_BREP"));
            assert_eq!(step.matches("FACE_BOUND(").count(), 0);
        }
        mesh_closed(&split.lower.tessellate(0.2 * scale, p).unwrap());
        let reverse = initial.upper.split_by_parallel_plane(&third, p).unwrap();
        let reverse = reverse.lower.split_by_parallel_plane(&second, p).unwrap();
        close(
            reverse.lower.volume(p).unwrap(),
            split.lower.volume(p).unwrap(),
        );
        close(
            reverse.upper.volume(p).unwrap(),
            again.lower.volume(p).unwrap(),
        );
        for q in 0..4 {
            for t in [0., 0.23, 0.71, 1.] {
                let a = split.lower.solid().edges[4 + q]
                    .curve
                    .try_evaluate(t)
                    .unwrap();
                let b = again.lower.solid().edges[q].curve.try_evaluate(t).unwrap();
                assert!((a - b).norm() < 1e-9 * scale);
            }
        }
        close(
            earlier.lower.volume(p).unwrap()
                + earlier.upper.volume(p).unwrap()
                + split.lower.volume(p).unwrap()
                + again.lower.volume(p).unwrap()
                + again.upper.volume(p).unwrap(),
            source.volume(p).unwrap(),
        );
        assert_eq!(before, format!("{:?}", source.solid()));
    }
}
#[test]
fn repeated_annular_slabs_preserve_hole_inner_rims_and_tapered_volume() {
    for radii in [[16., 16.], [16., 8.], [8., 16.]] {
        let scale = 1.;
        let f = frame(scale);
        let p = policy(scale);
        let source = NurbsFrustumSolid::new(f, radii, 24., p)
            .unwrap()
            .bore_coaxial(4., p)
            .unwrap();
        let before = format!("{:?}", source.solid());
        let plane = first_plane(f, scale);
        let initial = source.split_by_plane(&plane, p).unwrap();
        let second = moved(&plane, f.axes()[2], 4.);
        let third = moved(&plane, f.axes()[2], 8.);
        let split = initial.upper.split_by_parallel_plane(&second, p).unwrap();
        let again = split.upper.split_by_parallel_plane(&third, p).unwrap();
        for (part, height) in [(&split.lower, 4.), (&again.lower, 4.), (&again.upper, 8.)] {
            part.validate(p).unwrap();
            geometry(part.solid(), true, 1.);
            assert!(part.volume(p).unwrap() > 0.);
            if radii[0] == radii[1] {
                close(
                    part.volume(p).unwrap(),
                    std::f64::consts::PI * (256. - 16.) * height,
                );
            }
            let step = part.export_step_mm(p).unwrap();
            assert_eq!(step.matches("FACE_BOUND(").count(), 2);
        }
        mesh_closed(&split.lower.tessellate(0.2, p).unwrap());
        for (top, bottom) in [(4, 0), (16, 12)] {
            for q in 0..4 {
                for t in [0., 0.23, 0.71, 1.] {
                    let a = split.lower.solid().edges[top + q]
                        .curve
                        .try_evaluate(t)
                        .unwrap();
                    let b = again.lower.solid().edges[bottom + q]
                        .curve
                        .try_evaluate(t)
                        .unwrap();
                    assert!((a - b).norm() < 1e-9);
                }
            }
        }
        close(
            initial.lower.volume(p).unwrap()
                + split.lower.volume(p).unwrap()
                + again.lower.volume(p).unwrap()
                + again.upper.volume(p).unwrap(),
            source.volume(p).unwrap(),
        );
        assert_eq!(before, format!("{:?}", source.solid()));
    }
}
#[test]
fn exact_basis_and_source_policy_contacts_refuse_without_changing_slab() {
    let scale = 1.;
    let f = frame(scale);
    let p = policy(scale);
    let source = NurbsFrustumSolid::new(f, [16., 8.], 24., p).unwrap();
    let plane = first_plane(f, scale);
    let first = source.split_by_plane(&plane, p).unwrap();
    let before = format!("{:?}", first.upper.solid());
    let step = first.upper.export_step_mm(p).unwrap();
    let Surface::Plane { origin, u, v } = moved(&plane, f.axes()[2], 4.) else {
        panic!("plane")
    };
    let changed_u = Vec3::new(f64::from_bits(u.x.to_bits() + 1), u.y, u.z);
    for bad in [
        plane.clone(),
        moved(&plane, f.axes()[2], 1e-7),
        moved(&plane, f.axes()[2], 30.),
        Surface::Plane {
            origin,
            u: changed_u,
            v,
        },
        Surface::Plane { origin, u: v, v: u },
        Surface::Plane {
            origin: Point3::new(f64::NAN, 0., 0.),
            u,
            v,
        },
        Surface::Plane {
            origin: Point3::new(1e12, 1e12, 1e12),
            u,
            v,
        },
    ] {
        assert!(first.upper.split_by_parallel_plane(&bad, p).is_err());
    }
    assert!(first
        .upper
        .split_by_parallel_plane(
            &moved(&plane, f.axes()[2], 0.2),
            GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap()
        )
        .is_err());
    assert_eq!(before, format!("{:?}", first.upper.solid()));
    assert_eq!(step, first.upper.export_step_mm(p).unwrap());
    assert!(first
        .upper
        .split_by_parallel_plane(&moved(&plane, f.axes()[2], 4.), p)
        .is_ok());
}

fn mesh_closed(mesh: &Mesh) {
    let key = |p: Point3| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
    let mut uses = BTreeMap::new();
    for tri in &mesh.triangles {
        for i in 0..3 {
            let a = key(mesh.positions[tri[i]]);
            let b = key(mesh.positions[tri[(i + 1) % 3]]);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let usage = uses.entry(edge).or_insert((0, 0));
            usage.0 += 1;
            usage.1 += sign;
        }
    }
    assert!(uses.values().all(|v| *v == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
}
