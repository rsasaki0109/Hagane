use hagane::*;
use std::collections::BTreeMap;
fn records(step: &str) -> BTreeMap<usize, &str> {
    step.lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            let (id, body) = line.split_once('=').unwrap();
            (id[1..].parse().unwrap(), body)
        })
        .collect()
}
fn references(body: &str) -> Vec<usize> {
    body.split('#')
        .skip(1)
        .map(|tail| {
            tail.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap()
        })
        .collect()
}
fn check(step: &str, solid: &Solid) {
    assert!(step.starts_with("ISO-10303-21;"));
    assert!(step.ends_with("END-ISO-10303-21;\n"));
    assert!(step.contains("SI_UNIT(.MILLI.,.METRE.)"));
    let records = records(step);
    for body in records.values() {
        for id in references(body) {
            assert!(records.contains_key(&id));
        }
    }
    for (name, count) in [
        ("VERTEX_POINT", solid.vertices.len()),
        ("EDGE_CURVE", solid.edges.len()),
        ("B_SPLINE_CURVE_WITH_KNOTS", solid.edges.len()),
        ("SURFACE_CURVE", solid.edges.len()),
        ("PCURVE", 2 * solid.edges.len()),
        ("B_SPLINE_SURFACE_WITH_KNOTS", solid.shell.faces.len()),
        ("ADVANCED_FACE", solid.shell.faces.len()),
    ] {
        assert_eq!(
            records
                .values()
                .filter(|body| body.starts_with(&format!("{name}(")))
                .count(),
            count
        );
    }
    let mut uses = BTreeMap::<usize, (usize, i32)>::new();
    for face in records.values().filter(|b| b.starts_with("ADVANCED_FACE(")) {
        let sign = if face.ends_with(",.T.);") { 1 } else { -1 };
        let ids = references(face);
        for bound in &ids[..ids.len() - 1] {
            let edge_loop = references(records[bound])[0];
            for oriented in references(records[&edge_loop]) {
                let body = records[&oriented];
                let entry = uses.entry(references(body)[0]).or_default();
                entry.0 += 1;
                entry.1 += sign * if body.ends_with(",.T.);") { 1 } else { -1 };
            }
        }
    }
    assert_eq!(uses.len(), solid.edges.len());
    assert!(uses.values().all(|x| *x == (2, 0)));
    for surface_curve in records.values().filter(|b| b.starts_with("SURFACE_CURVE(")) {
        let refs = references(surface_curve);
        assert_eq!(refs.len(), 3);
        assert!(records[&refs[0]].starts_with("B_SPLINE_CURVE_WITH_KNOTS("));
        for pcurve in &refs[1..] {
            assert!(records[pcurve].starts_with("PCURVE("));
        }
    }
}
#[test]
fn plain_export_retains_shared_polynomial_brep_and_orientation() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap(),
            tol,
        )
        .unwrap();
    let step = source.export_step_mm(tol).unwrap();
    check(&step, source.brep());
    assert_eq!(step.matches("=FACE_BOUND(").count(), 0);
    assert_eq!(step.matches("=FACE_OUTER_BOUND(").count(), 6);
    assert!(export_step_mm(source.brep(), tol).is_err());
    assert!(import_step_mm(&step, tol).is_err());
}
#[test]
fn holed_export_preserves_inner_wires_and_original_nonunit_parameter_domains() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)
        .unwrap();
    let solid = NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.4, 0.6]], tol).unwrap();
    let step = solid.export_step_mm(tol).unwrap();
    check(&step, solid.brep());
    assert_eq!(step.matches("=FACE_BOUND(").count(), 2);
    assert_eq!(step.matches("=FACE_OUTER_BOUND(").count(), 10);
    assert!(step.contains("(3,2,2,3),(3,2,2,3),(0.1,0.3,0.7,0.9),(0.2,0.4,0.6,0.8)"));
    assert_eq!(step, solid.export_step_mm(tol).unwrap());
}
#[test]
fn step_export_rejects_public_geometry_and_topology_edits_before_serializing() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let mut changed = source.clone();
    changed.solid.vertices[0].point.x += tol.linear / 4.;
    assert!(changed.export_step_mm(tol).is_err());
    let mut changed = NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.4, 0.6]], tol).unwrap();
    changed.solid.shell.faces[0].wires[1].coedges[0].forward ^= true;
    assert!(changed.export_step_mm(tol).is_err());
}
