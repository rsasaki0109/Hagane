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
            let point = actual.edges[coedge.edge].curve.try_evaluate(0.5).unwrap();
            let other_uv = other.surface.try_parameters(point).unwrap();
            let b = other.surface.normal(other_uv[0]) * f64::from(other.orientation);
            assert!(a.dot(b) > 1. - 1e-12, "import preserves physical tangency");
        }
    }
}

fn holed_region(scale: f64) -> ArcLineRegion {
    let t = Tolerance::new(1e-6 * scale).unwrap();
    let outer = rounded_rectangle_profile(
        Point3::new(0., 0., 0.),
        20. * scale,
        16. * scale,
        2. * scale,
        t,
    )
    .unwrap();
    let points = [[-5., -1.], [-3., -1.], [-3., 1.], [-5., 1.]];
    let rectangle = (0..4)
        .map(|i| PlanarSegment::Line {
            a: points[i].map(|x| x * scale),
            b: points[(i + 1) % 4].map(|x| x * scale),
        })
        .collect();
    let circle = (0..4)
        .map(|i| PlanarSegment::Arc {
            center: [4. * scale, 0.],
            radius: 1.5 * scale,
            start_angle: i as f64 * PI / 2.,
            sweep: PI / 2.,
        })
        .collect();
    ArcLineRegion {
        origin: outer.origin,
        outer: outer.segments,
        holes: vec![rectangle, circle],
    }
}

fn review_holed_mesh(body: &Solid, frame: Transform) {
    let error = 0.02;
    let mesh = body
        .tessellate(error, Tolerance::new(1e-6).unwrap())
        .unwrap();
    let guard = 4096.
        * f64::EPSILON
        * mesh
            .positions
            .iter()
            .fold(1_f64, |w, p| w.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
    let mut nodes: Vec<_> = body.vertices.iter().map(|v| v.point).collect();
    for (ei, edge) in body.edges.iter().enumerate() {
        if let Curve::Arc { radius, sweep, .. } = edge.curve {
            let fi = body
                .shell
                .faces
                .iter()
                .enumerate()
                .find(|(_, f)| {
                    matches!(f.surface, Surface::FramedCylinder { .. })
                        && f.wires[0].coedges.iter().any(|c| c.edge == ei)
                })
                .unwrap()
                .0;
            let n = mesh.face_ids.iter().filter(|&&id| id == fi).count() / 2;
            assert!(radius * (1. - (sweep / (2. * n as f64)).cos()) <= error);
            let range = edge.curve.range();
            for k in 1..n {
                nodes.push(
                    edge.curve
                        .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / n as f64)
                        .unwrap(),
                );
            }
        }
    }
    let ids: Vec<_> = mesh
        .positions
        .iter()
        .map(|p| {
            let matches: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, q)| (**q - *p).norm() <= guard)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(matches.len(), 1);
            matches[0]
        })
        .collect();
    let mut uses = std::collections::BTreeMap::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let points = tri.map(|i| mesh.positions[i]);
        assert!(
            (points[1] - points[0])
                .cross(points[2] - points[0])
                .dot(mesh.normals[tri[0]])
                > 0.
        );
        for i in 0..3 {
            let a = ids[tri[i]];
            let b = ids[tri[(i + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(key).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
        let surface = &body.shell.faces[fi].surface;
        for weights in [[1. / 3.; 3], [0.2, 0.3, 0.5], [0.7, 0.2, 0.1]] {
            let p = points[0] * weights[0] + points[1] * weights[1] + points[2] * weights[2];
            match surface {
                Surface::FramedCylinder { frame, radius, .. } => {
                    let q = frame.local_point(p);
                    assert!((q.x.hypot(q.y) - radius).abs() <= error + guard);
                }
                Surface::Plane { origin, u, v } => {
                    assert!((p - *origin).dot(u.cross(*v)).abs() <= guard);
                    let local = frame.local_point(p);
                    assert!(
                        !(local.x > -5. + guard
                            && local.x < -3. - guard
                            && local.y > -1. + guard
                            && local.y < 1. - guard)
                    );
                    // Circular trim chords may enter the analytic hole only
                    // within the explicitly bounded display chord tolerance.
                    assert!((local.x - 4.).hypot(local.y) >= 1.5 - error - guard);
                }
                _ => panic!("bounded actual surface"),
            }
        }
    }
    assert!(uses.values().all(|&(n, sign)| n == 2 && sign == 0));
    assert_eq!(
        nodes.len() as isize - uses.len() as isize + mesh.triangles.len() as isize,
        -2
    );
}

#[test]
fn rectangular_and_circular_inner_wires_retain_genus_volume_units_and_closed_display() {
    let t = Tolerance::new(1e-6).unwrap();
    let source = extrude_arc_line_region(&holed_region(1.), 5., t).unwrap();
    let expected = (320. - 4. * 4. * (1. - PI / 4.) - 4. - PI * 1.5_f64.powi(2)) * 5.;
    assert!((source.volume().unwrap() - expected).abs() < 1e-9);
    assert_eq!(
        source.vertices.len() as isize - source.edges.len() as isize
            + source
                .shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>(),
        -2
    );
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    for pose in [Transform::IDENTITY, frame] {
        let placed = source.transformed(pose, t).unwrap();
        let imported = roundtrip(&placed);
        review_holed_mesh(&imported, pose);
        assert_eq!(
            imported
                .shell
                .faces
                .iter()
                .filter(|f| matches!(f.surface, Surface::Plane { .. }) && f.wires.len() == 3)
                .count(),
            2
        );
        for (p, location) in [
            (Point3::new(0., 0., 2.), PointLocation::Inside),
            (Point3::new(-4., 0., 2.), PointLocation::Outside),
            (Point3::new(4., 0., 2.), PointLocation::Outside),
        ] {
            assert_eq!(
                classify_point_in_solid(
                    &imported,
                    pose.point(p),
                    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
                )
                .unwrap(),
                location
            );
        }
    }
    let small = extrude_arc_line_region(&holed_region(0.001), 0.005, Tolerance::new(1e-9).unwrap())
        .unwrap();
    let metre = export_step_bounded_analytic_mm(&small, 1e-9)
        .unwrap()
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    check_geometry(
        &source,
        &import_step_bounded_analytic_mm(&metre, t).unwrap(),
    );
}

// Move all actual geometry belonging to the rectangular opening, including its
// cap pcurves. This preserves local same-parameter relationships while making
// the complete planar material region invalid.
fn shifted_rectangle_hole(mut body: Solid, offset: Vec3) -> Solid {
    let ids: std::collections::BTreeSet<_> = body
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| v.point.x >= -5. && v.point.x <= -3. && v.point.y.abs() <= 1.)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(ids.len(), 8);
    let edges: std::collections::BTreeSet<_> = body
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.iter().all(|i| ids.contains(i)))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(edges.len(), 12);
    for i in ids {
        body.vertices[i].point = body.vertices[i].point + offset;
    }
    for &i in &edges {
        let Curve::Line {
            ref mut a,
            ref mut b,
        } = body.edges[i].curve
        else {
            unreachable!()
        };
        *a = *a + offset;
        *b = *b + offset;
    }
    for face in &mut body.shell.faces {
        let Surface::Plane {
            ref mut origin,
            u,
            v,
        } = face.surface
        else {
            continue;
        };
        if face.wires.len() == 1
            && face.wires[0]
                .coedges
                .iter()
                .all(|c| edges.contains(&c.edge))
        {
            *origin = *origin + offset;
        } else {
            for coedge in face
                .wires
                .iter_mut()
                .flat_map(|w| &mut w.coedges)
                .filter(|c| edges.contains(&c.edge))
            {
                let PCurve::Affine { ref mut origin, .. } = coedge.pcurve else {
                    unreachable!()
                };
                origin[0] += offset.dot(u);
                origin[1] += offset.dot(v);
            }
        }
    }
    body
}

#[test]
fn coherent_outside_nested_and_touching_inner_boundaries_fail_import_certificate() {
    let t = Tolerance::new(1e-6).unwrap();
    let source = extrude_arc_line_region(&holed_region(1.), 5., t).unwrap();
    let original = export_step_bounded_analytic_mm(&source, t.linear).unwrap();
    for (label, offset) in [
        ("outside", Vec3::new(20., 0., 0.)),
        ("nested", Vec3::new(8., 0., 0.)),
        ("contact", Vec3::new(-5., 0., 0.)),
    ] {
        let invalid = shifted_rectangle_hole(source.clone(), offset);
        assert!(invalid.validate(t).is_err());
        // Mutate the encoded actual 3D coordinates and corresponding cap UV
        // points coherently, bypassing the writer's authoritative validation.
        let mut text = original.clone();
        for record in original
            .lines()
            .filter(|line| line.contains("=CARTESIAN_POINT("))
        {
            let id = record.split_once('=').unwrap().0;
            let (_, mut args) = entity(&original, id);
            let coordinates: Vec<f64> = fields(
                args[1]
                    .strip_prefix('(')
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap(),
            )
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
            let is_hole =
                coordinates[0] >= -5. && coordinates[0] <= -3. && coordinates[1].abs() <= 1.;
            if !is_hole {
                continue;
            }
            let mut shifted = coordinates;
            shifted[0] += offset.x;
            shifted[1] += offset.y;
            if shifted.len() == 3 {
                shifted[2] += offset.z;
            }
            args[1] = format!(
                "({})",
                shifted
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            text = replace(&text, id, &args);
        }
        assert!(import_step_bounded_analytic_mm(&text, t).is_err());
        if let Ok(directory) = std::env::var("HAGANE_REVIEW_FIXTURE_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("bounded-inner-{label}.step")),
                text,
            )
            .unwrap();
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
