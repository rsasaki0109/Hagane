use hagane::*;
fn plane(frame: Frame3) -> Surface {
    let n = Vec3::new(0.1, 0.05, 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., -0.05).normalized().unwrap();
    Surface::Plane {
        origin: frame.point(Point3::new(0., 0., 12.)),
        u: frame.vector(u),
        v: frame.vector(n.cross(u)),
    }
}
#[test]
fn actual_annular_caps_reproject_inner_rims_and_conserve_positive_volume() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for radii in [[16., 8.], [12., 12.]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., p)
            .unwrap()
            .bore_coaxial(4., p)
            .unwrap();
        let split = source.split_by_plane(&plane(Frame3::IDENTITY), p).unwrap();
        assert!(
            (split.lower.volume(p).unwrap() + split.upper.volume(p).unwrap()
                - source.volume(p).unwrap())
            .abs()
                < 1e-8
        );
        for child in [&split.lower, &split.upper] {
            assert_eq!(
                (
                    child.solid().vertices.len(),
                    child.solid().edges.len(),
                    child.solid().shell.faces.len()
                ),
                (16, 24, 10)
            );
            for face in &child.solid().shell.faces {
                for wire in &face.wires {
                    for c in &wire.coedges {
                        for t in [0., 0.2, 0.6, 1.] {
                            let uv = c.pcurve.try_evaluate(t).unwrap();
                            let actual = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                            let curve = child.solid().edges[c.edge].curve.try_evaluate(t).unwrap();
                            assert!((actual - curve).norm() < 1e-10);
                        }
                    }
                }
            }
            assert!(child.solid().shell.faces[6..]
                .iter()
                .all(|f| f.orientation == -1));
            child.export_step_mm(p).unwrap();
        }
    }
}
#[test]
fn actual_pose_plane_reversal_and_invalid_cut_preserve_source() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let r = Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), r.axes(), p).unwrap();
    let source = NurbsFrustumSolid::new(frame, [16., 8.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let original = source.export_step_mm(p).unwrap();
    let cut = plane(frame);
    let a = source.split_by_plane(&cut, p).unwrap();
    let Surface::Plane { origin, u, v } = cut else {
        unreachable!()
    };
    let b = source
        .split_by_plane(
            &Surface::Plane {
                origin,
                u,
                v: v * -1.,
            },
            p,
        )
        .unwrap();
    assert!((a.lower.volume(p).unwrap() - b.lower.volume(p).unwrap()).abs() < 1e-8);
    assert!(source
        .split_by_plane(
            &Surface::Plane {
                origin: frame.origin(),
                u: frame.axes()[0],
                v: frame.axes()[1]
            },
            p
        )
        .is_err());
    assert_eq!(source.export_step_mm(p).unwrap(), original);
}
