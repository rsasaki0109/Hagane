use hagane::*;
use std::collections::HashMap;
fn base(b: f64) -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], b, Tolerance::default()).unwrap()
}
fn ranges() -> [[f64; 2]; 2] {
    [[0.2, 0.8], [0.1, 0.7]]
}
fn volume(r: [[f64; 2]; 2], b: f64) -> f64 {
    let f = |x: f64| x * x / 2. - x * x * x / 3.;
    240. * (3. * (r[0][1] - r[0][0]) * (r[1][1] - r[1][0])
        + 4. * b * (f(r[0][1]) - f(r[0][0])) * (f(r[1][1]) - f(r[1][0])))
}
#[test]
fn exact_trim_volume_source_coordinates_and_all_curved_edge_pcurves() {
    for bulge in [20., -2.] {
        let s = base(bulge)
            .trimmed_uv(ranges(), Tolerance::default())
            .unwrap();
        s.validate(Tolerance::default()).unwrap();
        assert_eq!(s.source_domain(), ranges());
        assert!((s.volume().unwrap() - volume(ranges(), bulge)).abs() < 1e-10);
        assert_eq!(s.solid.vertices.len(), 8);
        assert_eq!(s.solid.edges.len(), 12);
        assert_eq!(s.solid.shell.faces.len(), 6);
        let Surface::Nurbs(top) = &s.solid.shell.faces[1].surface else {
            panic!("exact top required")
        };
        assert_eq!(top.domain(), ranges());
        for i in 0..=10 {
            for j in 0..=10 {
                let u = 0.2 + 0.6 * i as f64 / 10.;
                let v = 0.1 + 0.6 * j as f64 / 10.;
                assert!(
                    (top.evaluate(u, v).unwrap()
                        - Point3::new(
                            20. * u,
                            12. * v,
                            3. + 4. * bulge * u * (1. - u) * v * (1. - v)
                        ))
                    .norm()
                        < 1e-10
                );
            }
        }
        let mut uses: HashMap<usize, Vec<bool>> = HashMap::new();
        for face in &s.solid.shell.faces {
            for c in &face.wires[0].coedges {
                uses.entry(c.edge)
                    .or_default()
                    .push(if face.orientation == 1 {
                        c.forward
                    } else {
                        !c.forward
                    });
                let edge = &s.solid.edges[c.edge];
                let domain = edge.curve.range();
                for i in 0..=20 {
                    let t = domain[0] + (domain[1] - domain[0]) * i as f64 / 20.;
                    let uv = c.pcurve.evaluate(t);
                    assert!(
                        (edge.curve.try_evaluate(t).unwrap()
                            - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                            < 1e-10
                    );
                }
            }
        }
        assert_eq!(uses.len(), 12);
        for use_ in uses.values() {
            assert_eq!(use_.len(), 2);
            assert_ne!(use_[0], use_[1]);
        }
        let curved = s
            .solid
            .edges
            .iter()
            .filter(|edge| {
                let [a, b] = edge.curve.range();
                let midpoint = edge.curve.try_evaluate((a + b) / 2.).unwrap();
                let chord = (edge.curve.try_evaluate(a).unwrap()
                    + edge.curve.try_evaluate(b).unwrap())
                    * 0.5;
                (midpoint - chord).norm() > 1e-6
            })
            .count();
        assert_eq!(curved, 4);
    }
}
#[test]
fn trimmed_nonplanar_walls_have_bounded_closed_display_even_when_placed() {
    let local = base(20.)
        .trimmed_uv(ranges(), Tolerance::default())
        .unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap();
    let placement = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let moved = local.transformed(placement, Tolerance::default()).unwrap();
    assert!((moved.volume().unwrap() - volume(ranges(), 20.)).abs() < 1e-10);
    let bounds = moved.bounds().unwrap();
    let display = moved
        .tessellate_bounded(0.1, 65536, Tolerance::default())
        .unwrap();
    let mut nodes = HashMap::new();
    for (i, &node) in display.vertex_nodes.iter().enumerate() {
        let p = display.mesh.positions[i];
        if let Some(previous) = nodes.insert(node, p) {
            assert_eq!(previous, p);
        }
        assert!(
            p.x >= bounds.min.x - 1e-10
                && p.x <= bounds.max.x + 1e-10
                && p.y >= bounds.min.y - 1e-10
                && p.y <= bounds.max.y + 1e-10
                && p.z >= bounds.min.z - 1e-10
                && p.z <= bounds.max.z + 1e-10
        );
    }
    let mut edges: HashMap<(usize, usize), (usize, i32)> = HashMap::new();
    let mut curved_wall_bounds = 0;
    for (t, ids) in display.mesh.triangles.iter().enumerate() {
        let f = display.vertex_faces[ids[0]];
        assert!(ids.iter().all(|i| display.vertex_faces[*i] == f));
        let face = &moved.solid.shell.faces[f];
        let uv = ids.map(|i| display.vertex_uv[i]);
        let p = ids.map(|i| display.mesh.positions[i]);
        let bound = display.error_bounds[t];
        assert!((0. ..=0.1).contains(&bound));
        if f >= 2 && bound > 1e-8 {
            curved_wall_bounds += 1;
        }
        for i in 0..=5 {
            for j in 0..=5 - i {
                let w = [i as f64 / 5., j as f64 / 5., 1. - (i + j) as f64 / 5.];
                let Surface::Nurbs(surface) = &face.surface else {
                    panic!("rational face required")
                };
                let domain = surface.domain();
                let at = [0, 1].map(|axis| {
                    let raw = (0..3).map(|k| w[k] * uv[k][axis]).sum::<f64>();
                    let clamped = raw.clamp(domain[axis][0], domain[axis][1]);
                    assert!((raw - clamped).abs() < 1e-14);
                    clamped
                });
                let exact = face.surface.try_evaluate(at[0], at[1]).unwrap();
                let chord = Point3::new(
                    (0..3).map(|k| w[k] * p[k].x).sum(),
                    (0..3).map(|k| w[k] * p[k].y).sum(),
                    (0..3).map(|k| w[k] * p[k].z).sum(),
                );
                assert!((exact - chord).norm() <= bound + 1e-10);
            }
        }
        for i in 0..3 {
            let a = display.vertex_nodes[ids[i]];
            let b = display.vertex_nodes[ids[(i + 1) % 3]];
            let e = edges.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(curved_wall_bounds > 0);
    for e in edges.values() {
        assert_eq!(*e, (2, 0));
    }
    assert!(
        (display.mesh.signed_volume() - moved.volume().unwrap()).abs() <= 240. * 0.6 * 0.6 * 0.1
    );
}
#[test]
fn nested_restrictions_match_direct_geometry_and_reject_invalid_domains() {
    let s = base(20.);
    let first = s.trimmed_uv(ranges(), Tolerance::default()).unwrap();
    let inner = [[0.3, 0.6], [0.2, 0.5]];
    let nested = first.trimmed_uv(inner, Tolerance::default()).unwrap();
    let direct = s.trimmed_uv(inner, Tolerance::default()).unwrap();
    assert_eq!(nested.source_domain(), inner);
    assert!((nested.volume().unwrap() - direct.volume().unwrap()).abs() < 1e-10);
    for (a, b) in nested.solid.vertices.iter().zip(&direct.solid.vertices) {
        assert!((a.point - b.point).norm() < 1e-10);
    }
    for invalid in [
        [[0.1, 0.6], [0.2, 0.5]],
        [[0.3, 0.9], [0.2, 0.5]],
        [[0.4, 0.4], [0.2, 0.5]],
        [[0.6, 0.3], [0.2, 0.5]],
        [[0.3, 0.6], [f64::NAN, 0.5]],
        [[0.3, 0.3 + 1e-15], [0.2, 0.5]],
    ] {
        assert!(first.trimmed_uv(invalid, Tolerance::default()).is_err());
    }
    assert!(nested
        .tessellate_bounded(1e-30, 65536, Tolerance::default())
        .is_err());
    assert!(nested
        .tessellate_bounded(0.1, 1, Tolerance::default())
        .is_err());
}
