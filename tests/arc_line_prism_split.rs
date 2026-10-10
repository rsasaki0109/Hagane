use hagane::*;
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 1e-10).unwrap()
}
fn region() -> ArcLineRegion {
    ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: rounded_rectangle_profile(
            Point3::new(0., 0., 0.),
            20.,
            16.,
            2.,
            tolerance().absolute(),
        )
        .unwrap()
        .segments,
        holes: vec![],
    }
}
fn plane(x: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(x, 0., 0.),
        u: Vec3::new(0., 0., 1.),
        v: Vec3::new(0., -1., 0.),
    }
}
fn check(result: &NormalArcLinePrismPlaneSplit, source: &Solid) {
    let mut volume = 0.;
    for body in [result.negative(), result.positive()] {
        body.validate(tolerance().absolute()).unwrap();
        volume += body.volume().unwrap();
        for f in &body.shell.faces {
            for c in f.wires.iter().flat_map(|w| &w.coedges) {
                let e = &body.edges[c.edge];
                let r = e.curve.range();
                for p in [0., 0.23, 0.71, 1.] {
                    let p = r[0] + p * (r[1] - r[0]);
                    let uv = c.pcurve.evaluate(p);
                    assert!((e.curve.evaluate(p) - f.surface.evaluate(uv[0], uv[1])).norm() < 1e-6);
                }
            }
        }
        let text = export_step_bounded_analytic_mm(body, 1e-6).unwrap();
        let imported = import_step_bounded_analytic_mm(&text, tolerance().absolute()).unwrap();
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-9);
    }
    assert!((volume - source.volume().unwrap()).abs() < 1e-9);
    let patch = result.section();
    let Surface::Plane { u, v, .. } = patch.surface else {
        panic!()
    };
    let Surface::Plane { u: pu, v: pv, .. } = *result.plane() else {
        panic!()
    };
    assert!((u.cross(v) * f64::from(patch.orientation)).dot(pu.cross(pv)) > 0.99);
    // The report is an actual retained face, with its actual parameter basis.
    assert!(result
        .negative()
        .shell
        .faces
        .iter()
        .any(
            |f| format!("{:?}", f.surface) == format!("{:?}", patch.surface)
                && f.orientation == patch.orientation
        ));
}
#[test]
fn exact_sides_section_and_signed_normal_extrusion_have_independent_volumes() {
    for height in [5., -5.] {
        let source = extrude_arc_line_region_along(
            &region(),
            Vec3::new(0., 0., height),
            tolerance().absolute(),
        )
        .unwrap();
        for cut in [0., 3.] {
            let result =
                split_normal_arc_line_prism_by_plane(&source, &plane(cut), tolerance()).unwrap();
            check(&result, &source);
            let expected = ((10. - cut) * 16. - (2. - std::f64::consts::FRAC_PI_2) * 4.) * 5.;
            assert!((result.positive().volume().unwrap() - expected).abs() < 1e-9);
        }
    }
}
#[test]
fn untouched_hole_ownership_pose_and_axis_reverse_are_real() {
    let source = extrude_arc_line_region(&region(), 5., tolerance().absolute()).unwrap();
    let source = bore_normal_arc_line_prism(&source, Point3::new(-4., 0., 2.), 0.7, tolerance())
        .unwrap()
        .into_solids()
        .0;
    let transform = Transform::translation(Vec3::new(13., -9., 6.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    let source = source
        .transformed(transform, tolerance().absolute())
        .unwrap();
    let p = plane(0.).transformed(transform).unwrap();
    let result = split_normal_arc_line_prism_by_plane(&source, &p, tolerance()).unwrap();
    check(&result, &source);
    assert_eq!(
        result
            .negative()
            .shell
            .faces
            .iter()
            .filter(|f| f.wires.len() == 2)
            .count(),
        2
    );
    assert!(result
        .positive()
        .shell
        .faces
        .iter()
        .all(|f| f.wires.len() == 1));
    let Surface::Plane { origin, u, v } = p else {
        panic!()
    };
    let reverse = Surface::Plane {
        origin,
        u,
        v: v * (-1.),
    };
    let opposite = split_normal_arc_line_prism_by_plane(&source, &reverse, tolerance()).unwrap();
    assert!(
        (result.negative().volume().unwrap() - opposite.positive().volume().unwrap()).abs() < 1e-9
    );
}
#[test]
fn contacts_hole_crossings_skew_invalid_and_precision_fail_atomically() {
    let source = extrude_arc_line_region(&region(), 5., tolerance().absolute()).unwrap();
    let before = export_step_bounded_analytic_mm(&source, 1e-6).unwrap();
    for p in [
        plane(10.),
        plane(8.),
        plane(20.),
        Surface::Plane {
            origin: Point3::new(0., 0., 0.),
            u: Vec3::new(0., 1., 0.),
            v: Vec3::new(1., 0., 1.).normalized().unwrap(),
        },
        plane(1e12),
        Surface::Plane {
            origin: Point3::new(f64::NAN, 0., 0.),
            u: Vec3::new(0., 0., 1.),
            v: Vec3::new(0., -1., 0.),
        },
    ] {
        assert!(split_normal_arc_line_prism_by_plane(&source, &p, tolerance()).is_err());
    }
    assert_eq!(
        export_step_bounded_analytic_mm(&source, 1e-6).unwrap(),
        before
    );
    let holed =
        bore_normal_arc_line_prism(&source, Point3::new(0., 0., 0.), 1., tolerance()).unwrap();
    assert!(split_normal_arc_line_prism_by_plane(holed.kept(), &plane(0.), tolerance()).is_err());
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(split_normal_arc_line_prism_by_plane(&source, &plane(9.999), relative).is_err());
}
