use hagane::*;
use std::f64::consts::PI;
fn lens(sweep: f64, t: Tolerance) -> Solid {
    let radius = 2.;
    let phase = 0.37;
    let a = [radius * f64::cos(phase), radius * f64::sin(phase)];
    let b = [
        radius * f64::cos(phase + sweep),
        radius * f64::sin(phase + sweep),
    ];
    extrude_arc_line(
        &ArcLineProfile {
            origin: Point3::new(0., 0., 0.),
            segments: vec![
                PlanarSegment::Arc {
                    center: [0., 0.],
                    radius,
                    start_angle: phase,
                    sweep,
                },
                PlanarSegment::Line { a: b, b: a },
            ],
        },
        3.,
        t,
    )
    .unwrap()
}
fn box_fillet(scale: f64, t: Tolerance) -> Solid {
    let stock = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80. * scale, 60. * scale, 20. * scale),
        },
        t,
    )
    .unwrap();
    fillet_parallel_box_edges(
        &stock,
        &[(8, 3. * scale), (9, 4. * scale)],
        GeometryTolerance::new(t.linear, 1e-10, 0.).unwrap(),
    )
    .unwrap()
    .into_solid()
}
#[test]
fn arbitrary_phase_and_near_pi_preserve_actual_branches_and_cap_prism() {
    let t = Tolerance::new(1e-6).unwrap();
    for sweep in [0.05, PI - 1e-10, PI] {
        let source = lens(sweep, t);
        let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
        let imported = match import_step_bounded_analytic_mm(&text, t) {
            Ok(solid) => solid,
            Err(Error::Unsupported(_)) if sweep == PI => continue,
            Err(error) => panic!("resolved positive phase failed: {error}"),
        };
        assert!((imported.volume().unwrap() - 6. * (sweep - sweep.sin())).abs() < 1e-8);
        assert_eq!(
            (
                imported.vertices.len(),
                imported.edges.len(),
                imported.shell.faces.len()
            ),
            (4, 6, 4)
        );
        for edge in &source.edges {
            let range = edge.curve.range();
            let samples: Vec<_> = (0..=16)
                .map(|i| {
                    edge.curve
                        .evaluate(range[0] + (range[1] - range[0]) * i as f64 / 16.)
                })
                .collect();
            assert!(imported.edges.iter().any(|other| {
                let range = other.curve.range();
                samples.iter().enumerate().all(|(i, p)| {
                    (*p - other
                        .curve
                        .evaluate(range[0] + (range[1] - range[0]) * i as f64 / 16.))
                    .norm()
                        < 1e-8
                })
            }));
        }
    }
}
#[test]
fn metre_uv_conversion_and_unknown_units_are_checked() {
    let t = Tolerance::new(1e-6).unwrap();
    let original = box_fillet(1., t);
    let small = box_fillet(0.001, Tolerance::new(1e-9).unwrap());
    let text = export_step_bounded_analytic_mm(&small, 1e-9).unwrap();
    let metre = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let imported = import_step_bounded_analytic_mm(&metre, t).unwrap();
    assert!((imported.volume().unwrap() - original.volume().unwrap()).abs() < 1e-7);
    assert!((imported.bounds().max - original.bounds().max).norm() < 1e-8);
    assert!(import_step_bounded_analytic_mm(
        &metre.replace("SI_UNIT($,.METRE.)", "SI_UNIT($,.INCH.)"),
        t
    )
    .is_err());
}
#[test]
fn phase_precision_and_same_sense_tampering_do_not_fabricate_a_body() {
    let t = Tolerance::new(1e-6).unwrap();
    let source = box_fillet(1., t);
    let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    let mut changed = false;
    let tampered = text
        .lines()
        .map(|line| {
            if !changed && line.contains("=EDGE_CURVE(") {
                changed = true;
                line.replace(",.T.);", ",.F.);")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(changed);
    assert!(import_step_bounded_analytic_mm(&tampered, t).is_err());
    assert!(import_step_bounded_analytic_mm(
        &text.replace("SI_UNIT($,.RADIAN.)", "SI_UNIT($,.DEGREE.)"),
        t
    )
    .is_err());
    let loose = Tolerance::new(0.01).unwrap();
    let tr = Transform::translation(Vec3::new(1e6, -1e6, 1e6))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let far = source.transformed(tr, loose).unwrap();
    let far_text = export_step_bounded_analytic_mm(&far, loose.linear).unwrap();
    assert!(
        import_step_bounded_analytic_mm(&far_text, Tolerance::new(1e-8).unwrap()).is_err(),
        "lost zero-angle phase precision must be explicit"
    );
    assert!(import_step_bounded_analytic_mm(&text, t).is_ok());
}

#[test]
fn negative_near_pi_endpoint_is_refused_without_phase_clamping() {
    let t = Tolerance::new(1e-6).unwrap();
    let source = lens(PI - 1e-10, t);
    let mut text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    fn number(x: f64) -> String {
        let s = x.to_string();
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s.replace('e', "E")
        } else {
            format!("{s}.")
        }
    }
    fn point(p: Point3) -> String {
        format!(
            "CARTESIAN_POINT('',({},{},{}))",
            number(p.x),
            number(p.y),
            number(p.z)
        )
    }
    let mut changed = 0;
    for edge in &source.edges {
        if let Curve::Arc { .. } = edge.curve {
            let old = source.vertices[edge.vertices[1]].point;
            let new = edge.curve.evaluate(PI + 1e-10);
            let key = point(old);
            if text.contains(&key) {
                text = text.replace(&key, &point(new));
                changed += 1;
            }
        }
    }
    assert_eq!(changed, 2);
    assert!(
        import_step_bounded_analytic_mm(&text, t).is_err(),
        "negative endpoint phase must not become a canonical pi arc"
    );
}

#[test]
fn coherent_sub_tolerance_cap_uv_translation_cannot_replace_actual_boundary() {
    let t = Tolerance::new(1e-6).unwrap();
    let mut source = box_fillet(1., t);
    for face in &mut source.shell.faces {
        if matches!(face.surface, Surface::Plane { .. })
            && face
                .wires
                .iter()
                .flat_map(|w| &w.coedges)
                .any(|c| matches!(c.pcurve, PCurve::Arc { .. }))
        {
            for coedge in &mut face.wires[0].coedges {
                match &mut coedge.pcurve {
                    PCurve::Affine { origin, .. } => origin[0] += 0.5 * t.linear,
                    PCurve::Arc { center, .. } => center[0] += 0.5 * t.linear,
                    _ => panic!("unexpected cap trim"),
                }
            }
        }
    }
    // Generic modeling tolerance admits the coherent translation and its area is unchanged.
    source.validate(t).unwrap();
    let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    assert!(
        import_step_bounded_analytic_mm(&text, t).is_err(),
        "a translated pcurve profile must not certify different actual 3D rims"
    );
}
fn holed_source(t: Tolerance) -> Solid {
    let outer = rounded_rectangle_profile(Point3::new(0., 0., 0.), 8., 6., 1., t).unwrap();
    let points = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]];
    let hole = (0..4)
        .map(|i| PlanarSegment::Line {
            a: points[i],
            b: points[(i + 1) % 4],
        })
        .collect();
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: outer.origin,
            outer: outer.segments,
            holes: vec![hole],
        },
        3.,
        t,
    )
    .unwrap()
}
#[test]
fn genuine_valid_prism_with_inner_opening_has_actual_certified_boundary() {
    let t = Tolerance::new(1e-6).unwrap();
    let source = holed_source(t);
    let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    let imported = import_step_bounded_analytic_mm(&text, t).unwrap();
    imported.validate(t).unwrap();
    assert_eq!(
        (
            imported.vertices.len(),
            imported.edges.len(),
            imported.shell.faces.len()
        ),
        (24, 36, 14)
    );
    assert_eq!(
        imported
            .shell
            .faces
            .iter()
            .filter(|f| f.wires.len() == 2)
            .count(),
        2
    );
    assert!((imported.volume().unwrap() - (120. + 3. * PI)).abs() < 1e-9);
    let roundtrip = export_step_bounded_analytic_mm(&imported, t.linear).unwrap();
    assert!(
        (import_step_bounded_analytic_mm(&roundtrip, t)
            .unwrap()
            .volume()
            .unwrap()
            - source.volume().unwrap())
        .abs()
            < 1e-9
    );
}
#[test]
fn inner_wire_sub_tolerance_shift_is_not_a_geometry_preimage() {
    let t = Tolerance::new(1e-6).unwrap();
    let mut source = holed_source(t);
    for face in &mut source.shell.faces {
        if face.wires.len() == 2 {
            for c in &mut face.wires[1].coedges {
                if let PCurve::Affine { origin, .. } = &mut c.pcurve {
                    origin[0] += 0.5 * t.linear;
                }
            }
        }
    }
    source.validate(t).unwrap();
    let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    assert!(import_step_bounded_analytic_mm(&text, t).is_err());
}
#[test]
fn nesting_and_contact_are_refused_by_actual_region_simplicity_proof() {
    let t = Tolerance::new(1e-6).unwrap();
    let outer = rounded_rectangle_profile(Point3::new(0., 0., 0.), 8., 6., 1., t).unwrap();
    let square = |a: f64, b: f64| {
        let points = [[a, a], [b, a], [b, b], [a, b]];
        (0..4)
            .map(|i| PlanarSegment::Line {
                a: points[i],
                b: points[(i + 1) % 4],
            })
            .collect::<Vec<_>>()
    };
    for holes in [
        vec![square(-1., 1.), square(-0.5, 0.5)],
        vec![square(-1., 1.), square(1., 2.)],
    ] {
        assert!(extrude_arc_line_region(
            &ArcLineRegion {
                origin: outer.origin,
                outer: outer.segments.clone(),
                holes
            },
            3.,
            t
        )
        .is_err());
    }
}

#[test]
fn distant_support_plane_and_compensated_circle_uv_do_not_evade_world_guard() {
    let loose = Tolerance::new(1e-2).unwrap();
    let mut source = make_cylinder(
        CylinderSpec {
            base: Point3::new(3., 4., 0.),
            radius: 2.,
            height: 5.,
        },
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    for face in &mut source.shell.faces {
        if let Surface::Plane { origin, u, .. } = &mut face.surface {
            *origin = *origin + *u * 1e12;
            for wire in &mut face.wires {
                for coedge in &mut wire.coedges {
                    if let PCurve::Circle { center, .. } = &mut coedge.pcurve {
                        center[0] -= 1e12;
                    }
                }
            }
        }
    }
    source.validate(loose).unwrap();
    assert!((source.volume().unwrap() - 20. * PI).abs() < 1e-8);
    // The unchanged legacy writer can express this full-circle geometry; the
    // opt-in reader must inspect all support origins, not just circle centers.
    let text = export_step_mm(&source, loose).unwrap();
    assert!(import_step_bounded_analytic_mm(&text, Tolerance::new(1e-6).unwrap()).is_err());
}
