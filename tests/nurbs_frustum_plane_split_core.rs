use hagane::*;
fn plane(z: f64, tilt: f64) -> Surface {
    let n = Vec3::new(tilt, 0., 1.).normalized().unwrap();
    let u = Vec3::new(0., 1., 0.);
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: n.cross(u),
    }
}
#[test]
fn real_closed_pieces_preserve_source_caps_and_actual_shared_rational_cut() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        let source = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 24., p).unwrap();
        let split = source.split_by_plane(&plane(12., 0.1), p).unwrap();
        for child in [&split.lower, &split.upper] {
            child.validate(p).unwrap();
            assert_eq!(child.solid().vertices.len(), 8);
            assert_eq!(child.solid().edges.len(), 12);
            assert_eq!(child.solid().shell.faces.len(), 6);
            assert!(child
                .export_step_mm(p)
                .unwrap()
                .contains("RATIONAL_B_SPLINE_SURFACE"));
            assert!(child.solid().validate(p.absolute()).is_err());
        }
        assert!(
            (split.lower.volume(p).unwrap() + split.upper.volume(p).unwrap()
                - source.volume(p).unwrap())
            .abs()
                < 1e-9
        );
        for q in 0..4 {
            assert_eq!(
                format!("{:?}", split.lower.solid().edges[q]),
                format!("{:?}", source.solid().edges[q])
            );
            let (Curve::Nurbs(a), Curve::Nurbs(b)) = (
                &split.lower.solid().edges[4 + q].curve,
                &split.upper.solid().edges[q].curve,
            ) else {
                panic!()
            };
            assert_eq!(a.control_points(), b.control_points());
            assert_eq!(a.weights(), b.weights());
        }
        let horizontal = source.split_by_plane(&plane(6., 0.), p).unwrap();
        let axial = source.split_axial(6., p).unwrap();
        assert!(
            (horizontal.lower.volume(p).unwrap() - axial.lower.volume(p).unwrap()).abs() < 1e-8
        );
    }
}
#[test]
fn true_generator_parameters_match_every_actual_face_and_cylinder_tilt_volume() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let split = source.split_by_plane(&plane(10., 0.2), p).unwrap();
    assert!((split.lower.volume(p).unwrap() - std::f64::consts::PI * 144. * 10.).abs() < 1e-9);
    for child in [&split.lower, &split.upper] {
        for face in &child.solid().shell.faces {
            for c in &face.wires[0].coedges {
                for i in 0..101 {
                    let t = i as f64 / 100.;
                    let uv = c.pcurve.try_evaluate(t).unwrap();
                    let a = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                    let b = child.solid().edges[c.edge].curve.try_evaluate(t).unwrap();
                    assert!((a - b).norm() < 1e-10);
                }
            }
        }
    }
    assert!(source.split_by_plane(&plane(0., 0.2), p).is_err());
}
