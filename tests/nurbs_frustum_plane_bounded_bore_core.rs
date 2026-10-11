use hagane::*;
fn plane(z: f64, n: Vec3) -> Surface {
    let normal = n.normalized().unwrap();
    let u = Vec3::new(-normal.y, normal.x, 0.).normalized().unwrap();
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: normal.cross(u),
    }
}
#[test]
fn actual_removed_cylinder_and_ring_share_exact_interfaces_and_original_void() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let stock = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let body = stock
        .split_by_plane(&plane(12., Vec3::new(0.2, 0., 1.)), p)
        .unwrap()
        .lower
        .split_by_interior_plane(&plane(6., Vec3::new(-0.1, 0.1, 1.)), p)
        .unwrap()
        .upper;
    let before = format!("{:?}", body.solid());
    let first = body.partition_coaxial_bore(3., p).unwrap();
    assert_eq!(before, format!("{:?}", first.kept.outer().solid()));
    assert!((first.removed.volume(p).unwrap() - std::f64::consts::PI * 9. * 6.).abs() < 1e-8);
    assert_eq!(
        format!("{:?}", first.kept.inner().solid()),
        format!("{:?}", first.removed.solid())
    );
    let old_inner = format!("{:?}", first.kept.inner().solid());
    let enlarged = first
        .kept
        .partition_coaxial_bore_enlargement(5., p)
        .unwrap();
    assert_eq!(before, format!("{:?}", enlarged.kept.outer().solid()));
    assert_eq!(old_inner, format!("{:?}", enlarged.removed.inner().solid()));
    assert_eq!(
        format!("{:?}", enlarged.kept.inner().solid()),
        format!("{:?}", enlarged.removed.outer().solid())
    );
    assert!((enlarged.removed.volume(p).unwrap() - std::f64::consts::PI * 16. * 6.).abs() < 1e-8);
    for q in 0..4 {
        let a = &enlarged.kept.solid().shell.faces[6 + q];
        let b = &enlarged.removed.solid().shell.faces[2 + q];
        assert_eq!(format!("{:?}", a.surface), format!("{:?}", b.surface));
        assert_eq!(a.orientation, -b.orientation);
    }
    for shape in [enlarged.kept.solid(), enlarged.removed.solid()] {
        assert_eq!(
            (
                shape.vertices.len(),
                shape.edges.len(),
                shape.shell.faces.len()
            ),
            (16, 24, 10)
        );
        let mut uses = [(0, 0); 24];
        for face in &shape.shell.faces {
            for wire in &face.wires {
                for c in &wire.coedges {
                    uses[c.edge].0 += 1;
                    uses[c.edge].1 += i32::from(face.orientation) * if c.forward { 1 } else { -1 };
                    for k in 0..17 {
                        let t = k as f64 / 16.;
                        let uv = c.pcurve.try_evaluate(t).unwrap();
                        assert!(
                            (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                                - shape.edges[c.edge].curve.try_evaluate(t).unwrap())
                            .norm()
                                < 1e-10
                        );
                    }
                }
            }
        }
        assert!(uses.iter().all(|&u| u == (2, 0)));
    }
    let total = enlarged.kept.volume(p).unwrap()
        + enlarged.removed.volume(p).unwrap()
        + first.removed.volume(p).unwrap();
    assert!((total - body.volume(p).unwrap()).abs() < 1e-8);
    assert!(matches!(
        body.bore_coaxial(10., p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        first.kept.enlarge_coaxial_bore(3., p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        first.kept.enlarge_coaxial_bore(2., p),
        Err(Error::Unsupported(_))
    ));
    let relative = GeometryTolerance::new(1e-6, 1e-10, 1e-4).unwrap();
    assert!(matches!(
        first.kept.enlarge_coaxial_bore(3.03, relative),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        first.kept.enlarge_coaxial_bore(f64::NAN, p),
        Err(Error::InvalidInput(_))
    ));
    assert_eq!(before, format!("{:?}", body.solid()));
    assert_eq!(old_inner, format!("{:?}", first.kept.inner().solid()));
}
