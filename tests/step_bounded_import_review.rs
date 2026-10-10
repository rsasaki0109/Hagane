use hagane::*;
use std::f64::consts::PI;

fn stock(scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80. * scale, 60. * scale, 20. * scale),
        },
        Tolerance::new(1e-6 * scale).unwrap(),
    )
    .unwrap()
}
fn check_geometry(expected: &Solid, actual: &Solid) {
    actual.validate(Tolerance::new(1e-6).unwrap()).unwrap();
    assert_eq!(
        (
            actual.vertices.len(),
            actual.edges.len(),
            actual.shell.faces.len()
        ),
        (
            expected.vertices.len(),
            expected.edges.len(),
            expected.shell.faces.len()
        )
    );
    assert!((actual.volume().unwrap() - expected.volume().unwrap()).abs() < 1e-7);
    assert!((actual.bounds().min - expected.bounds().min).norm() < 1e-9);
    assert!((actual.bounds().max - expected.bounds().max).norm() < 1e-9);
    for edge in &expected.edges {
        let range = edge.curve.range();
        let samples: Vec<_> = (0..=16)
            .map(|i| {
                edge.curve
                    .try_evaluate(range[0] + (range[1] - range[0]) * i as f64 / 16.)
                    .unwrap()
            })
            .collect();
        assert!(
            actual.edges.iter().any(|other| {
                let domain = other.curve.range();
                samples.iter().enumerate().all(|(i, p)| {
                    (*p - other
                        .curve
                        .try_evaluate(domain[0] + (domain[1] - domain[0]) * i as f64 / 16.)
                        .unwrap())
                    .norm()
                        < 1e-9
                })
            }),
            "actual curve parameterization must survive import"
        );
    }
    let mut uses = vec![Vec::new(); actual.edges.len()];
    for face in &actual.shell.faces {
        for wire in &face.wires {
            for coedge in &wire.coedges {
                uses[coedge.edge].push(coedge.forward == (face.orientation == 1));
                let curve = &actual.edges[coedge.edge].curve;
                let range = curve.range();
                for i in 0..=16 {
                    let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                    let uv = coedge.pcurve.try_evaluate(t).unwrap();
                    assert!(
                        (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                            - curve.try_evaluate(t).unwrap())
                        .norm()
                            < 1e-9
                    );
                }
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0] != u[1]));
    for (fi, face) in actual.shell.faces.iter().enumerate() {
        if !matches!(
            face.surface,
            Surface::FramedCylinder { .. } | Surface::Cylinder { .. }
        ) {
            continue;
        }
        for coedge in &face.wires[0].coedges {
            if !matches!(actual.edges[coedge.edge].curve, Curve::Line { .. }) {
                continue;
            }
            let other = actual.shell.faces.iter().enumerate().find(|(index, f)| {
                *index != fi
                    && f.wires
                        .iter()
                        .flat_map(|w| &w.coedges)
                        .any(|c| c.edge == coedge.edge)
            });
            let Some((_, other)) = other else {
                continue;
            }; // Full-cylinder seam.
            let uv = coedge.pcurve.try_evaluate(0.5).unwrap();
            let a = face.surface.normal(uv[0]) * f64::from(face.orientation);
            let b = other.surface.normal(0.) * f64::from(other.orientation);
            assert!(a.dot(b) > 1. - 1e-12, "import preserves physical tangency");
        }
    }
}
fn roundtrip(source: &Solid) -> Solid {
    let t = Tolerance::new(1e-6).unwrap();
    let step = export_step_bounded_analytic_mm(source, t.linear).unwrap();
    let imported = import_step_bounded_analytic_mm(&step, t).unwrap();
    check_geometry(source, &imported);
    let reimported = import_step_bounded_analytic_mm(
        &export_step_bounded_analytic_mm(&imported, t.linear).unwrap(),
        t,
    )
    .unwrap();
    check_geometry(&imported, &reimported);
    imported
}

#[test]
fn all_fillet_subsets_axes_and_pose_preserve_actual_geometry() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = stock(1.);
    for (edges, height) in [
        ([0, 2, 4, 6], 80.),
        ([1, 3, 5, 7], 60.),
        ([8, 9, 10, 11], 20.),
    ] {
        for mask in 1..16 {
            let request: Vec<_> = edges
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(i, &edge)| (edge, 2. + i as f64 * 0.25))
                .collect();
            let fillet = fillet_parallel_box_edges(&source, &request, tol).unwrap();
            let imported = roundtrip(fillet.solid());
            let expected =
                96000. - height * (1. - PI / 4.) * request.iter().map(|(_, r)| r * r).sum::<f64>();
            assert!((imported.volume().unwrap() - expected).abs() < 1e-7);
            assert!(import_step_mm(
                &export_step_bounded_analytic_mm(fillet.solid(), tol.linear()).unwrap(),
                tol.absolute()
            )
            .is_err());
        }
    }
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    roundtrip(
        fillet_parallel_box_edges(
            &source.transformed(frame, tol.absolute()).unwrap(),
            &[(8, 2.), (9, 3.), (10, 4.), (11, 2.5)],
            tol,
        )
        .unwrap()
        .solid(),
    );
    roundtrip(&source);
    roundtrip(
        &make_cylinder(
            CylinderSpec {
                base: Point3::new(3., 4., 0.),
                radius: 2.,
                height: 5.,
            },
            tol.absolute(),
        )
        .unwrap(),
    );
}

#[test]
fn metre_records_scale_planar_uv_and_cylinder_height_but_preserve_angles() {
    let request = [(8, 2.), (9, 3.), (10, 4.), (11, 2.5)];
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let original = fillet_parallel_box_edges(&stock(1.), &request, tol).unwrap();
    let scaled_request: Vec<_> = request.iter().map(|&(i, r)| (i, r * 0.001)).collect();
    let scaled = fillet_parallel_box_edges(
        &stock(0.001),
        &scaled_request,
        GeometryTolerance::new(1e-9, 1e-10, 0.).unwrap(),
    )
    .unwrap();
    let metre = export_step_bounded_analytic_mm(scaled.solid(), 1e-9)
        .unwrap()
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let imported = import_step_bounded_analytic_mm(&metre, tol.absolute()).unwrap();
    check_geometry(original.solid(), &imported);
}

fn fields(text: &str) -> Vec<String> {
    let mut depth = 0;
    let mut quote = false;
    let mut start = 0;
    let mut result = Vec::new();
    for (i, c) in text.char_indices() {
        match c {
            '\'' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => depth -= 1,
            ',' if !quote && depth == 0 => {
                result.push(text[start..i].to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(text[start..].to_owned());
    result
}
fn entity(step: &str, id: &str) -> (String, Vec<String>) {
    let line = step
        .lines()
        .find(|line| line.starts_with(&format!("{id}=")))
        .unwrap();
    let body = line.split_once('=').unwrap().1.trim_end_matches(';');
    let (kind, args) = body.split_once('(').unwrap();
    (kind.to_owned(), fields(args.strip_suffix(')').unwrap()))
}
fn replace(step: &str, id: &str, args: &[String]) -> String {
    let (kind, _) = entity(step, id);
    let old = step
        .lines()
        .find(|line| line.starts_with(&format!("{id}=")))
        .unwrap();
    step.replace(old, &format!("{id}={kind}({});", args.join(",")))
}
fn first(step: &str, kind: &str) -> String {
    step.lines()
        .find(|line| line.contains(&format!("={kind}(")))
        .unwrap()
        .split_once('=')
        .unwrap()
        .0
        .to_owned()
}

#[test]
fn entity_order_numbering_loop_start_are_equivalent_and_actual_geometry_tampering_rejects() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let result =
        fillet_parallel_box_edges(&stock(1.), &[(8, 2.), (9, 3.), (10, 4.), (11, 2.5)], tol)
            .unwrap();
    let step = export_step_bounded_analytic_mm(result.solid(), tol.linear()).unwrap();
    let mut reordered = String::new();
    let mut entries: Vec<_> = step.lines().filter(|line| line.starts_with('#')).collect();
    entries.reverse();
    let mut used = false;
    for line in step.lines() {
        if line.starts_with('#') {
            if !used {
                reordered.push_str(&entries.join("\n"));
                reordered.push('\n');
                used = true;
            }
        } else {
            reordered.push_str(line);
            reordered.push('\n');
        }
    }
    let chars: Vec<_> = reordered.chars().collect();
    let mut renamed = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        if chars[i] == '\'' {
            quoted = !quoted;
        }
        if chars[i] == '#' && !quoted {
            i += 1;
            let begin = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let id: usize = chars[begin..i].iter().collect::<String>().parse().unwrap();
            renamed.push_str(&format!("#{}", 10000 - id));
        } else {
            renamed.push(chars[i]);
            i += 1;
        }
    }
    check_geometry(
        result.solid(),
        &import_step_bounded_analytic_mm(&renamed, tol.absolute()).unwrap(),
    );
    let loop_id = first(&step, "EDGE_LOOP");
    let (_, mut args) = entity(&step, &loop_id);
    let mut refs = fields(
        args[1]
            .strip_prefix('(')
            .unwrap()
            .strip_suffix(')')
            .unwrap(),
    );
    refs.rotate_left(1);
    args[1] = format!("({})", refs.join(","));
    check_geometry(
        result.solid(),
        &import_step_bounded_analytic_mm(&replace(&step, &loop_id, &args), tol.absolute()).unwrap(),
    );
    let circle2 = step
        .lines()
        .filter(|line| line.contains("=CIRCLE("))
        .map(|line| line.split_once('=').unwrap().0)
        .find(|id| {
            let (_, a) = entity(&step, id);
            entity(&step, &a[1]).0 == "AXIS2_PLACEMENT_2D"
        })
        .unwrap()
        .to_owned();
    let (_, mut radius) = entity(&step, &circle2);
    radius[2] = "9.123".into();
    let axis = entity(&step, &circle2).1[1].clone();
    let dir = entity(&step, &axis).1[2].clone();
    let (_, mut direction) = entity(&step, &dir);
    direction[1] = "(0.6,0.8)".into();
    let line2 = step
        .lines()
        .filter(|line| line.contains("=LINE("))
        .map(|line| line.split_once('=').unwrap().0)
        .find(|id| {
            let (_, a) = entity(&step, id);
            entity(&step, &a[1]).1[1].matches(',').count() == 1
        })
        .unwrap()
        .to_owned();
    let vec_id = entity(&step, &line2).1[2].clone();
    let (_, mut vector) = entity(&step, &vec_id);
    vector[2] = "1.2345".into();
    let ec = first(&step, "EDGE_CURVE");
    let (_, mut sense) = entity(&step, &ec);
    sense[4] = if sense[4] == ".T." { ".F." } else { ".T." }.into();
    let pc = first(&step, "PCURVE");
    let (_, mut owner) = entity(&step, &pc);
    owner[1] = step
        .lines()
        .filter(|line| line.contains("=CYLINDRICAL_SURFACE("))
        .map(|line| line.split_once('=').unwrap().0.to_owned())
        .find(|id| *id != owner[1])
        .unwrap();
    let circle3 = step
        .lines()
        .filter(|line| line.contains("=CIRCLE("))
        .map(|line| line.split_once('=').unwrap().0)
        .find(|id| {
            let (_, a) = entity(&step, id);
            entity(&step, &a[1]).0 == "AXIS2_PLACEMENT_3D"
        })
        .unwrap();
    let (_, mut tiny) = entity(&step, circle3);
    tiny[2] = "1e-14".into();
    let placement = entity(&step, circle3).1[1].clone();
    let axis3 = entity(&step, &placement).1[2].clone();
    let (_, mut reflection) = entity(&step, &axis3);
    let coordinates = reflection[1]
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    reflection[1] = format!(
        "({})",
        fields(coordinates)
            .iter()
            .map(|s| (-s.parse::<f64>().unwrap()).to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    for bad in [
        replace(&step, &circle2, &radius),
        replace(&step, &dir, &direction),
        replace(&step, &vec_id, &vector),
        replace(&step, &ec, &sense),
        replace(&step, &pc, &owner),
        replace(&step, circle3, &tiny),
        // Reversing the actual 3D circle normal reflects the short quarter
        // into an incompatible long sweep; raw 2D geometry remains untouched.
        replace(&step, &axis3, &reflection),
    ] {
        assert!(
            import_step_bounded_analytic_mm(&bad, tol.absolute()).is_err(),
            "tampered actual geometry must reject"
        );
    }
}
