use hagane::*;
fn source() -> Solid {
    let t = Tolerance::new(1e-6).unwrap();
    extrude_arc_line(
        &rounded_rectangle_profile(Point3::new(0., 0., 0.), 12., 10., 1., t).unwrap(),
        5.,
        t,
    )
    .unwrap()
}
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 1e-10).unwrap()
}
fn check(body: &Solid) {
    let t = Tolerance::new(1e-6).unwrap();
    body.validate(t).unwrap();
    for f in &body.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            let e = &body.edges[c.edge];
            let r = e.curve.range();
            for t in [0., 0.17, 0.63, 1.] {
                let p = r[0] + t * (r[1] - r[0]);
                let uv = c.pcurve.evaluate(p);
                assert!((e.curve.evaluate(p) - f.surface.evaluate(uv[0], uv[1])).norm() < 1e-6);
            }
        }
    }
}
#[test]
fn exact_normal_bore_keeps_closed_curved_stock_and_actual_removed_solid() {
    let s = source();
    let before = s.volume().unwrap();
    let bore =
        bore_normal_arc_line_prism(&s, Point3::new(1., -0.5, 37.), 0.7, tolerance()).unwrap();
    check(bore.kept());
    check(bore.removed());
    let removed = std::f64::consts::PI * 0.7f64.powi(2) * 5.;
    assert!((bore.direct_removed_volume() - removed).abs() < 1e-12);
    assert!((bore.removed().volume().unwrap() - removed).abs() < 1e-12);
    assert!((bore.kept().volume().unwrap() - (before - removed)).abs() < 1e-10);
    assert_eq!(bore.kept().shell.faces[0].wires.len(), 2);
    assert_eq!(bore.hole_faces().len(), 4);
    for &i in bore.hole_faces() {
        assert!(matches!(
            bore.kept().shell.faces[i].surface,
            Surface::FramedCylinder { .. }
        ));
    }
    assert_eq!(s.shell.faces[0].wires.len(), 1);
}
#[test]
fn every_parallel_axis_and_pose_preserve_real_bore_geometry() {
    let t = Tolerance::new(1e-6).unwrap();
    let box_body = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 6., 4.),
        },
        t,
    )
    .unwrap();
    for (edges, center) in [
        ([0, 2, 4, 6], Point3::new(4., 3., 2.)),
        ([1, 3, 5, 7], Point3::new(4., 3., 2.)),
        ([8, 9, 10, 11], Point3::new(4., 3., 2.)),
    ] {
        let s = fillet_parallel_box_edges(&box_body, &edges.map(|i| (i, 0.3)), tolerance())
            .unwrap()
            .into_solid();
        let transform = Transform::translation(Vec3::new(7., -3., 11.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
            .unwrap();
        let s = s.transformed(transform, t).unwrap();
        let bore =
            bore_normal_arc_line_prism(&s, transform.point(center), 0.4, tolerance()).unwrap();
        check(bore.kept());
        check(bore.removed());
        for body in [bore.kept(), bore.removed()] {
            let text = export_step_bounded_analytic_mm(body, t.linear).unwrap();
            let imported = import_step_bounded_analytic_mm(&text, t).unwrap();
            assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-9);
        }
    }
}
#[test]
fn sequential_holes_reject_overlap_contact_void_and_invalid_inputs() {
    let s = source();
    let first = bore_normal_arc_line_prism(&s, Point3::new(-2., 0., 0.), 0.7, tolerance()).unwrap();
    let second =
        bore_normal_arc_line_prism(first.kept(), Point3::new(2., 0., 5.), 0.8, tolerance())
            .unwrap();
    check(second.kept());
    assert_eq!(second.kept().shell.faces[0].wires.len(), 3);
    for (center, radius) in [
        (Point3::new(-2., 0., 3.), 0.2),
        (Point3::new(-0.6, 0., 0.), 0.7),
        (Point3::new(5.3, 0., 0.), 0.7),
        (Point3::new(8., 0., 0.), 0.4),
        (Point3::new(f64::NAN, 0., 0.), 0.5),
        (Point3::new(0., 0., 1e12), 0.5),
        (Point3::new(0., 0., 0.), 0.),
        (Point3::new(0., 0., 0.), f64::NAN),
        (Point3::new(0., 0., 0.), 1e-7),
    ] {
        assert!(bore_normal_arc_line_prism(first.kept(), center, radius, tolerance()).is_err());
    }
}

#[test]
fn resolved_micro_and_large_models_and_skew_rejection() {
    for scale in [1e-100, 1e100] {
        let t = GeometryTolerance::new(1e-8 * scale, 1e-10, 0.).unwrap();
        let profile = rounded_rectangle_profile(
            Point3::new(0., 0., 0.),
            12. * scale,
            10. * scale,
            scale,
            t.absolute(),
        )
        .unwrap();
        let source = extrude_arc_line(&profile, 5. * scale, t.absolute()).unwrap();
        let bore =
            bore_normal_arc_line_prism(&source, Point3::new(0., 0., 20. * scale), 0.7 * scale, t)
                .unwrap();
        let expected = std::f64::consts::PI * 0.49 * 5. * scale * scale * scale;
        assert!((bore.direct_removed_volume() / expected - 1.).abs() < 1e-12);
        assert!((bore.removed().volume().unwrap() / expected - 1.).abs() < 1e-10);
    }
    let t = tolerance();
    let profile = ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: rounded_rectangle_profile(Point3::new(0., 0., 0.), 12., 10., 1., t.absolute())
            .unwrap()
            .segments,
        holes: vec![],
    };
    let source =
        extrude_arc_line_region_along(&profile, Vec3::new(1., 0., 5.), t.absolute()).unwrap();
    assert!(matches!(
        bore_normal_arc_line_prism(&source, Point3::new(0., 0., 0.), 0.5, t),
        Err(Error::Unsupported(_))
    ));
}
