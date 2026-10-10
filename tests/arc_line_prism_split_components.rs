use hagane::*;
fn tolerance() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 1e-10).unwrap()
}
fn plane(y: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(0., y, 0.),
        u: Vec3::new(0., 0., 1.),
        v: Vec3::new(1., 0., 0.),
    }
}
fn source() -> Solid {
    let t = tolerance();
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., t.absolute()).unwrap();
    let mut s = extrude_arc_line(&profile, 5., t.absolute()).unwrap();
    for x in [-4., 4.] {
        s = bore_normal_arc_line_prism(&s, Point3::new(x, 0., 0.), 1.5, t)
            .unwrap()
            .into_solids()
            .0;
    }
    s
}
fn check(source: &Solid, result: &NormalArcLinePrismPlaneSplitComponents) {
    let mut total = 0.;
    for body in result.negative().iter().chain(result.positive()) {
        body.validate(tolerance().absolute()).unwrap();
        total += body.volume().unwrap();
        for face in &body.shell.faces {
            for c in face.wires.iter().flat_map(|w| &w.coedges) {
                let edge = &body.edges[c.edge];
                let r = edge.curve.range();
                for t in [0., 0.18, 0.61, 1.] {
                    let t = r[0] + t * (r[1] - r[0]);
                    let uv = c.pcurve.evaluate(t);
                    assert!(
                        (edge.curve.evaluate(t) - face.surface.evaluate(uv[0], uv[1])).norm()
                            < 1e-6
                    );
                }
            }
        }
        let text = export_step_bounded_analytic_mm(body, 1e-6).unwrap();
        let parsed = import_step_bounded_analytic_mm(&text, tolerance().absolute()).unwrap();
        assert!((body.volume().unwrap() - parsed.volume().unwrap()).abs() < 1e-9);
    }
    assert!((total - source.volume().unwrap()).abs() < 1e-9);
    for section in result.sections() {
        assert_eq!(section.rings[0].len(), 4);
        let Surface::Plane { u, v, .. } = section.surface else {
            panic!()
        };
        let normal = u.cross(v) * f64::from(section.orientation);
        let corners = &section.rings[0];
        let matching = |sides: &[Solid], sign: f64| {
            sides
                .iter()
                .flat_map(|s| s.shell.faces.iter().map(move |f| (s, f)))
                .any(|(body, f)| {
                    let Surface::Plane { u, v, .. } = f.surface else {
                        return false;
                    };
                    if f.wires.len() != 1
                        || f.wires[0].coedges.len() != 4
                        || normal.dot(u.cross(v) * f64::from(f.orientation)) * sign < 0.99
                    {
                        return false;
                    }
                    f.wires[0].coedges.iter().all(|c| {
                        let p = body.vertices[body.edges[c.edge].vertices[usize::from(!c.forward)]]
                            .point;
                        corners.iter().any(|q| (p - *q).norm() < 1e-6)
                    })
                })
        };
        assert!(matching(result.negative(), 1.));
        assert!(matching(result.positive(), -1.));
    }
}
#[test]
fn crossed_two_circular_openings_have_three_actual_material_sections() {
    let source = source();
    let cut = 0.6;
    let result =
        split_normal_arc_line_prism_by_plane_components(&source, &plane(cut), tolerance()).unwrap();
    assert_eq!(
        (
            result.negative().len(),
            result.positive().len(),
            result.sections().len()
        ),
        (1, 1, 3)
    );
    check(&source, &result);
    let r = 1.5f64;
    let extra = r * r * (cut / r).asin() + cut * (r * r - cut * cut).sqrt();
    let expected = ((320. - (4. - std::f64::consts::PI) * 4.) / 2.
        - 20. * cut
        - 2. * (std::f64::consts::PI * r * r / 2. - extra))
        * 5.;
    assert!((result.positive()[0].volume().unwrap() - expected).abs() < 1e-9);
    assert!(split_normal_arc_line_prism_by_plane(&source, &plane(cut), tolerance()).is_err());
}
#[test]
fn concave_arc_stock_returns_every_disconnected_component() {
    let points = [
        [-29., -20.],
        [30., -20.],
        [30., 20.],
        [10., 20.],
        [10., -5.],
        [-10., -5.],
        [-10., 20.],
        [-30., 20.],
        [-30., -19.],
    ];
    let mut segments: Vec<_> = points
        .windows(2)
        .map(|p| PlanarSegment::Line { a: p[0], b: p[1] })
        .collect();
    segments.push(PlanarSegment::Arc {
        center: [-29., -19.],
        radius: 1.,
        start_angle: std::f64::consts::PI,
        sweep: std::f64::consts::FRAC_PI_2,
    });
    let source = extrude_arc_line(
        &ArcLineProfile {
            origin: Point3::new(0., 0., 0.),
            segments,
        },
        5.,
        tolerance().absolute(),
    )
    .unwrap();
    let result =
        split_normal_arc_line_prism_by_plane_components(&source, &plane(0.), tolerance()).unwrap();
    assert_eq!(
        (
            result.negative().len(),
            result.positive().len(),
            result.sections().len()
        ),
        (1, 2, 2)
    );
    check(&source, &result);
    for child in result.positive() {
        assert!((child.volume().unwrap() - 2000.).abs() < 1e-9);
        assert!(child
            .edges
            .iter()
            .all(|e| matches!(e.curve, Curve::Line { .. })));
    }
    let expected = (1100. - (1. - std::f64::consts::FRAC_PI_4)) * 5.;
    assert!((result.negative()[0].volume().unwrap() - expected).abs() < 1e-9);
}
#[test]
fn posed_crossed_hole_split_and_failures_keep_source_unchanged() {
    let source = source();
    let transform = Transform::translation(Vec3::new(7., -4., 9.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.61).unwrap())
        .unwrap();
    let placed = source
        .transformed(transform, tolerance().absolute())
        .unwrap();
    let cut = plane(0.6).transformed(transform).unwrap();
    let result =
        split_normal_arc_line_prism_by_plane_components(&placed, &cut, tolerance()).unwrap();
    check(&placed, &result);
    let before = export_step_bounded_analytic_mm(&source, 1e-6).unwrap();
    for cut in [
        plane(0.),
        plane(1.5),
        plane(8.),
        plane(1e12),
        Surface::Plane {
            origin: Point3::new(0., 0., 0.),
            u: Vec3::new(0., 0., 1.),
            v: Vec3::new(1., 0., 1.).normalized().unwrap(),
        },
        Surface::Plane {
            origin: Point3::new(f64::NAN, 0., 0.),
            u: Vec3::new(0., 0., 1.),
            v: Vec3::new(1., 0., 0.),
        },
    ] {
        assert!(
            split_normal_arc_line_prism_by_plane_components(&source, &cut, tolerance()).is_err()
        );
    }
    assert_eq!(
        export_step_bounded_analytic_mm(&source, 1e-6).unwrap(),
        before
    );
}
