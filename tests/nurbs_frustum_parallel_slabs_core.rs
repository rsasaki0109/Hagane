use hagane::*;
fn plane(z: f64) -> Surface {
    let n = Vec3::new(0.2, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: n.cross(u),
    }
}
#[test]
fn repeated_two_cut_caps_preserve_canonical_and_annular_closed_bodies() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let initial = source.split_by_plane(&plane(18.), p).unwrap();
    let split = initial
        .lower
        .split_by_parallel_plane(&plane(12.), p)
        .unwrap();
    let repeated = split.upper.split_by_parallel_plane(&plane(15.), p).unwrap();
    for (body, height) in [
        (&split.lower, 12.),
        (&repeated.lower, 3.),
        (&repeated.upper, 3.),
    ] {
        assert!((body.volume(p).unwrap() - std::f64::consts::PI * 144. * height).abs() < 1e-8);
        assert_eq!(
            (
                body.solid().vertices.len(),
                body.solid().edges.len(),
                body.solid().shell.faces.len()
            ),
            (8, 12, 6)
        );
        body.export_step_mm(p).unwrap();
    }
    let source = source.bore_coaxial(4., p).unwrap();
    let first = source.split_by_plane(&plane(18.), p).unwrap();
    let split = first.lower.split_by_parallel_plane(&plane(12.), p).unwrap();
    let repeated = split.upper.split_by_parallel_plane(&plane(15.), p).unwrap();
    for body in [&repeated.lower, &repeated.upper] {
        assert!((body.volume(p).unwrap() - std::f64::consts::PI * 128. * 3.).abs() < 1e-8);
        assert_eq!(
            (
                body.solid().vertices.len(),
                body.solid().edges.len(),
                body.solid().shell.faces.len()
            ),
            (16, 24, 10)
        );
        for face in &body.solid().shell.faces {
            for wire in &face.wires {
                for c in &wire.coedges {
                    for t in [0., 0.3, 0.7, 1.] {
                        let uv = c.pcurve.try_evaluate(t).unwrap();
                        let represented = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                        assert!(
                            (represented
                                - body.solid().edges[c.edge].curve.try_evaluate(t).unwrap())
                            .norm()
                                < 1e-10
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn exact_basis_and_intersection_clearance_refuse_without_parent_changes() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let parent = source.split_by_plane(&plane(18.), p).unwrap().lower;
    let original = parent.export_step_mm(p).unwrap();
    assert!(parent.split_by_parallel_plane(&plane(18.), p).is_err());
    assert!(parent
        .split_by_parallel_plane(&plane(18. + 1e-6), p)
        .is_err());
    let Surface::Plane { origin, u, v } = plane(12.) else {
        unreachable!()
    };
    assert!(parent
        .split_by_parallel_plane(
            &Surface::Plane {
                origin,
                u: u * 1.000000000000001,
                v
            },
            p
        )
        .is_err());
    assert_eq!(parent.export_step_mm(p).unwrap(), original);
    parent.split_by_parallel_plane(&plane(12.), p).unwrap();
}
