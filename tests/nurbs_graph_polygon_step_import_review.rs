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
fn permute(step: &str) -> String {
    let max = step
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            line[1..]
                .split('=')
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap()
        })
        .max()
        .unwrap();
    let mut mapped = String::new();
    let chars: Vec<_> = step.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let id: usize = chars[start..i].iter().collect::<String>().parse().unwrap();
            mapped.push_str(&format!("#{}", max + 17 - id));
        } else {
            mapped.push(chars[i]);
            i += 1;
        }
    }
    let mut entities: Vec<_> = mapped
        .lines()
        .filter(|line| line.starts_with('#'))
        .collect();
    entities.reverse();
    format!(
        "{}DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;",
        mapped.split("DATA;\n").next().unwrap(),
        entities.join("\n")
    )
}

fn ring() -> Vec<[f64; 2]> {
    (0..16)
        .map(|i| {
            let a = std::f64::consts::TAU * (i as f64 + 0.01) / 16.;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect()
}
fn body(mut polygon: Vec<[f64; 2]>, b: f64) -> NurbsGraphPolygonSolid {
    let first = (0..polygon.len())
        .min_by(|&a, &b| {
            polygon[a][0]
                .total_cmp(&polygon[b][0])
                .then(polygon[a][1].total_cmp(&polygon[b][1]))
        })
        .unwrap();
    polygon.rotate_left(first);
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], b, t).unwrap();
    NurbsGraphPolygonSolid::new(&source, polygon, t).unwrap()
}
fn changed(step: &str, id: usize, kind: &str, args: &[String]) -> String {
    let prefix = format!("#{id}=");
    assert_eq!(step.lines().filter(|l| l.starts_with(&prefix)).count(), 1);
    step.lines()
        .map(|l| {
            if l.starts_with(&prefix) {
                format!("{prefix}{kind}({});", args.join(","))
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn triangle_and_trig_sixteen_actual_rational_brep_imports_preserve_every_entity_geometry() {
    let t = Tolerance::default();
    for polygon in [vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]], ring()] {
        for b in [-2., 0., 20., 0.001] {
            let original = body(polygon.clone(), b);
            let text = original.export_step_mm(t).unwrap();
            let imported = import_step_nurbs_graph_polygon_mm(&text, t).unwrap();
            assert_eq!(imported.export_step_mm(t).unwrap(), text);
            assert_eq!(imported.polygon(), original.polygon());
            verify(&text, imported.brep());
            assert!(import_step_nurbs_graph_mm(&text, t).is_err());
            assert!(import_step_nurbs_graph_holed_mm(&text, t).is_err());
            assert!(import_step_nurbs_graph_auto_mm(&text, t).is_err());
            assert!(import_step_mm(&text, t).is_err());
        }
    }
    let original = body(ring(), -2.);
    assert!(original
        .brep()
        .edges
        .iter()
        .any(|e| matches!(&e.curve,Curve::Nurbs(c) if c.weights().iter().any(|w|*w!=1.))));
}
#[test]
fn arbitrary_ids_entity_order_and_loop_start_preserve_actual_geometry() {
    let t = Tolerance::default();
    let original = body(ring(), 20.);
    let step = original.export_step_mm(t).unwrap();
    let rotated = step
        .lines()
        .map(|l| {
            if let Some((head, tail)) = l.split_once("EDGE_LOOP('',(") {
                let mut ids: Vec<_> = tail.strip_suffix("));").unwrap().split(',').collect();
                ids.rotate_left(1);
                format!("{head}EDGE_LOOP('',({}));", ids.join(","))
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let imported = import_step_nurbs_graph_polygon_mm(&permute(&rotated), t).unwrap();
    assert!((imported.volume().unwrap() - original.volume().unwrap()).abs() < 1e-10);
    let mut a: Vec<_> = original
        .brep()
        .vertices
        .iter()
        .map(|v| {
            [
                v.point.x.to_bits(),
                v.point.y.to_bits(),
                v.point.z.to_bits(),
            ]
        })
        .collect();
    let mut b: Vec<_> = imported
        .brep()
        .vertices
        .iter()
        .map(|v| {
            [
                v.point.x.to_bits(),
                v.point.y.to_bits(),
                v.point.z.to_bits(),
            ]
        })
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    verify(&imported.export_step_mm(t).unwrap(), imported.brep());
}
#[test]
fn one_ulp_raw_pcurve_ratio_length_origin_and_orientation_changes_are_rejected() {
    let t = Tolerance::default();
    let text = body(ring(), 20.).export_step_mm(t).unwrap();
    let r = records(&text);
    let (&dir, (_, args)) = r
        .iter()
        .find(|(_, (kind, args))| {
            kind == "DIRECTION" && {
                let p = numbers(&args[1]);
                p.len() == 2 && p.iter().all(|v| *v != 0.)
            }
        })
        .unwrap();
    let mut a = args.clone();
    let mut p = numbers(&a[1]);
    p[0] = f64::from_bits(p[0].to_bits() + 1);
    a[1] = format!("({},{})", p[0], p[1]);
    assert!(import_step_nurbs_graph_polygon_mm(&changed(&text, dir, "DIRECTION", &a), t).is_err());
    let (&vector, (_, args)) = r
        .iter()
        .find(|(_, (kind, args))| kind == "VECTOR" && reference(&args[1]) == dir)
        .unwrap();
    let mut a = args.clone();
    let v: f64 = a[2].parse().unwrap();
    a[2] = f64::from_bits(v.to_bits() + 1).to_string();
    assert!(import_step_nurbs_graph_polygon_mm(&changed(&text, vector, "VECTOR", &a), t).is_err());
    let (_, (_, line)) = r
        .iter()
        .find(|(_, (kind, args))| kind == "LINE" && reference(&args[2]) == vector)
        .unwrap();
    let id = reference(&line[1]);
    let mut a = r[&id].1.clone();
    let mut p = numbers(&a[1]);
    p[0] = f64::from_bits(p[0].to_bits() + 1);
    a[1] = format!("({},{})", p[0], p[1]);
    assert!(
        import_step_nurbs_graph_polygon_mm(&changed(&text, id, "CARTESIAN_POINT", &a), t).is_err()
    );
    let (&id, (_, args)) = r
        .iter()
        .find(|(_, (kind, _))| kind == "ORIENTED_EDGE")
        .unwrap();
    let mut a = args.clone();
    a[4] = if a[4] == ".T." {
        ".F.".into()
    } else {
        ".T.".into()
    };
    assert!(
        import_step_nurbs_graph_polygon_mm(&changed(&text, id, "ORIENTED_EDGE", &a), t).is_err()
    );
}
#[test]
fn cap_and_noncap_physical_controls_mutations_and_unsupported_scope_reject() {
    let t = Tolerance::default();
    let text = body(ring(), 20.).export_step_mm(t).unwrap();
    let r = records(&text);
    for (_, (kind, args)) in r
        .iter()
        .filter(|(_, (kind, _))| kind == "B_SPLINE_SURFACE_WITH_KNOTS")
    {
        assert_eq!(kind, "B_SPLINE_SURFACE_WITH_KNOTS");
        let ids: Vec<_> = fields(&args[3])
            .iter()
            .flat_map(|row| references(row))
            .collect();
        let id = ids[ids.len() / 2];
        let mut a = r[&id].1.clone();
        let mut p = numbers(&a[1]);
        p[2] += t.linear / 100.;
        a[1] = format!("({},{},{})", p[0], p[1], p[2]);
        assert!(
            import_step_nurbs_graph_polygon_mm(&changed(&text, id, "CARTESIAN_POINT", &a), t)
                .is_err()
        );
    }
    let source = NurbsGraphSolid::new([20., 12., 3.], 20., t).unwrap();
    for source in [
        source
            .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], t)
            .unwrap(),
        source
            .transformed(Transform::translation(Vec3::new(1., 2., 3.)).unwrap(), t)
            .unwrap(),
    ] {
        let polygon =
            NurbsGraphPolygonSolid::new(&source, vec![[0.25, 0.25], [0.75, 0.25], [0.5, 0.75]], t)
                .unwrap();
        assert!(
            import_step_nurbs_graph_polygon_mm(&polygon.export_step_mm(t).unwrap(), t).is_err()
        );
    }
    let stock = body(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], 20.);
    let holed = stock
        .through_uv_polygon(vec![[0.25, 0.25], [0.75, 0.25], [0.5, 0.75]], t)
        .unwrap();
    assert!(import_step_nurbs_graph_polygon_mm(&holed.export_step_mm(t).unwrap(), t).is_err());
}

#[test]
fn raw_near_unit_rational_weights_knots_and_surface_associations_are_not_normalized_away() {
    let t = Tolerance::default();
    let text = body(ring(), 20.).export_step_mm(t).unwrap();
    let line = text
        .lines()
        .find(|l| l.contains("RATIONAL_B_SPLINE_CURVE("))
        .unwrap();
    let (id, raw) = line[1..].split_once('=').unwrap();
    let id: usize = id.parse().unwrap();
    let original = components(raw.trim_end_matches(';'));
    for weight in [true, false] {
        let mut parts = original.clone();
        if weight {
            let a = parts.get_mut("RATIONAL_B_SPLINE_CURVE").unwrap();
            let mut values = numbers(&a[0]);
            values[1] = f64::from_bits(values[1].to_bits() + 1);
            a[0] = format!(
                "({})",
                values
                    .iter()
                    .map(f64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
        } else {
            let a = parts.get_mut("B_SPLINE_CURVE_WITH_KNOTS").unwrap();
            let mut values = numbers(&a[1]);
            let last = values.len() - 1;
            values[last] = f64::from_bits(values[last].to_bits() + 1);
            a[1] = format!(
                "({})",
                values
                    .iter()
                    .map(f64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        let replacement = format!(
            "#{id}=({});",
            parts
                .iter()
                .map(|(kind, args)| format!("{kind}({})", args.join(",")))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let prefix = format!("#{id}=");
        let changed = text
            .lines()
            .map(|l| {
                if l.starts_with(&prefix) {
                    replacement.clone()
                } else {
                    l.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(import_step_nurbs_graph_polygon_mm(&changed, t).is_err());
    }
    let r = records(&text);
    let (&id, (_, args)) = r.iter().find(|(_, (kind, _))| kind == "PCURVE").unwrap();
    let current = reference(&args[1]);
    let (&other, _) = r
        .iter()
        .find(|(id, (kind, _))| kind == "B_SPLINE_SURFACE_WITH_KNOTS" && **id != current)
        .unwrap();
    let mut args = args.clone();
    args[1] = format!("#{other}");
    assert!(import_step_nurbs_graph_polygon_mm(&changed(&text, id, "PCURVE", &args), t).is_err());
}
