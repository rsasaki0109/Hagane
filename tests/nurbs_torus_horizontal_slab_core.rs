use hagane::*;
fn integral(lo: f64, hi: f64) -> f64 {
    let steps = 512;
    let h = (hi - lo) / steps as f64;
    let mut sum = 0.;
    for i in 0..=steps {
        let z = lo + h * i as f64;
        let w = if i == 0 || i == steps {
            1.
        } else if i % 2 == 0 {
            2.
        } else {
            4.
        };
        sum += w * ((4. - z) * (4. + z)).sqrt();
    }
    4. * std::f64::consts::PI * 16. * h * sum / 3.
}
#[test]
fn actual_two_chain_slabs_are_closed_and_thin_positive_volumes_stay_resolved() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let before = source.export_step_mm(p).unwrap();
    for heights in [
        [-2., 2.],
        [0., 2.],
        [-2., 0.],
        [1., 3.],
        [-3., -1.],
        [3.998, 3.999],
        [1., 1.00002],
    ] {
        let slab = source.slab_between_local_z(heights, p).unwrap();
        let body = slab.solid();
        let caps = &body.shell.faces[body.shell.faces.len() - 2..];
        assert_eq!(caps[0].orientation, -1);
        assert_eq!(caps[1].orientation, 1);
        assert!(caps.iter().all(|f| f.wires.len() == 2));
        assert_eq!(
            body.vertices.len() as isize - body.edges.len() as isize
                + body.shell.faces.len() as isize
                - 2,
            0
        );
        let volume = slab.volume(p).unwrap();
        assert!((volume / integral(heights[0], heights[1]) - 1.).abs() < 1e-9);
        for face in &body.shell.faces[..body.shell.faces.len() - 2] {
            let Surface::Nurbs(s) = &face.surface else {
                panic!()
            };
            let domain = s.domain();
            for t in [0., 0.37, 0.82, 1.] {
                let point = s
                    .evaluate(t, domain[1][0] + t * (domain[1][1] - domain[1][0]))
                    .unwrap();
                assert!(((point.x.hypot(point.y) - 16.).hypot(point.z) - 4.).abs() < 1e-11);
                assert!(point.z >= heights[0] - 1e-11 && point.z <= heights[1] + 1e-11);
            }
        }
        assert!(slab
            .export_step_mm(p)
            .unwrap()
            .contains("RATIONAL_B_SPLINE_SURFACE"));
    }
    for heights in [[1., 1.], [-4., 1.], [-1e-8, 2.], [1., 1. + 1e-7]] {
        assert!(source.slab_between_local_z(heights, p).is_err());
    }
    assert_eq!(before, source.export_step_mm(p).unwrap());
}

#[test]
fn partial_meridians_keep_exact_uv_endpoints_and_bounds_include_actual_caps() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let pose = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let frame = Frame3::new_with_tolerance(Point3::new(12., -5., 8.), pose.axes(), p).unwrap();
    for frame in [Frame3::IDENTITY, frame] {
        let source = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
        for heights in [[0.1, 0.3], [-0.3, -0.1]] {
            let slab = source.slab_between_local_z(heights, p).unwrap();
            slab.validate(p).unwrap();
            let bounds = slab.bounds(p).unwrap();
            let inside = |point: Point3| {
                assert!(
                    point.x >= bounds.min.x
                        && point.x <= bounds.max.x
                        && point.y >= bounds.min.y
                        && point.y <= bounds.max.y
                        && point.z >= bounds.min.z
                        && point.z <= bounds.max.z
                );
            };
            for face in &slab.solid().shell.faces {
                if let Surface::Nurbs(s) = &face.surface {
                    let domain = s.domain();
                    for c in &face.wires[0].coedges {
                        for t in [0., 1.] {
                            let uv = c.pcurve.try_evaluate(t).unwrap();
                            assert!((domain[0][0]..=domain[0][1]).contains(&uv[0]));
                            assert!((domain[1][0]..=domain[1][1]).contains(&uv[1]));
                            face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                        }
                    }
                } else {
                    for c in face.wires.iter().flat_map(|w| &w.coedges) {
                        let PCurve::Nurbs(curve) = &c.pcurve else {
                            panic!()
                        };
                        for p in curve.control_points() {
                            inside(face.surface.try_evaluate(p.x, p.y).unwrap());
                        }
                        for i in 0..=64 {
                            let uv = c.pcurve.try_evaluate(i as f64 / 64.).unwrap();
                            inside(face.surface.try_evaluate(uv[0], uv[1]).unwrap());
                        }
                    }
                }
            }
            for p in slab.tessellate(0.1, p).unwrap().positions {
                inside(p);
            }
            assert!(slab
                .export_step_mm(p)
                .unwrap()
                .contains("RATIONAL_B_SPLINE_CURVE"));
        }
    }
}
