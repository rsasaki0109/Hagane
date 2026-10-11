use hagane::*;
#[test]
fn horizontal_restrictions_keep_source_and_close_both_annular_caps() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let before = source.export_step_mm(p).unwrap();
    for cut in [0., 1.5, -1.5, 3.9, -3.9] {
        let split = source.split_by_local_z(cut, p).unwrap();
        assert!(
            ((split.lower.volume(p).unwrap() + split.upper.volume(p).unwrap())
                / source.volume(p).unwrap()
                - 1.)
                .abs()
                < 1e-12
        );
        for child in [split.lower, split.upper] {
            let s = child.solid();
            assert_eq!(
                s.vertices.len() as isize - s.edges.len() as isize + s.shell.faces.len() as isize
                    - 1,
                0
            );
            assert_eq!(s.shell.faces.last().unwrap().wires.len(), 2);
            for face in &s.shell.faces[..s.shell.faces.len() - 1] {
                let Surface::Nurbs(surface) = &face.surface else {
                    panic!()
                };
                let domain = surface.domain();
                for t in [0., 0.25, 0.75, 1.] {
                    let point = surface
                        .evaluate(t, domain[1][0] + t * (domain[1][1] - domain[1][0]))
                        .unwrap();
                    assert!(((point.x.hypot(point.y) - 16.).hypot(point.z) - 4.).abs() < 1e-11);
                    assert!(if child.is_lower() {
                        point.z <= cut + 1e-11
                    } else {
                        point.z >= cut - 1e-11
                    });
                }
            }
            assert!(child
                .export_step_mm(p)
                .unwrap()
                .contains("RATIONAL_B_SPLINE_SURFACE"));
        }
    }
    for cut in [4., -4., 1e-8, f64::NAN] {
        assert!(source.split_by_local_z(cut, p).is_err());
    }
    assert_eq!(before, source.export_step_mm(p).unwrap());
}
