use hagane::*;
use std::collections::BTreeMap;
fn hash(s: &str) -> u64 {
    s.bytes().fold(14695981039346656037, |a, b| {
        (a ^ b as u64).wrapping_mul(1099511628211)
    })
}
fn records(s: &str) -> BTreeMap<usize, String> {
    s.lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| {
            let (id, body) = l.split_once('=').unwrap();
            (id[1..].parse().unwrap(), body.to_string())
        })
        .collect()
}
fn refs(s: &str) -> Vec<usize> {
    s.split('#')
        .skip(1)
        .map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap()
        })
        .collect()
}
#[test]
fn actual_rational_uv_pcurves_are_two_dimensional_with_original_weights() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    let body = source.through_xy_circle([40., 30.], 12., t).unwrap();
    let text = body.export_step_mm(t).unwrap();
    let db = records(&text);
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
    assert_eq!(text.matches("=PCURVE(").count(), 48);
    assert_eq!(text.matches("=ADVANCED_FACE(").count(), 10);
    assert_eq!(text.matches("=EDGE_CURVE(").count(), 24);
    assert_eq!(text.matches("=FACE_BOUND(").count(), 2);
    assert_eq!(text.matches("B_SPLINE_CURVE(8,").count(), 8);
    assert_eq!(text.matches("B_SPLINE_SURFACE(8,1,").count(), 4);
    let mut uv_count = 0;
    for record in db.values().filter(|r| r.starts_with("PCURVE(")) {
        let ids = refs(record);
        let representation = &db[&ids[1]];
        let rids = refs(representation);
        assert!(db[&rids[1]].contains("GEOMETRIC_REPRESENTATION_CONTEXT(2)"));
        let curve = &db[&rids[0]];
        if curve.starts_with("(BOUNDED_CURVE()") {
            uv_count += 1;
            assert!(curve.contains("B_SPLINE_CURVE(2,"));
            assert!(curve.contains("RATIONAL_B_SPLINE_CURVE((1.,0.7071067811865476,1.))"));
            assert!(curve.contains("B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.UNSPECIFIED.)"));
            for id in refs(curve) {
                let p = &db[&id];
                assert!(p.starts_with("CARTESIAN_POINT('',("));
                assert_eq!(p.matches(',').count(), 2);
            }
        } else {
            assert!(curve.starts_with("LINE("));
        }
    }
    assert_eq!(uv_count, 8);
    assert!(import_step_mm(&text, t).is_err());
    assert!(import_step_nurbs_graph_mm(&text, t).is_err());
    assert!(import_step_nurbs_graph_holed_mm(&text, t).is_err());
    assert!(import_step_nurbs_graph_polygon_mm(&text, t).is_err());
}
#[test]
fn old_affine_and_near_unit_writer_bytes_are_unchanged() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    assert_eq!(hash(&s.export_step_mm(t).unwrap()), 11717637070999924214);
    let r = NurbsGraphHoledSolid::new(&s, [[0.35, 0.65], [0.35, 0.65]], t).unwrap();
    assert_eq!(hash(&r.export_step_mm(t).unwrap()), 12005359103225553077);
    let p = NurbsGraphPolygonSolid::new(&s, vec![[0.1, 0.2], [0.9, 0.3], [0.3, 0.9]], t).unwrap();
    assert_eq!(hash(&p.export_step_mm(t).unwrap()), 1158138697538388731);
}
#[test]
fn placed_trimmed_signed_exports_and_strict_corruption_failure() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], -12., t)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], t)
        .unwrap()
        .transformed(Transform::translation(Vec3::new(3., -2., 5.)).unwrap(), t)
        .unwrap();
    let body = source.through_xy_circle([40., 30.], 12., t).unwrap();
    let text = body.export_step_mm(t).unwrap();
    assert_eq!(text.matches("=PCURVE(").count(), 48);
    let mut bad = body.clone();
    bad.solid.vertices[8].point.x = bad.solid.vertices[8].point.x.next_up();
    assert!(bad.export_step_mm(t).is_err());
    let mut bad = body;
    bad.solid.shell.faces[1].wires[1].coedges[0].pcurve = PCurve::Affine {
        origin: [0., 0.],
        direction: [1., 0.],
    };
    assert!(bad.export_step_mm(t).is_err());
}
