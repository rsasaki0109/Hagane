use hagane::*;

#[test]
fn small_coordinates_and_thin_child_preserve_own_scale_volume() {
    for scale in [1., 1e-5] {
        let tolerance = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
        let source = make_box(
            BoxSpec {
                min: Point3::new(-40. * scale, -30. * scale, 0.),
                size: Vec3::new(80. * scale, 60. * scale, 20. * scale),
            },
            tolerance.absolute(),
        )
        .unwrap();
        let rounded = fillet_parallel_box_edges(
            &source,
            &[
                (8, 8. * scale),
                (9, 8. * scale),
                (10, 8. * scale),
                (11, 8. * scale),
            ],
            tolerance,
        )
        .unwrap();
        for offset in [34., 39.99] {
            let plane = Surface::Plane {
                origin: Point3::new(offset * scale, 0., 0.),
                u: Vec3::new(0., 0., 1.),
                v: Vec3::new(0., -1., 0.),
            };
            let split =
                split_normal_arc_line_prism_by_plane(rounded.solid(), &plane, tolerance).unwrap();
            let d = offset - 32.;
            // Independent circular-segment integral, evaluated on unscaled stock.
            let circular_area = 64. * (d / 8.).acos() - d * (64. - d * d).sqrt();
            let expected = (44. * (40. - offset) + circular_area) * 20. * scale.powi(3);
            let actual = split.positive().volume().unwrap();
            assert!((actual - expected).abs() <= 32768. * f64::EPSILON * expected);
            assert!(actual > 0.);
            for body in [split.negative(), split.positive()] {
                body.validate(tolerance.absolute()).unwrap();
                assert!(body.vertices.iter().all(|vertex| vertex.point.finite()));
                let step = export_step_bounded_analytic_mm(body, tolerance.linear()).unwrap();
                let imported =
                    import_step_bounded_analytic_mm(&step, tolerance.absolute()).unwrap();
                assert!(
                    (imported.volume().unwrap() - body.volume().unwrap()).abs()
                        <= 32768. * f64::EPSILON * body.volume().unwrap()
                );
            }
        }
    }
}
