use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn pose() -> Frame3 {
    Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap()
}
fn close(a: f64, b: f64, scale: f64) {
    assert!((a - b).abs() <= 2e-11 * scale, "{a} vs {b}");
}
// Integrate expanded monomials exactly in real arithmetic. This independent
// oracle uses neither the production mass method nor its Gauss quadrature.
fn integral(a: f64, delta: f64, power: usize, moment: usize) -> f64 {
    let mut binomial = 1.;
    let mut sum = 0.;
    for k in 0..=power {
        if k > 0 {
            binomial *= (power + 1 - k) as f64 / k as f64;
        }
        sum +=
            binomial * a.powi((power - k) as i32) * delta.powi(k as i32) / (moment + k + 1) as f64;
    }
    sum
}
fn moments(r: [f64; 2], h: f64) -> (f64, f64, [f64; 3]) {
    let a = integral(r[0], r[1] - r[0], 2, 0);
    let b = integral(r[0], r[1] - r[0], 2, 1);
    let c = integral(r[0], r[1] - r[0], 2, 2);
    let d = integral(r[0], r[1] - r[0], 4, 0);
    let radial = PI * h * d / 4.;
    let axial = PI * h.powi(3) * (c - b * b / a);
    (
        PI * h * a,
        h * b / a,
        [radial + axial, radial + axial, 2. * radial],
    )
}
fn coordinate(p: Vec3, i: usize) -> f64 {
    [p.x, p.y, p.z][i]
}
fn check_metrics(part: &NurbsFrustumSolid, p: GeometryTolerance) {
    let (volume, z, local) = moments(part.radii(), part.height());
    close(part.volume(p).unwrap(), volume, volume);
    let mass = part.mass_properties(p).unwrap();
    close(mass.volume, volume, volume);
    let expected = part.frame().point(Point3::new(0., 0., z));
    assert!((mass.centroid - expected).norm() <= 2e-11 * part.height());
    let inertia = part.inertia_properties(p).unwrap();
    close(inertia.volume, volume, volume);
    assert!((inertia.centroid - expected).norm() <= 2e-11 * part.height());
    let axes = part.frame().axes();
    for i in 0..3 {
        for j in 0..3 {
            let expected: f64 = (0..3)
                .map(|k| coordinate(axes[k], i) * coordinate(axes[k], j) * local[k])
                .sum();
            close(inertia.inertia[i][j], expected, local[0].max(local[2]));
            assert_eq!(inertia.inertia[i][j], inertia.inertia[j][i]);
        }
    }
    let bounds = part.bounds(p).unwrap();
    // Independently sample each end disk's exact coordinate extremum direction.
    for i in 0..3 {
        let u = coordinate(axes[0], i);
        let v = coordinate(axes[1], i);
        let norm = u.hypot(v);
        let mut extrema = Vec::new();
        for (z, radius) in [(0., part.radii()[0]), (part.height(), part.radii()[1])] {
            for sign in [-1., 1.] {
                let local = if norm == 0. {
                    Point3::new(0., 0., z)
                } else {
                    Point3::new(sign * radius * u / norm, sign * radius * v / norm, z)
                };
                extrema.push(coordinate(part.frame().point(local), i));
            }
        }
        close(
            coordinate(bounds.min, i),
            extrema.iter().copied().fold(f64::INFINITY, f64::min),
            part.height().max(part.radii()[0]),
        );
        close(
            coordinate(bounds.max, i),
            extrema.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            part.height().max(part.radii()[0]),
        );
    }
}
fn check_geometry(part: &NurbsFrustumSolid, p: GeometryTolerance) {
    part.validate(p).unwrap();
    let body = part.solid();
    assert_eq!(
        (
            body.vertices.len(),
            body.edges.len(),
            body.shell.faces.len()
        ),
        (8, 12, 6)
    );
    let world = part
        .frame()
        .origin()
        .norm()
        .max(part.height())
        .max(part.radii()[0])
        .max(part.radii()[1]);
    let guard = 8192. * f64::EPSILON * world;
    let mut uses = [(0, 0); 12];
    for (fi, face) in body.shell.faces.iter().enumerate() {
        for coedge in face.wires.iter().flat_map(|w| &w.coedges) {
            uses[coedge.edge].0 += 1;
            uses[coedge.edge].1 += face.orientation as i32 * if coedge.forward { 1 } else { -1 };
            for i in 0..=32 {
                let t = i as f64 / 32.;
                let uv = coedge.pcurve.try_evaluate(t).unwrap();
                let point = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                assert!(
                    (point - body.edges[coedge.edge].curve.try_evaluate(t).unwrap()).norm()
                        <= guard
                );
            }
        }
        if fi < 2 {
            assert_eq!(face.orientation, if fi == 0 { -1 } else { 1 });
            continue;
        }
        let Surface::Nurbs(surface) = &face.surface else {
            panic!("actual rational side");
        };
        assert_eq!(surface.degrees(), [2, 1]);
        assert_eq!(surface.control_counts(), [3, 2]);
        assert_eq!(surface.knots(0).unwrap(), [0., 0., 0., 1., 1., 1.]);
        assert_eq!(surface.knots(1).unwrap(), [0., 0., 1., 1.]);
        assert_eq!(
            surface.weights(),
            [1., 1., FRAC_1_SQRT_2, FRAC_1_SQRT_2, 1., 1.]
        );
        for i in 0..=16 {
            for j in 0..=16 {
                let u = i as f64 / 16.;
                let v = j as f64 / 16.;
                let point = part.frame().local_point(surface.evaluate(u, v).unwrap());
                let radius = part.radii()[0] * (1. - v) + part.radii()[1] * v;
                close(point.x.hypot(point.y), radius, world);
                close(point.z, part.height() * v, world);
                let expected = part
                    .frame()
                    .vector(Vec3::new(
                        point.x / radius,
                        point.y / radius,
                        (part.radii()[0] - part.radii()[1]) / part.height(),
                    ))
                    .normalized()
                    .unwrap();
                assert!((surface.normal(u, v).unwrap() - expected).norm() < 2e-12);
            }
        }
    }
    assert!(uses.iter().all(|&u| u == (2, 0)));
    for (i, edge) in body.edges.iter().enumerate() {
        if i < 8 {
            let Curve::Nurbs(curve) = &edge.curve else {
                panic!("actual rational circle rim");
            };
            assert_eq!(curve.degree(), 2);
            assert_eq!(curve.knots(), [0., 0., 0., 1., 1., 1.]);
            assert_eq!(curve.weights(), [1., FRAC_1_SQRT_2, 1.]);
            for k in 0..=32 {
                let q = part
                    .frame()
                    .local_point(curve.evaluate(k as f64 / 32.).unwrap());
                close(q.x.hypot(q.y), part.radii()[i / 4], world);
                close(q.z, if i < 4 { 0. } else { part.height() }, world);
            }
        } else {
            assert!(matches!(edge.curve, Curve::Line { .. }));
        }
    }
}
#[test]
fn exact_rational_geometry_and_polynomial_metrics_for_both_tapers_and_cylinder() {
    for radii in [[12., 6.], [6., 12.], [12., 12.]] {
        for frame in [Frame3::IDENTITY, pose()] {
            let p = policy(1.);
            let part = NurbsFrustumSolid::new(frame, radii, 20., p).unwrap();
            check_geometry(&part, p);
            check_metrics(&part, p);
            let raw = part.solid().clone();
            let restored = NurbsFrustumSolid::from_brep(raw.clone(), frame, radii, 20., p).unwrap();
            assert_eq!(format!("{:?}", restored.solid()), format!("{raw:?}"));
        }
    }
    for scale in [1e-60, 1e-4, 10., 1e60] {
        let part = NurbsFrustumSolid::new(
            Frame3::IDENTITY,
            [12. * scale, 6. * scale],
            20. * scale,
            policy(scale),
        )
        .unwrap();
        check_metrics(&part, policy(scale));
    }
}
fn key(p: Point3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}
fn check_mesh(part: &NurbsFrustumSolid, error: f64, p: GeometryTolerance, dense: bool) -> Mesh {
    let mesh = part.tessellate(error, p).unwrap();
    let world = part
        .frame()
        .origin()
        .norm()
        .max(part.height())
        .max(part.radii()[0])
        .max(part.radii()[1]);
    let guard = 8192. * f64::EPSILON * world;
    let mut nodes = BTreeMap::new();
    let mut incidence = BTreeMap::new();
    for vertex in &mesh.positions {
        let next = nodes.len();
        nodes.entry(key(*vertex)).or_insert(next);
    }
    let count = mesh.face_ids.iter().filter(|&&fi| fi == 2).count() / 2;
    let n = (count as f64).sqrt() as usize;
    assert_eq!(n * n, count);
    assert!(n.is_power_of_two());
    let directions = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let positions = tri.map(|i| mesh.positions[i]);
        let cross = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
        assert!(cross.norm() > 0.);
        assert!(cross.dot(mesh.normals[tri[0]]) > 0.);
        for k in 0..3 {
            let a = nodes[&key(positions[k])];
            let b = nodes[&key(positions[(k + 1) % 3])];
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = incidence.entry(edge).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
        if fi < 2 {
            let local = positions.map(|p| part.frame().local_point(p));
            assert!(local
                .iter()
                .all(|p| (p.z - if fi == 0 { 0. } else { part.height() }).abs() <= guard));
            continue;
        }
        let Surface::Nurbs(surface) = &part.solid().shell.faces[fi].surface else {
            panic!()
        };
        let q = fi - 2;
        let indices = positions.map(|point| {
            let local = part.frame().local_point(point);
            let x = local.x * directions[q][0] + local.y * directions[q][1];
            let y = local.x * directions[(q + 1) % 4][0] + local.y * directions[(q + 1) % 4][1];
            let k = y / (x.hypot(y) + x);
            let u = 2_f64.sqrt() * k / (1. + (2_f64.sqrt() - 1.) * k);
            let i = (u * n as f64).round() as usize;
            let j = (local.z / part.height() * n as f64).round() as usize;
            assert!(i <= n && j <= n);
            let actual = surface
                .evaluate(i as f64 / n as f64, j as f64 / n as f64)
                .unwrap();
            assert!((actual - point).norm() <= guard);
            [i, j]
        });
        if dense {
            for a in 0..=6 {
                for b in 0..=6 - a {
                    let weights = [6 - a - b, a, b];
                    let uv: [f64; 2] = std::array::from_fn(|axis| {
                        (0..3).map(|k| weights[k] * indices[k][axis]).sum::<usize>() as f64
                            / (6 * n) as f64
                    });
                    let exact = surface.evaluate(uv[0], uv[1]).unwrap();
                    let plane = positions[0]
                        + (positions[1] - positions[0]) * (a as f64 / 6.)
                        + (positions[2] - positions[0]) * (b as f64 / 6.);
                    assert!(
                        (exact - plane).norm() <= error + guard,
                        "face{fi} UV{uv:?} deviation{} error{error}",
                        (exact - plane).norm()
                    );
                }
            }
        }
    }
    assert!(incidence.values().all(|&(n, s)| n == 2 && s == 0));
    assert_eq!(
        nodes.len() as isize - incidence.len() as isize + mesh.triangles.len() as isize,
        2
    );
    mesh
}
#[test]
fn actual_surface_chords_include_mixed_parameters_and_mesh_volume_converges() {
    for frame in [Frame3::IDENTITY, pose()] {
        let part = NurbsFrustumSolid::new(frame, [12., 6.], 20., policy(1.)).unwrap();
        let exact = part.volume(policy(1.)).unwrap();
        let mut previous = f64::INFINITY;
        let mut triangles = 0;
        for error in [1., 0.2, 0.04] {
            let mesh = check_mesh(&part, error, policy(1.), true);
            let loss = exact - mesh.signed_volume();
            assert!(loss > 0.);
            assert!(loss < previous);
            assert!(mesh.triangles.len() > triangles);
            previous = loss;
            triangles = mesh.triangles.len();
        }
    }
    let cylinder = NurbsFrustumSolid::new(Frame3::IDENTITY, [6., 6.], 20., policy(1.)).unwrap();
    check_mesh(&cylinder, 0.3, policy(1.), true);
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
fn component(text: &str, name: &str) -> String {
    let start = text.find(&format!("{name}(")).unwrap() + name.len() + 1;
    let mut depth = 1;
    for (i, c) in text[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..start + i].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("unterminated component")
}
fn list(text: &str) -> Vec<String> {
    fields(text.strip_prefix('(').unwrap().strip_suffix(')').unwrap())
}
fn numbers(text: &str) -> Vec<f64> {
    list(text).iter().map(|x| x.parse().unwrap()).collect()
}
fn point(records: &BTreeMap<String, String>, reference: &str) -> Vec<f64> {
    numbers(&fields(&component(&records[reference], "CARTESIAN_POINT"))[1])
}
fn expanded(multiplicities: &str, knots: &str) -> Vec<f64> {
    let knots = numbers(knots);
    let mult = list(multiplicities);
    let mut result = Vec::new();
    for (k, n) in knots.into_iter().zip(mult) {
        result.extend(std::iter::repeat_n(k, n.parse::<usize>().unwrap()));
    }
    result
}
#[test]
fn step_retains_actual_rational_controls_weights_knots_and_mixed_plane_line_geometry() {
    let part = NurbsFrustumSolid::new(pose(), [12., 6.], 20., policy(1.)).unwrap();
    let step = part.export_step_mm(policy(1.)).unwrap();
    assert!(step.contains("AUTOMOTIVE_DESIGN"));
    assert!(step.contains("SI_UNIT(.MILLI.,.METRE.)"));
    let records: BTreeMap<_, _> = step
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            let (id, body) = line.split_once('=').unwrap();
            (id.to_owned(), body.to_owned())
        })
        .collect();
    assert_eq!(step.matches("=PLANE(").count(), 2);
    assert_eq!(step.matches("=EDGE_CURVE(").count(), 12);
    assert_eq!(step.matches("=SURFACE_CURVE(").count(), 12);
    assert_eq!(step.matches("=PCURVE(").count(), 24);
    assert_eq!(step.matches("=ADVANCED_FACE(").count(), 6);
    let surfaces: Vec<_> = step
        .lines()
        .filter(|line| line.contains("RATIONAL_B_SPLINE_SURFACE("))
        .collect();
    assert_eq!(surfaces.len(), 4);
    for (q, line) in surfaces.into_iter().enumerate() {
        let basis = fields(&component(line, "B_SPLINE_SURFACE"));
        assert_eq!(&basis[..2], ["2", "1"]);
        let Surface::Nurbs(actual) = &part.solid().shell.faces[2 + q].surface else {
            panic!()
        };
        let controls: Vec<_> = list(&basis[2])
            .iter()
            .flat_map(|row| {
                list(row)
                    .into_iter()
                    .map(|reference| point(&records, &reference))
            })
            .collect();
        assert_eq!(
            controls,
            actual
                .control_points()
                .iter()
                .map(|p| vec![p.x, p.y, p.z])
                .collect::<Vec<_>>()
        );
        let weights: Vec<_> = list(&fields(&component(line, "RATIONAL_B_SPLINE_SURFACE"))[0])
            .iter()
            .flat_map(|row| numbers(row))
            .collect();
        assert_eq!(weights, actual.weights());
        let knots = fields(&component(line, "B_SPLINE_SURFACE_WITH_KNOTS"));
        assert_eq!(expanded(&knots[0], &knots[2]), actual.knots(0).unwrap());
        assert_eq!(expanded(&knots[1], &knots[3]), actual.knots(1).unwrap());
    }
    let mut physical = 0;
    let mut uv = 0;
    for line in step
        .lines()
        .filter(|line| line.contains("RATIONAL_B_SPLINE_CURVE("))
    {
        let basis = fields(&component(line, "B_SPLINE_CURVE"));
        assert_eq!(basis[0], "2");
        let controls: Vec<_> = list(&basis[1])
            .iter()
            .map(|reference| point(&records, reference))
            .collect();
        let weights = numbers(&fields(&component(line, "RATIONAL_B_SPLINE_CURVE"))[0]);
        assert_eq!(weights, [1., FRAC_1_SQRT_2, 1.]);
        let knots = fields(&component(line, "B_SPLINE_CURVE_WITH_KNOTS"));
        assert_eq!(expanded(&knots[0], &knots[1]), [0., 0., 0., 1., 1., 1.]);
        if controls[0].len() == 3 {
            let Curve::Nurbs(actual) = &part.solid().edges[physical].curve else {
                panic!()
            };
            assert_eq!(
                controls,
                actual
                    .control_points()
                    .iter()
                    .map(|p| vec![p.x, p.y, p.z])
                    .collect::<Vec<_>>()
            );
            physical += 1;
        } else {
            let PCurve::Nurbs(actual) =
                &part.solid().shell.faces[uv / 4].wires[0].coedges[uv % 4].pcurve
            else {
                panic!()
            };
            assert_eq!(
                controls,
                actual
                    .control_points()
                    .iter()
                    .map(|p| vec![p.x, p.y])
                    .collect::<Vec<_>>()
            );
            uv += 1;
        }
    }
    assert_eq!((physical, uv), (8, 8));
    let mut generators = 0;
    for line in step.lines().filter(|line| line.contains("=LINE(")) {
        let args = fields(&component(line, "LINE"));
        let a = point(&records, &args[1]);
        if a.len() != 3 {
            continue;
        }
        let vector = fields(&component(&records[&args[2]], "VECTOR"));
        let direction = numbers(&fields(&component(&records[&vector[1]], "DIRECTION"))[1]);
        let magnitude: f64 = vector[2].parse().unwrap();
        let Curve::Line { a: actual_a, b } = &part.solid().edges[8 + generators].curve else {
            panic!()
        };
        assert_eq!(a, [actual_a.x, actual_a.y, actual_a.z]);
        assert!(
            (Vec3::new(direction[0], direction[1], direction[2]) * magnitude - (*b - *actual_a))
                .norm()
                < 8192. * f64::EPSILON * 40.
        );
        generators += 1;
    }
    assert_eq!(generators, 4);
    assert!(matches!(
        import_step_mm(&step, policy(1.).absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        import_step_bounded_analytic_mm(&step, policy(1.).absolute()),
        Err(Error::Unsupported(_))
    ));
}
fn altered_curve(curve: &NurbsCurve, control: bool, weights: bool) -> NurbsCurve {
    let mut points = curve.control_points().to_vec();
    let mut w = curve.weights().to_vec();
    if control {
        points[1].x += 1e-8;
    }
    if weights {
        w[1] *= 1.000001;
    }
    NurbsCurve::new(curve.degree(), curve.knots().to_vec(), points, w).unwrap()
}
#[test]
fn full_actual_certificate_rejects_controls_weights_uv_and_topology_corruptions_without_repair() {
    let part = NurbsFrustumSolid::new(pose(), [12., 6.], 20., policy(1.)).unwrap();
    let source = part.solid();
    let original = format!("{source:?}");
    let mut bads = Vec::new();
    let mut vertex = source.clone();
    vertex.vertices[0].point.x += 1e-12;
    bads.push(vertex);
    for (control, weights) in [(true, false), (false, true)] {
        let mut body = source.clone();
        let Curve::Nurbs(c) = &body.edges[0].curve else {
            panic!()
        };
        body.edges[0].curve = Curve::Nurbs(Box::new(altered_curve(c, control, weights)));
        bads.push(body);
    }
    let mut scaled = source.clone();
    let Curve::Nurbs(c) = &scaled.edges[0].curve else {
        panic!()
    };
    scaled.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            c.degree(),
            c.knots().to_vec(),
            c.control_points().to_vec(),
            c.weights().iter().map(|w| 2. * w).collect(),
        )
        .unwrap(),
    ));
    bads.push(scaled);
    for weight in [true, false] {
        let mut body = source.clone();
        let Surface::Nurbs(s) = &body.shell.faces[2].surface else {
            panic!()
        };
        let mut controls = s.control_points().to_vec();
        let mut w = s.weights().to_vec();
        if weight {
            w[2] *= 1.000001;
        } else {
            controls[1].z += 0.001;
        }
        body.shell.faces[2].surface = Surface::Nurbs(Box::new(
            NurbsSurface::new(
                s.degrees(),
                [s.knots(0).unwrap().to_vec(), s.knots(1).unwrap().to_vec()],
                s.control_counts(),
                controls,
                w,
            )
            .unwrap(),
        ));
        bads.push(body);
    }
    let mut uv = source.clone();
    let PCurve::Nurbs(c) = &uv.shell.faces[0].wires[0].coedges[0].pcurve else {
        panic!()
    };
    let mut points = c.control_points().to_vec();
    points[1].z = 1e-9;
    uv.shell.faces[0].wires[0].coedges[0].pcurve = PCurve::Nurbs(Box::new(
        NurbsCurve::new(c.degree(), c.knots().to_vec(), points, c.weights().to_vec()).unwrap(),
    ));
    bads.push(uv);
    let mut affine = source.clone();
    if let PCurve::Affine { origin, .. } = &mut affine.shell.faces[2].wires[0].coedges[0].pcurve {
        origin[0] += 1e-12;
    }
    bads.push(affine);
    let mut reference = source.clone();
    reference.shell.faces[2].wires[0].coedges[0].edge = 12;
    bads.push(reference);
    let mut endpoint = source.clone();
    endpoint.edges[0].vertices[0] = 8;
    bads.push(endpoint);
    let mut orientation = source.clone();
    orientation.shell.faces[2].orientation = -1;
    bads.push(orientation);
    let mut reversed = source.clone();
    reversed.shell.faces[2].wires[0].coedges[0].forward = false;
    bads.push(reversed);
    for body in bads {
        let before = format!("{body:?}");
        assert!(NurbsFrustumSolid::from_brep(
            body.clone(),
            part.frame(),
            part.radii(),
            part.height(),
            policy(1.)
        )
        .is_err());
        assert_eq!(before, format!("{body:?}"));
    }
    assert_eq!(original, format!("{:?}", part.solid()));
}
#[test]
fn unresolved_apices_frames_metrics_and_display_refuse_explicitly_without_generic_scope_changes() {
    for radii in [
        [0., 6.],
        [12., 0.],
        [-1., 6.],
        [f64::NAN, 6.],
        [f64::INFINITY, 6.],
        [1e-320, 6.],
        [1e-12, 1e12],
        [1e308, 1e308],
    ] {
        assert!(NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 20., policy(1.)).is_err());
    }
    for height in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 6.], height, policy(1.)).is_err());
    }
    assert!(Frame3::new_with_tolerance(
        Point3::new(0., 0., 0.),
        [
            Vec3::new(1., 0., 0.),
            Vec3::new(0.01, 1., 0.),
            Vec3::new(0., 0., 1.)
        ],
        policy(1.)
    )
    .is_err());
    let far = Frame3::translation(Vec3::new(1e14, 0., 0.)).unwrap();
    assert!(matches!(
        NurbsFrustumSolid::new(far, [12., 6.], 20., policy(1.)),
        Err(Error::Unsupported(_))
    ));
    assert!(NurbsFrustumSolid::new(
        Frame3::IDENTITY,
        [12., 6.],
        20.,
        GeometryTolerance::new(1e-6, 1e-10, 0.1).unwrap()
    )
    .is_err());
    let part = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 6.], 20., policy(1.)).unwrap();
    for error in [0., -1., f64::NAN, f64::INFINITY, 1e-12] {
        assert!(part.tessellate(error, policy(1.)).is_err());
    }
    let raw = part.solid();
    assert!(matches!(
        raw.validate(policy(1.).absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(raw.volume(), Err(Error::Unsupported(_))));
    assert!(matches!(
        raw.tessellate(0.5, policy(1.).absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        export_step_mm(raw, policy(1.).absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        export_step_bounded_analytic_mm(raw, 1e-6),
        Err(Error::Unsupported(_))
    ));
    for scale in [1e-70, 1e65] {
        let part = NurbsFrustumSolid::new(
            Frame3::IDENTITY,
            [12. * scale, 6. * scale],
            20. * scale,
            policy(scale),
        )
        .unwrap();
        assert!(part.volume(policy(scale)).unwrap().is_finite());
        assert!(matches!(
            part.inertia_properties(policy(scale)),
            Err(Error::Unsupported(_))
        ));
    }
    let scale = 1e-110;
    let part = NurbsFrustumSolid::new(
        Frame3::IDENTITY,
        [12. * scale, 6. * scale],
        20. * scale,
        policy(scale),
    )
    .unwrap();
    assert!(matches!(
        part.volume(policy(scale)),
        Err(Error::Unsupported(_))
    ));
}
