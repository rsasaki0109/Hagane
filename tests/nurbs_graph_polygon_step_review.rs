use hagane::*;
use std::collections::BTreeMap;
fn fields(input: &str) -> Vec<String> {
    let text = input
        .trim()
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut start = 0;
    for (i, ch) in text.char_indices() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                parts.push(text[start..i].trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim().to_owned());
    parts
}
fn components(input: &str) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::new();
    let mut rest = input
        .trim()
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .trim();
    while !rest.is_empty() {
        let open = rest.find('(').unwrap();
        let kind = rest[..open].trim().to_owned();
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in rest[open..].char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
        }
        result.insert(kind, fields(&rest[open..end]));
        rest = rest[end..].trim();
    }
    result
}
fn records(step: &str) -> BTreeMap<usize, (String, Vec<String>)> {
    step.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('#') {
                return None;
            }
            let (id, body) = line[1..].split_once('=').unwrap();
            let body = body.trim_end_matches(';');
            let decoded = if body.starts_with('(') {
                let c = components(body);
                if let Some(base) = c.get("B_SPLINE_CURVE") {
                    let mut args = vec!["''".into()];
                    args.extend(base.clone());
                    args.extend(c["B_SPLINE_CURVE_WITH_KNOTS"].clone());
                    args.push(c["RATIONAL_B_SPLINE_CURVE"][0].clone());
                    ("B_SPLINE_CURVE_WITH_KNOTS".into(), args)
                } else if c.contains_key("B_SPLINE_SURFACE") {
                    let mut args = vec!["''".into()];
                    args.extend(c["B_SPLINE_SURFACE"].clone());
                    args.extend(c["B_SPLINE_SURFACE_WITH_KNOTS"].clone());
                    args.push(c["RATIONAL_B_SPLINE_SURFACE"][0].clone());
                    ("B_SPLINE_SURFACE_WITH_KNOTS".into(), args)
                } else {
                    ("COMPLEX".into(), c.values().flatten().cloned().collect())
                }
            } else {
                let (kind, args) = body.split_once('(').unwrap();
                (kind.into(), fields(&format!("({args}")))
            };
            Some((id.parse().unwrap(), decoded))
        })
        .collect()
}
fn reference(s: &str) -> usize {
    s.trim_start_matches('#')
        .parse()
        .unwrap_or_else(|_| panic!("bad reference {s:?}"))
}
fn references(s: &str) -> Vec<usize> {
    fields(s).iter().map(|s| reference(s)).collect()
}
fn numbers(s: &str) -> Vec<f64> {
    fields(s).iter().map(|s| s.parse().unwrap()).collect()
}
fn point(records: &BTreeMap<usize, (String, Vec<String>)>, id: usize) -> Point3 {
    let (kind, args) = &records[&id];
    assert_eq!(kind, "CARTESIAN_POINT");
    let n = numbers(&args[1]);
    Point3::new(n[0], n[1], n[2])
}
fn knots(multiplicities: &str, values: &str) -> Vec<f64> {
    numbers(multiplicities)
        .iter()
        .zip(numbers(values))
        .flat_map(|(n, k)| std::iter::repeat_n(k, *n as usize))
        .collect()
}
fn basis(i: usize, d: usize, t: f64, k: &[f64], count: usize) -> f64 {
    if d == 0 {
        return if t == k[k.len() - 1] {
            f64::from(i == count - 1)
        } else {
            f64::from(k[i] <= t && t < k[i + 1])
        };
    }
    let a = k[i + d] - k[i];
    let b = k[i + d + 1] - k[i + 1];
    (if a == 0. {
        0.
    } else {
        (t - k[i]) / a * basis(i, d - 1, t, k, count)
    }) + (if b == 0. {
        0.
    } else {
        (k[i + d + 1] - t) / b * basis(i + 1, d - 1, t, k, count)
    })
}
fn verify(step: &str, solid: &Solid) {
    assert!(step.contains("MANIFOLD_SOLID_BREP"));
    assert!(step.contains(".MILLI.,.METRE."));
    assert!(!step.contains("TRIANGULATED_FACE_SET"));
    let r = records(step);
    let edges: Vec<_> = r
        .iter()
        .filter(|(_, (kind, _))| kind == "EDGE_CURVE")
        .collect();
    let faces: Vec<_> = r
        .iter()
        .filter(|(_, (kind, _))| kind == "ADVANCED_FACE")
        .collect();
    assert_eq!(edges.len(), solid.edges.len());
    assert_eq!(faces.len(), solid.shell.faces.len());
    let mut edge_uses = BTreeMap::<usize, Vec<bool>>::new();
    for ((_, (_, args)), expected) in edges.iter().zip(&solid.edges) {
        let raw = &r[&reference(&args[3])];
        assert_eq!(raw.0, "SURFACE_CURVE");
        assert_eq!(references(&raw.1[2]).len(), 2);
        let curve = &r[&reference(&raw.1[1])];
        assert_eq!(curve.0, "B_SPLINE_CURVE_WITH_KNOTS");
        let p: Vec<_> = references(&curve.1[2])
            .into_iter()
            .map(|id| point(&r, id))
            .collect();
        let d: usize = curve.1[1].parse().unwrap();
        let k = knots(&curve.1[6], &curve.1[7]);
        let Curve::Nurbs(actual) = &expected.curve else {
            panic!()
        };
        let weights = if curve.1.len() > 9 {
            numbers(&curve.1[9])
        } else {
            vec![1.; p.len()]
        };
        assert_eq!(d, actual.degree());
        assert_eq!(k, actual.knots());
        assert_eq!(p, actual.control_points());
        assert_eq!(weights, actual.weights());
        let domain = expected.curve.range();
        for j in 0..=16 {
            let t = domain[0] + (domain[1] - domain[0]) * j as f64 / 16.;
            let value = p
                .iter()
                .enumerate()
                .fold(Vec3::new(0., 0., 0.), |sum, (i, point)| {
                    sum + *point * (basis(i, d, t, &k, p.len()) * weights[i])
                });
            let denominator = (0..p.len())
                .map(|i| basis(i, d, t, &k, p.len()) * weights[i])
                .sum::<f64>();
            assert!(
                (value * (1. / denominator) - expected.curve.try_evaluate(t).unwrap()).norm()
                    < 1e-10
            );
        }
    }
    for ((_, (_, args)), expected) in faces.iter().zip(&solid.shell.faces) {
        assert_eq!(args[3] == ".T.", expected.orientation == 1);
        let surface = &r[&reference(&args[2])];
        assert_eq!(surface.0, "B_SPLINE_SURFACE_WITH_KNOTS");
        let degree = [
            surface.1[1].parse::<usize>().unwrap(),
            surface.1[2].parse::<usize>().unwrap(),
        ];
        let rows: Vec<_> = fields(&surface.1[3])
            .iter()
            .map(|row| references(row))
            .collect();
        let count = [rows.len(), rows[0].len()];
        let p: Vec<_> = rows.into_iter().flatten().map(|id| point(&r, id)).collect();
        let k = [
            knots(&surface.1[8], &surface.1[10]),
            knots(&surface.1[9], &surface.1[11]),
        ];
        let Surface::Nurbs(s) = &expected.surface else {
            panic!("retained rational face")
        };
        let weights = if surface.1.len() > 13 {
            fields(&surface.1[13])
                .iter()
                .flat_map(|row| numbers(row))
                .collect::<Vec<_>>()
        } else {
            vec![1.; p.len()]
        };
        assert_eq!(degree, s.degrees());
        assert_eq!(count, s.control_counts());
        assert_eq!(p, s.control_points());
        assert_eq!(weights, s.weights());
        assert_eq!(k[0], s.knots(0).unwrap());
        assert_eq!(k[1], s.knots(1).unwrap());
        for a in 0..=8 {
            for b in 0..=8 {
                let domain = s.domain();
                let uv = [
                    domain[0][0] + (domain[0][1] - domain[0][0]) * a as f64 / 8.,
                    domain[1][0] + (domain[1][1] - domain[1][0]) * b as f64 / 8.,
                ];
                let mut value = Vec3::new(0., 0., 0.);
                let mut denominator = 0.;
                for i in 0..count[0] {
                    for j in 0..count[1] {
                        let coefficient = basis(i, degree[0], uv[0], &k[0], count[0])
                            * basis(j, degree[1], uv[1], &k[1], count[1])
                            * weights[i * count[1] + j];
                        denominator += coefficient;
                        value = value + p[i * count[1] + j] * coefficient;
                    }
                }
                assert!(
                    (value * (1. / denominator) - s.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-10
                );
            }
        }
        let bounds = references(&args[1]);
        assert_eq!(bounds.len(), expected.wires.len());
        for (wire_index, (bound, wire)) in bounds.iter().zip(&expected.wires).enumerate() {
            let record = &r[bound];
            assert_eq!(
                record.0,
                if wire_index == 0 {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                }
            );
            assert_eq!(record.1[2], ".T.");
            let loop_ = &r[&reference(&record.1[1])];
            assert_eq!(loop_.0, "EDGE_LOOP");
            let uses = references(&loop_.1[1]);
            assert_eq!(uses.len(), wire.coedges.len());
            for (id, coedge) in uses.iter().zip(&wire.coedges) {
                let oriented = &r[id];
                assert_eq!(oriented.0, "ORIENTED_EDGE");
                let edge_id = reference(&oriented.1[3]);
                assert_eq!(edge_id, *edges[coedge.edge].0);
                let forward = oriented.1[4] == ".T.";
                assert_eq!(forward, coedge.forward);
                let edge_record = &r[&edge_id];
                let surface_curve = &r[&reference(&edge_record.1[3])];
                let surface_id = reference(&args[2]);
                let candidates: Vec<_> = references(&surface_curve.1[2])
                    .into_iter()
                    .filter(|id| reference(&r[id].1[1]) == surface_id)
                    .collect();
                assert_eq!(candidates.len(), 1);
                let pc = &r[&candidates[0]];
                assert_eq!(pc.0, "PCURVE");
                let definition = &r[&reference(&pc.1[2])];
                let geometry = references(&definition.1[1]);
                assert_eq!(geometry.len(), 1);
                let line = &r[&geometry[0]];
                assert_eq!(line.0, "LINE");
                let origin = numbers(&r[&reference(&line.1[1])].1[1]);
                let vector = &r[&reference(&line.1[2])];
                let ratios = numbers(&r[&reference(&vector.1[1])].1[1]);
                let length: f64 = vector.1[2].parse().unwrap();
                let norm = ratios[0].hypot(ratios[1]);
                let domain = solid.edges[coedge.edge].curve.range();
                for j in 0..=8 {
                    let t = domain[0] + (domain[1] - domain[0]) * j as f64 / 8.;
                    let actual = coedge.pcurve.evaluate(t);
                    for axis in 0..2 {
                        assert!(
                            (origin[axis] + t * ratios[axis] / norm * length - actual[axis]).abs()
                                < 1e-12
                        );
                    }
                }
                edge_uses
                    .entry(edge_id)
                    .or_default()
                    .push(if expected.orientation == 1 {
                        forward
                    } else {
                        !forward
                    });
            }
        }
    }
    for use_ in edge_uses.values() {
        assert_eq!(use_.len(), 2);
        assert_ne!(use_[0], use_[1]);
    }
    for (_, args) in r.values() {
        for arg in args {
            for token in arg.split(|c: char| !c.is_ascii_digit() && c != '#') {
                if token.starts_with('#') {
                    assert!(r.contains_key(&reference(token)));
                }
            }
        }
    }
}
#[test]
fn rational_polygon_and_annular_actual_basis_topology_and_pcurves_roundtrip_text() {
    let tol = Tolerance::default();
    let tr = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([20., 12., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(tr, tol)
        .unwrap();
    let p = NurbsGraphPolygonSolid::new(
        &source,
        vec![
            [0.2, 0.2],
            [0.75, 0.15],
            [0.85, 0.65],
            [0.5, 0.8],
            [0.15, 0.6],
        ],
        tol,
    )
    .unwrap();
    let step = p.export_step_mm(tol).unwrap();
    verify(&step, p.brep());
    assert!(import_step_nurbs_graph_auto_mm(&step, tol).is_err());
    let h = p
        .through_uv_polygon(vec![[0.35, 0.35], [0.55, 0.35], [0.4, 0.55]], tol)
        .unwrap();
    let step = h.export_step_mm(tol).unwrap();
    verify(&step, h.brep());
    assert!(import_step_nurbs_graph_auto_mm(&step, tol).is_err());
    assert!(h
        .brep()
        .edges
        .iter()
        .any(|e| matches!(&e.curve,Curve::Nurbs(c) if c.weights().iter().any(|w|*w!=1.))));
}
#[test]
fn three_and_sixteen_corners_and_public_corruption_export_checks() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], 20., tol).unwrap();
    for corners in [
        vec![[0., 0.], [1., 0.], [0., 1.]],
        (0..16)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 16.;
                [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
            })
            .collect(),
    ] {
        let p = NurbsGraphPolygonSolid::new(&source, corners, tol).unwrap();
        verify(&p.export_step_mm(tol).unwrap(), p.brep());
        let mut bad = p.clone();
        bad.solid.vertices[0].point.x += 1e-12;
        assert!(bad.export_step_mm(tol).is_err());
        let mut bad = p;
        bad.solid.shell.faces[0].wires[0].coedges[0].edge = 999;
        assert!(bad.export_step_mm(tol).is_err());
    }
}
