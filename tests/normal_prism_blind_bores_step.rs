use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

fn euler(s: &Solid) -> isize {
    s.vertices.len() as isize - s.edges.len() as isize
        + s.shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>()
}
fn check_closed(s: &Solid, scale: f64, world_scale: f64) {
    let tol = Tolerance::new(1e-6 * scale).unwrap();
    s.validate(tol).unwrap();
    let guard = 8192. * f64::EPSILON * world_scale;
    let mut uses = vec![Vec::new(); s.edges.len()];
    for (fi, f) in s.shell.faces.iter().enumerate() {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push((fi, c.forward == (f.orientation == 1)));
            let curve = &s.edges[c.edge].curve;
            let range = curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - curve.try_evaluate(t).unwrap())
                    .norm()
                        <= guard
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0].1 != u[1].1));
    let error = 0.02 * scale;
    let mesh = s.tessellate(error, tol).unwrap();
    let mut nodes: Vec<_> = s.vertices.iter().map(|v| v.point).collect();
    for (ei, e) in s.edges.iter().enumerate() {
        if let Curve::Arc { radius, sweep, .. } = e.curve {
            let fi = uses[ei]
                .iter()
                .map(|u| u.0)
                .find(|&i| matches!(s.shell.faces[i].surface, Surface::FramedCylinder { .. }))
                .unwrap();
            let n = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
            assert!(radius * (1. - (sweep / (2. * n as f64)).cos()) <= error);
            let range = e.curve.range();
            for k in 1..n {
                nodes.push(
                    e.curve
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
            let found: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, q)| (**q - *p).norm() <= guard)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(found.len(), 1);
            found[0]
        })
        .collect();
    let mut incidence = BTreeMap::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for k in 0..3 {
            let a = ids[tri[k]];
            let b = ids[tri[(k + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = incidence.entry(key).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
        let midpoint = (p[0] + p[1] + p[2]) * (1. / 3.);
        match s.shell.faces[fi].surface {
            Surface::FramedCylinder { frame, radius, .. } => {
                let q = frame.local_point(midpoint);
                assert!((q.x.hypot(q.y) - radius).abs() <= error + guard);
            }
            Surface::Plane { origin, u, v } => {
                assert!((midpoint - origin).dot(u.cross(v)).abs() <= guard)
            }
            _ => panic!("actual analytic BRep"),
        }
    }
    assert!(incidence.values().all(|&(n, d)| n == 2 && d == 0));
    assert_eq!(
        nodes.len() as isize - incidence.len() as isize + mesh.triangles.len() as isize,
        euler(s)
    );
}

fn check_retained_curves(source: &Solid, kept: &Solid, guard: f64) {
    for e in &source.edges {
        let range = e.curve.range();
        assert!(
            kept.edges.iter().any(|q| {
                let qr = q.curve.range();
                [false, true].into_iter().any(|reverse| {
                    (0..=16).all(|i| {
                        let t = i as f64 / 16.;
                        let u = if reverse { 1. - t } else { t };
                        (e.curve
                            .try_evaluate(range[0] + (range[1] - range[0]) * t)
                            .unwrap()
                            - q.curve.try_evaluate(qr[0] + (qr[1] - qr[0]) * u).unwrap())
                        .norm()
                            <= guard
                    })
                })
            }),
            "retained original line/arc geometry"
        );
    }
}

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn block(scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-10. * scale, -8. * scale, 0.),
            size: Vec3::new(20. * scale, 16. * scale, 5. * scale),
        },
        policy(scale).absolute(),
    )
    .unwrap()
}
fn plane(x: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(x, 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    }
}
fn equivalent(expected: &Solid, actual: &Solid, scale: f64) {
    let world = expected.vertices.iter().fold(30. * scale, |w, v| {
        w.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let guard = 8192. * f64::EPSILON * world;
    actual.validate(policy(scale).absolute()).unwrap();
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
    assert_eq!(euler(actual), euler(expected));
    assert!((actual.volume().unwrap() - expected.volume().unwrap()).abs() < 1e-8 * scale.powi(3));
    assert!((actual.bounds().min - expected.bounds().min).norm() < guard);
    assert!((actual.bounds().max - expected.bounds().max).norm() < guard);
    check_retained_curves(expected, actual, guard);
    check_retained_curves(actual, expected, guard);
    check_closed(actual, scale, world);
}
fn roundtrip(s: &Solid, scale: f64) {
    let text = export_step_bounded_analytic_mm(s, policy(scale).linear()).unwrap();
    let parsed = import_step_bounded_analytic_mm(&text, policy(scale).absolute()).unwrap();
    equivalent(s, &parsed, scale);
    let text2 = export_step_bounded_analytic_mm(&parsed, policy(scale).linear()).unwrap();
    let second = import_step_bounded_analytic_mm(&text2, policy(scale).absolute()).unwrap();
    equivalent(&parsed, &second, scale);
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

fn permute(step: &str) -> String {
    let mut records: Vec<_> = step.lines().filter(|l| l.starts_with('#')).collect();
    records.reverse();
    let mut reordered = String::new();
    let mut used = false;
    for line in step.lines() {
        if line.starts_with('#') {
            if !used {
                reordered.push_str(&records.join("\n"));
                reordered.push('\n');
                used = true;
            }
        } else {
            reordered.push_str(line);
            reordered.push('\n');
        }
    }
    let chars: Vec<_> = reordered.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut quote = false;
    while i < chars.len() {
        if chars[i] == '\'' {
            quote = !quote;
        }
        if chars[i] == '#' && !quote {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let id: usize = chars[start..i].iter().collect::<String>().parse().unwrap();
            out.push_str(&format!("#{}", 10000 - id));
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}
fn spec(x: f64, y: f64, r: f64, d: f64, top: bool, scale: f64) -> NormalPrismBlindBoreSpec {
    NormalPrismBlindBoreSpec {
        center: Point3::new(x * scale, y * scale, if top { 5. * scale } else { 0. }),
        radius: r * scale,
        depth: d * scale,
        entry: if top {
            NormalPrismBoreEntry::Positive
        } else {
            NormalPrismBoreEntry::Negative
        },
    }
}
fn make(source: &Solid, specs: &[NormalPrismBlindBoreSpec], scale: f64) -> Solid {
    blind_bores_normal_prism(source, specs, Vec3::new(0., 0., 1.), policy(scale))
        .unwrap()
        .kept()
        .clone()
}
#[test]
fn multi_pocket_roundtrips_preserve_two_three_sixteen_mixed_entry_actual_breps() {
    for scale in [1e-4, 1., 10.] {
        let source = block(scale);
        let specs = [
            spec(-4., -2., 1., 2., true, scale),
            spec(4., 2., 0.75, 1.5, false, scale),
            spec(0., 3., 0.5, 3., true, scale),
        ];
        for count in [2, 3] {
            let body = make(&source, &specs[..count], scale);
            let cut: f64 = specs[..count]
                .iter()
                .map(|s| PI * s.radius * s.radius * s.depth)
                .sum();
            assert!(
                (body.volume().unwrap() - (1600. * scale.powi(3) - cut)).abs()
                    < 1e-8 * scale.powi(3)
            );
            roundtrip(&body, scale);
        }
    }
    let source = block(1.);
    let sixteen: Vec<_> = (0..16)
        .map(|i| {
            spec(
                -7.5 + 5. * (i % 4) as f64,
                -6. + 4. * (i / 4) as f64,
                0.6,
                1. + 0.5 * (i % 3) as f64,
                i % 2 == 0,
                1.,
            )
        })
        .collect();
    roundtrip(&make(&source, &sixteen, 1.), 1.);
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., policy(1.).absolute())
            .unwrap();
    let rounded = extrude_arc_line(&profile, 5., policy(1.).absolute()).unwrap();
    let through = bore_normal_prism(
        &rounded,
        Point3::new(4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    let body = make(
        through.kept(),
        &[
            spec(-4., 0., 1., 2., true, 1.),
            spec(0., 3., 0.5, 1., false, 1.),
        ],
        1.,
    );
    assert_eq!(euler(&body), 0);
    roundtrip(&body, 1.);
    let split = split_normal_prism_by_plane_components(
        &source,
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    roundtrip(
        &make(
            &split.negative()[0],
            &[
                spec(-4., -3., 1., 2., true, 1.),
                spec(-4., 3., 1., 1., false, 1.),
            ],
            1.,
        ),
        1.,
    );
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let posed = source
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let specs = [
        spec(-4., -2., 1., 2., true, 1.),
        spec(4., 2., 0.75, 1.5, false, 1.),
    ]
    .map(|mut s| {
        s.center = transform.point(s.center);
        s
    });
    let result = blind_bores_normal_prism(
        &posed,
        &specs,
        transform.vector(Vec3::new(0., 0., 1.)),
        policy(1.),
    )
    .unwrap();
    roundtrip(result.kept(), 1.);
    let small = make(
        &block(0.001),
        &[
            spec(-4., -2., 1., 2., true, 0.001),
            spec(4., 2., 0.75, 1.5, false, 0.001),
        ],
        0.001,
    );
    let text = export_step_bounded_analytic_mm(&small, 1e-9)
        .unwrap()
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let mm = make(
        &source,
        &[
            spec(-4., -2., 1., 2., true, 1.),
            spec(4., 2., 0.75, 1.5, false, 1.),
        ],
        1.,
    );
    equivalent(
        &mm,
        &import_step_bounded_analytic_mm(&text, policy(1.).absolute()).unwrap(),
        1.,
    );
}
#[test]
fn entity_order_floor_loops_face_order_and_geometric_uv_origins_are_equivalent() {
    let source = block(1.);
    let specs = [
        spec(-4., -2., 1., 2., true, 1.),
        spec(4., 2., 0.75, 1.5, false, 1.),
        spec(0., 3., 0.5, 3., true, 1.),
    ];
    let result =
        blind_bores_normal_prism(&source, &specs, Vec3::new(0., 0., 1.), policy(1.)).unwrap();
    let text = export_step_bounded_analytic_mm(result.kept(), 1e-6).unwrap();
    equivalent(
        result.kept(),
        &import_step_bounded_analytic_mm(&permute(&text), policy(1.).absolute()).unwrap(),
        1.,
    );
    let mut reordered = result.kept().clone();
    for &fi in result.floor_faces() {
        reordered.shell.faces[fi].wires[0].coedges.rotate_left(1);
    }
    reordered.shell.faces.rotate_left(4);
    roundtrip(&reordered, 1.);
    let mut shifted = result.kept().clone();
    for &fi in result.floor_faces() {
        let floor = &mut shifted.shell.faces[fi];
        let Surface::Plane { origin, u, v } = &mut floor.surface else {
            unreachable!()
        };
        *origin = *origin + *u * 3. + *v * 2.;
        for c in &mut floor.wires[0].coedges {
            match &mut c.pcurve {
                PCurve::Arc { center, .. } => {
                    center[0] -= 3.;
                    center[1] -= 2.;
                }
                _ => panic!("actual quarter floor UV"),
            }
        }
    }
    roundtrip(&shifted, 1.);
    let legacy = include_str!("../docs/step-two-quarter-blind-pockets.step");
    let expected = make(
        &source,
        &[
            spec(-4., 0., 1., 2., true, 1.),
            spec(4., 0., 1., 2., true, 1.),
        ],
        1.,
    );
    equivalent(
        &expected,
        &import_step_bounded_analytic_mm(legacy, policy(1.).absolute()).unwrap(),
        1.,
    );
}
fn trusted_graft(source: &Solid, specs: &[NormalPrismBlindBoreSpec]) -> Solid {
    let mut body = source.clone();
    for s in specs {
        let single = blind_bore_normal_prism(
            source,
            s.center,
            s.radius,
            s.depth,
            Vec3::new(0., 0., 1.),
            s.entry,
            policy(1.),
        )
        .unwrap();
        let other = single.kept();
        let vs = body.vertices.len() - source.vertices.len();
        let es = body.edges.len() - source.edges.len();
        body.vertices
            .extend(other.vertices[source.vertices.len()..].iter().cloned());
        body.edges.extend(
            other.edges[source.edges.len()..]
                .iter()
                .cloned()
                .map(|mut e| {
                    e.vertices = e.vertices.map(|i| i + vs);
                    e
                }),
        );
        for (i, f) in source.shell.faces.iter().enumerate() {
            for wire in &other.shell.faces[i].wires[f.wires.len()..] {
                let mut w = wire.clone();
                for c in &mut w.coedges {
                    c.edge += es;
                }
                body.shell.faces[i].wires.push(w);
            }
        }
        body.shell.faces.extend(
            other.shell.faces[source.shell.faces.len()..]
                .iter()
                .cloned()
                .map(|mut f| {
                    for c in f.wires.iter_mut().flat_map(|w| &mut w.coedges) {
                        c.edge += es;
                    }
                    f
                }),
        );
    }
    body
}
#[test]
fn valid_opposing_pockets_with_axial_web_and_seventeen_pockets_are_unsupported() {
    let source = block(1.);
    let before = format!("{source:?}");
    let specs = [
        spec(0., 0., 1., 2., true, 1.),
        spec(0., 0., 1., 2., false, 1.),
    ];
    let opposing = trusted_graft(&source, &specs);
    opposing.validate(policy(1.).absolute()).unwrap();
    assert_eq!(euler(&opposing), 2);
    assert!((opposing.volume().unwrap() - (1600. - 4. * PI)).abs() < 1e-8);
    check_closed(&opposing, 1., 200.);
    assert_eq!(
        classify_point_in_solid(&opposing, Point3::new(0., 0., 2.5), policy(1.)).unwrap(),
        PointLocation::Inside
    );
    let text = export_step_bounded_analytic_mm(&opposing, 1e-6).unwrap();
    if let Ok(dir) = std::env::var("HAGANE_BLIND_STEP_FIXTURE_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join("step-opposing-quarter-blind-pockets.step"),
            &text,
        )
        .unwrap();
    }
    let rejection = import_step_bounded_analytic_mm(&text, policy(1.).absolute());
    assert!(
        matches!(rejection, Err(Error::Unsupported(_))),
        "{rejection:?}"
    );
    let many: Vec<_> = (0..17)
        .map(|i| {
            spec(
                -8. + 4. * (i % 5) as f64,
                -6. + 4. * (i / 5) as f64,
                0.4,
                1.,
                true,
                1.,
            )
        })
        .collect();
    let body = trusted_graft(&source, &many);
    body.validate(policy(1.).absolute()).unwrap();
    assert_eq!(euler(&body), 2);
    assert!((body.volume().unwrap() - (1600. - 17. * 0.16 * PI)).abs() < 1e-8);
    check_closed(&body, 1., 200.);
    let text = export_step_bounded_analytic_mm(&body, 1e-6).unwrap();
    let rejection = import_step_bounded_analytic_mm(&text, policy(1.).absolute());
    assert!(
        matches!(rejection, Err(Error::Unsupported(_))),
        "{rejection:?}"
    );
    assert_eq!(before, format!("{source:?}"));
}
#[test]
fn actual_multi_floor_sense_pcurve_and_cylinder_tampering_rejects_and_operations_remain_scoped() {
    let source = block(1.);
    let specs = [
        spec(-4., 0., 1., 2., true, 1.),
        spec(4., 0., 0.75, 1., false, 1.),
    ];
    let result =
        blind_bores_normal_prism(&source, &specs, Vec3::new(0., 0., 1.), policy(1.)).unwrap();
    let step = export_step_bounded_analytic_mm(result.kept(), 1e-6).unwrap();
    let parsed = import_step_bounded_analytic_mm(&step, policy(1.).absolute()).unwrap();
    assert!(bore_normal_prism(
        &parsed,
        Point3::new(0., 0., 0.),
        0.5,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    assert!(split_normal_prism_by_plane_components(
        &parsed,
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let faces: Vec<_> = step
        .lines()
        .filter(|l| l.contains("=ADVANCED_FACE("))
        .map(|l| l.split_once('=').unwrap().0.to_owned())
        .collect();
    let floor = &faces[result.floor_faces()[1]];
    let (_, mut sense) = entity(&step, floor);
    sense[3] = if sense[3] == ".T." { ".F." } else { ".T." }.into();
    let cylinder = first(&step, "CYLINDRICAL_SURFACE");
    let (_, mut radius) = entity(&step, &cylinder);
    radius[2] = "1.25".into();
    let circle2 = step
        .lines()
        .filter(|l| l.contains("=CIRCLE("))
        .map(|l| l.split_once('=').unwrap().0)
        .find(|id| {
            let (_, a) = entity(&step, id);
            entity(&step, &a[1]).0 == "AXIS2_PLACEMENT_2D"
        })
        .unwrap();
    let (_, mut uv) = entity(&step, circle2);
    uv[2] = "1.25".into();
    for bad in [
        replace(&step, floor, &sense),
        replace(&step, &cylinder, &radius),
        replace(&step, circle2, &uv),
    ] {
        assert!(import_step_bounded_analytic_mm(&bad, policy(1.).absolute()).is_err());
    }
}
