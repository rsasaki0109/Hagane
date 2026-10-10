use hagane::*;
fn stock() -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(8., 6., 4.),
        },
        Tolerance::default(),
    )
    .unwrap()
}
fn roundtrip(source: &Solid) {
    let t = Tolerance::default();
    let text = export_step_bounded_analytic_mm(source, t.linear).unwrap();
    let solid = import_step_bounded_analytic_mm(&text, t).unwrap();
    solid.validate(t).unwrap();
    assert_eq!(
        (
            solid.vertices.len(),
            solid.edges.len(),
            solid.shell.faces.len()
        ),
        (
            source.vertices.len(),
            source.edges.len(),
            source.shell.faces.len()
        )
    );
    assert!((solid.volume().unwrap() - source.volume().unwrap()).abs() < 1e-8);
    for f in &solid.shell.faces {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            let range = solid.edges[c.edge].curve.range();
            for a in [0., 0.17, 0.61, 1.] {
                let param = range[0] + a * (range[1] - range[0]);
                let uv = c.pcurve.try_evaluate(param).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - solid.edges[c.edge].curve.try_evaluate(param).unwrap())
                    .norm()
                        < t.linear
                );
            }
        }
    }
}
#[test]
fn every_parallel_fillet_subset_and_pose_imports_actual_brep() {
    let t = GeometryTolerance::default();
    let s = stock();
    for edges in [[8, 9, 10, 11], [0, 2, 4, 6], [1, 3, 5, 7]] {
        for mask in 1..16 {
            let requests: Vec<_> = (0..4)
                .filter(|k| mask & (1 << k) != 0)
                .map(|k| (edges[k], 0.3 + 0.1 * k as f64))
                .collect();
            let r = fillet_parallel_box_edges(&s, &requests, t).unwrap();
            roundtrip(r.solid());
        }
    }
    let r = fillet_parallel_box_edges(&s, &[(8, 0.5), (9, 0.3)], t).unwrap();
    let placed = r
        .solid()
        .transformed(
            Transform::translation(Vec3::new(12., -8., 4.))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
                .unwrap(),
            t.absolute(),
        )
        .unwrap();
    roundtrip(&placed);
}
#[test]
fn plane_and_full_cylinder_explicit_pcurves_and_legacy_scope() {
    roundtrip(&stock());
    let t = Tolerance::default();
    roundtrip(
        &make_cylinder(
            CylinderSpec {
                base: Point3::new(0., 0., 0.),
                radius: 2.,
                height: 4.,
            },
            t,
        )
        .unwrap(),
    );
    let r = fillet_parallel_box_edges(&stock(), &[(8, 0.5)], GeometryTolerance::default()).unwrap();
    let text = export_step_bounded_analytic_mm(r.solid(), t.linear).unwrap();
    assert!(import_step_mm(&text, t).is_err());
    assert!(import_step_bounded_analytic_mm(&(text + "trailing"), t).is_err());
    assert!(import_step_bounded_analytic_mm(&" ".repeat(STEP_IMPORT_MAX_BYTES + 1), t).is_err());
}
#[test]
fn semicircle_branch_and_wrong_master_curve_sense_are_explicit() {
    let t = Tolerance::default();
    let source = extrude_arc_line(
        &ArcLineProfile {
            origin: Point3::new(0., 0., 0.),
            segments: vec![
                PlanarSegment::Arc {
                    center: [0., 0.],
                    radius: 2.,
                    start_angle: 0.,
                    sweep: std::f64::consts::PI,
                },
                PlanarSegment::Arc {
                    center: [0., 0.],
                    radius: 2.,
                    start_angle: std::f64::consts::PI,
                    sweep: std::f64::consts::PI,
                },
            ],
        },
        3.,
        t,
    )
    .unwrap();
    let unresolved = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    assert!(matches!(
        import_step_bounded_analytic_mm(&unresolved, t),
        Err(Error::Unsupported(
            "bounded STEP arc endpoint phase is unresolved"
        ))
    ));
    // Construct a separate exactly axial semicircle fixture; the ordinary trig
    // construction above retains a tiny negative phase on its second circle.
    let mut source = source;
    let axial = |frame: Frame3| {
        Frame3::new(
            frame.origin(),
            frame.axes().map(|v| {
                Vec3::new(
                    if v.x.abs() < 1e-14 { 0. } else { v.x },
                    if v.y.abs() < 1e-14 { 0. } else { v.y },
                    if v.z.abs() < 1e-14 { 0. } else { v.z },
                )
            }),
            t,
        )
        .unwrap()
    };
    for v in &mut source.vertices {
        if v.point.y.abs() < 1e-14 {
            v.point.y = 0.;
        }
    }
    for e in &mut source.edges {
        if let Curve::Arc { frame, .. } = &mut e.curve {
            *frame = axial(*frame);
        }
    }
    for f in &mut source.shell.faces {
        if let Surface::FramedCylinder { frame, .. } = &mut f.surface {
            *frame = axial(*frame);
        }
    }
    source.validate(t).unwrap();
    roundtrip(&source);
    let text = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
    let index = lines
        .iter()
        .position(|s| s.contains("EDGE_CURVE("))
        .unwrap();
    lines[index] = lines[index].replace(",.T.)", ",.F.)");
    assert!(import_step_bounded_analytic_mm(&lines.join("\n"), t).is_err());
    for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(Tolerance::new(tolerance).is_err());
    }
}

#[test]
fn cyclic_edge_loop_start_preserves_partial_cylinder_parameters() {
    let t = Tolerance::default();
    let source = fillet_parallel_box_edges(
        &stock(),
        &[(8, 0.3), (9, 0.3), (10, 0.3), (11, 0.3)],
        GeometryTolerance::default(),
    )
    .unwrap()
    .into_solid();
    let original = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    for shift in 1..4 {
        let rotated = original
            .lines()
            .map(|line| {
                if !line.contains("=EDGE_LOOP(") {
                    return line.to_owned();
                }
                let start = line.find(",(").unwrap() + 2;
                let end = line.rfind("));").unwrap();
                let mut refs: Vec<_> = line[start..end].split(',').collect();
                let n = refs.len();
                refs.rotate_left(shift % n);
                format!("{}{}{}", &line[..start], refs.join(","), &line[end..])
            })
            .collect::<Vec<_>>()
            .join("\n");
        let body = import_step_bounded_analytic_mm(&rotated, t).unwrap();
        assert!((body.volume().unwrap() - source.volume().unwrap()).abs() < 1e-8);
        for face in &body.shell.faces {
            for c in face.wires.iter().flat_map(|w| &w.coedges) {
                for parameter in [0., 0.37, 1.] {
                    let edge = &body.edges[c.edge];
                    let r = edge.curve.range();
                    let parameter = r[0] + parameter * (r[1] - r[0]);
                    let uv = c.pcurve.evaluate(parameter);
                    assert!(
                        (edge.curve.evaluate(parameter) - face.surface.evaluate(uv[0], uv[1]))
                            .norm()
                            < t.linear
                    );
                }
            }
        }
    }
}

fn innerwire_fixture(count: usize, scale: f64) -> Solid {
    let t = Tolerance::new(1e-9 * scale).unwrap();
    let rounded = |width, depth, radius| {
        rounded_rectangle_profile(
            Point3::new(0., 0., 0.),
            width * scale,
            depth * scale,
            radius * scale,
            t,
        )
        .unwrap()
        .segments
    };
    let mut holes = Vec::new();
    for i in 0..count {
        let offset = (i as f64 - (count as f64 - 1.) / 2.) * 6. * scale;
        let segments = if i == 1 {
            vec![
                PlanarSegment::Line {
                    a: [-scale, -scale],
                    b: [scale, -scale],
                },
                PlanarSegment::Line {
                    a: [scale, -scale],
                    b: [scale, scale],
                },
                PlanarSegment::Line {
                    a: [scale, scale],
                    b: [-scale, scale],
                },
                PlanarSegment::Line {
                    a: [-scale, scale],
                    b: [-scale, -scale],
                },
            ]
        } else {
            rounded(3., 4., 0.4)
        };
        holes.push(
            segments
                .into_iter()
                .map(|s| match s {
                    PlanarSegment::Line { mut a, mut b } => {
                        a[0] += offset;
                        b[0] += offset;
                        PlanarSegment::Line { a, b }
                    }
                    PlanarSegment::Arc {
                        mut center,
                        radius,
                        start_angle,
                        sweep,
                    } => {
                        center[0] += offset;
                        PlanarSegment::Arc {
                            center,
                            radius,
                            start_angle,
                            sweep,
                        }
                    }
                })
                .collect(),
        );
    }
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: rounded(24., 14., 1.2),
            holes,
        },
        5. * scale,
        t,
    )
    .unwrap()
}

#[test]
fn actual_multiple_inner_wires_preserve_caps_and_normal_walls() {
    for count in 1..=3 {
        let source = innerwire_fixture(count, 1.);
        roundtrip(&source);
        let placed = source
            .transformed(
                Transform::translation(Vec3::new(7., -4., 11.))
                    .unwrap()
                    .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.61).unwrap())
                    .unwrap(),
                Tolerance::default(),
            )
            .unwrap();
        roundtrip(&placed);
        let text = export_step_bounded_analytic_mm(&source, 1e-7).unwrap();
        let body = import_step_bounded_analytic_mm(&text, Tolerance::default()).unwrap();
        assert_eq!(
            body.shell
                .faces
                .iter()
                .filter(|f| f.wires.len() == count + 1)
                .count(),
            2
        );
        assert!(import_step_mm(&text, Tolerance::default()).is_err());
    }
}

#[test]
fn inner_wire_boundary_order_loop_start_and_metre_units_are_semantic() {
    let source = innerwire_fixture(3, 1.);
    let text = export_step_bounded_analytic_mm(&source, 1e-7).unwrap();
    let shuffled = text
        .lines()
        .map(|line| {
            let bounds = line.contains("=ADVANCED_FACE(");
            if !bounds && !line.contains("=EDGE_LOOP(") {
                return line.to_owned();
            }
            let start = line.find(",(").unwrap() + 2;
            let end = if bounds {
                start + line[start..].find(')').unwrap()
            } else {
                line.rfind("));").unwrap()
            };
            let mut refs: Vec<_> = line[start..end].split(',').collect();
            refs.rotate_left(1);
            format!("{}{}{}", &line[..start], refs.join(","), &line[end..])
        })
        .collect::<Vec<_>>()
        .join("\n");
    let imported = import_step_bounded_analytic_mm(&shuffled, Tolerance::default()).unwrap();
    assert!((imported.volume().unwrap() - source.volume().unwrap()).abs() < 1e-8);
    let metres = export_step_bounded_analytic_mm(&innerwire_fixture(3, 0.001), 1e-10)
        .unwrap()
        .replace(".MILLI.,.METRE.", "$,.METRE.");
    let imported = import_step_bounded_analytic_mm(&metres, Tolerance::default()).unwrap();
    assert!((imported.volume().unwrap() - source.volume().unwrap()).abs() < 1e-8);
    assert_eq!(
        imported
            .shell
            .faces
            .iter()
            .filter(|f| f.wires.len() == 4)
            .count(),
        2
    );
    let malformed = text.replacen("=FACE_BOUND(", "=FACE_OUTER_BOUND(", 1);
    assert!(import_step_bounded_analytic_mm(&malformed, Tolerance::default()).is_err());
}
