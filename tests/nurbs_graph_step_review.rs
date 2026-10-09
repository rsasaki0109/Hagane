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
fn records(step: &str) -> BTreeMap<usize, (String, Vec<String>)> {
    step.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('#') {
                return None;
            }
            let (id, body) = line[1..].split_once('=').unwrap();
            let body = body.trim_end_matches(';');
            let (kind, args) = body.split_once('(').unwrap();
            Some((
                id.parse().unwrap(),
                (kind.to_owned(), fields(&format!("({args}"))),
            ))
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
        let domain = expected.curve.range();
        for j in 0..=16 {
            let t = domain[0] + (domain[1] - domain[0]) * j as f64 / 16.;
            let value = p
                .iter()
                .enumerate()
                .fold(Vec3::new(0., 0., 0.), |sum, (i, point)| {
                    sum + *point * basis(i, d, t, &k, p.len())
                });
            assert!((value - expected.curve.try_evaluate(t).unwrap()).norm() < 1e-10);
        }
    }
    for ((_, (_, args)), expected) in faces.iter().zip(&solid.shell.faces) {
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
        for a in 0..=8 {
            for b in 0..=8 {
                let domain = s.domain();
                let uv = [
                    domain[0][0] + (domain[0][1] - domain[0][0]) * a as f64 / 8.,
                    domain[1][0] + (domain[1][1] - domain[1][0]) * b as f64 / 8.,
                ];
                let mut value = Vec3::new(0., 0., 0.);
                for i in 0..count[0] {
                    for j in 0..count[1] {
                        value = value
                            + p[i * count[1] + j]
                                * (basis(i, degree[0], uv[0], &k[0], count[0])
                                    * basis(j, degree[1], uv[1], &k[1], count[1]));
                    }
                }
                assert!((value - s.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-10);
            }
        }
        let bounds = references(&args[1]);
        assert_eq!(bounds.len(), expected.wires.len());
        for (bound, wire) in bounds.iter().zip(&expected.wires) {
            let record = &r[bound];
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
fn serialized_basis_and_closed_refs_preserve_placed_trimmed_and_holed_graphs() {
    let tol = Tolerance::default();
    let transform = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let graph = NurbsGraphSolid::new([20., 12., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], tol)
        .unwrap()
        .transformed(transform, tol)
        .unwrap();
    verify(&graph.export_step_mm(tol).unwrap(), &graph.solid);
    let holed = NurbsGraphHoledSolid::new(&graph, [[0.35, 0.5], [0.25, 0.4]], tol).unwrap();
    verify(&holed.export_step_mm(tol).unwrap(), &holed.solid);
}
#[test]
fn step_export_rejects_public_corruption_without_writing_success() {
    let tol = Tolerance::default();
    let mut graph = NurbsGraphSolid::new([20., 12., 3.], 20., tol).unwrap();
    graph.solid.vertices[0].point.x += 0.01;
    assert!(graph.export_step_mm(tol).is_err());
    let source = NurbsGraphSolid::new([20., 12., 3.], 20., tol).unwrap();
    let mut holed = NurbsGraphHoledSolid::new(&source, [[0.35, 0.5], [0.25, 0.4]], tol).unwrap();
    holed.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(holed.export_step_mm(tol).is_err());
}
