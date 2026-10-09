use hagane::*;
use std::collections::BTreeMap;
fn tol() -> Tolerance {
    Tolerance::default()
}
fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], 20., tol()).unwrap()
}
fn closed(s: &NurbsGraphPolygonSolid) {
    s.validate(tol()).unwrap();
    let b = s.brep();
    let mut uses = vec![(0, 0); b.edges.len()];
    for face in &b.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
                let range = b.edges[c.edge].curve.range();
                for i in 0..=20 {
                    let t = range[0] + (range[1] - range[0]) * i as f64 / 20.;
                    let uv = c.pcurve.evaluate(t);
                    assert!(
                        (b.edges[c.edge].curve.try_evaluate(t).unwrap()
                            - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                            < 1e-10
                    );
                }
            }
        }
    }
    assert!(uses.iter().all(|x| *x == (2, 0)));
    let m = s.tessellate_bounded(0.5, 65536, tol()).unwrap();
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for (i, &node) in m.vertex_nodes.iter().enumerate() {
        if let Some(old) = nodes.insert(node, m.mesh.positions[i]) {
            assert_eq!(old, m.mesh.positions[i]);
        }
    }
    for t in &m.mesh.triangles {
        for i in 0..3 {
            let a = m.vertex_nodes[t[i]];
            let b = m.vertex_nodes[t[(i + 1) % 3]];
            let e = edges.entry((a.min(b), a.max(b))).or_insert((0, 0));
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|x| *x == (2, 0)));
}
fn cut(s: &NurbsGraphPolygonSplit) {
    s.validate(tol()).unwrap();
    closed(&s.negative);
    closed(&s.positive);
    let common = s
        .negative
        .polygon()
        .iter()
        .filter(|p| s.positive.polygon().contains(p))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(common.len(), 2);
    let wall = |body: &NurbsGraphPolygonSolid| {
        let n = body.polygon().len();
        let i = (0..n)
            .find(|&i| {
                common.contains(&body.polygon()[i]) && common.contains(&body.polygon()[(i + 1) % n])
            })
            .unwrap();
        let Surface::Nurbs(net) = &body.brep().shell.faces[2 + i].surface else {
            panic!()
        };
        net.clone()
    };
    let a = wall(&s.negative);
    let b = wall(&s.positive);
    assert_eq!(a.degrees(), b.degrees());
    let [nu, nv] = a.control_counts();
    assert_eq!(a.control_counts(), b.control_counts());
    for i in 0..nu {
        for j in 0..nv {
            assert_eq!(
                a.control_points()[i * nv + j],
                b.control_points()[(nu - 1 - i) * nv + j]
            );
            assert_eq!(a.weights()[i * nv + j], b.weights()[(nu - 1 - i) * nv + j]);
        }
    }
    let Surface::Plane { origin, u, v } = &s.plane else {
        panic!()
    };
    let n = u.cross(*v).normalized().unwrap();
    for i in 0..=20 {
        for j in 0..=5 {
            let x = i as f64 / 20.;
            let y = j as f64 / 5.;
            let p = a.evaluate(x, y).unwrap();
            assert!((p - *origin).dot(n).abs() < 1e-10);
            assert!((p - b.evaluate(1. - x, y).unwrap()).norm() < 1e-10);
            assert!((a.normal(x, y).unwrap() + b.normal(1. - x, y).unwrap()).norm() < 1e-10);
        }
    }
}
#[test]
fn independent_oblique_simplex_moments_and_complement_partition() {
    let source = source();
    let split = source.split_uv_line([0., 0.8], [0.8, 0.], tol()).unwrap();
    cut(&split);
    let fact = |n: usize| (1..=n).map(|i| i as f64).product::<f64>();
    let integral =
        |i: usize, j: usize| 0.8f64.powi((i + j + 2) as i32) * fact(i) * fact(j) / fact(i + j + 2);
    let coefficients = [
        (0, 0, 3.),
        (1, 1, 80.),
        (2, 1, -80.),
        (1, 2, -80.),
        (2, 2, 80.),
    ];
    let moment = |di, dj| {
        coefficients
            .iter()
            .map(|&(i, j, c)| c * integral(i + di, j + dj))
            .sum::<f64>()
    };
    let square = coefficients
        .iter()
        .flat_map(|&(i, j, c)| {
            coefficients
                .iter()
                .map(move |&(k, l, d)| c * d * integral(i + k, j + l))
        })
        .sum::<f64>();
    let volume = 240. * moment(0, 0);
    let centroid = Point3::new(
        20. * moment(1, 0) / moment(0, 0),
        12. * moment(0, 1) / moment(0, 0),
        square / (2. * moment(0, 0)),
    );
    let triangle = if split.negative.polygon().contains(&[0., 0.]) {
        &split.negative
    } else {
        &split.positive
    };
    let actual = triangle.mass_properties(tol()).unwrap();
    assert!((actual.volume - volume).abs() < 1e-10);
    assert!((actual.centroid - centroid).norm() < 1e-12);
    let whole = source.mass_properties(tol()).unwrap();
    let a = split.negative.mass_properties(tol()).unwrap();
    let b = split.positive.mass_properties(tol()).unwrap();
    assert!((a.volume + b.volume - whole.volume).abs() < 1e-10);
    let p = Point3::new(0., 0., 0.)
        + (a.centroid - Point3::new(0., 0., 0.)) * (a.volume / whole.volume)
        + (b.centroid - Point3::new(0., 0., 0.)) * (b.volume / whole.volume);
    assert!((p - whole.centroid).norm() < 1e-12);
}
#[test]
fn placed_trimmed_signed_polygon_cut_and_reversed_plane_partition() {
    let tr = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let s = NurbsGraphSolid::new([20., 12., 3.], -2., tol())
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol())
        .unwrap()
        .transformed(tr, tol())
        .unwrap();
    let p = NurbsGraphPolygonSolid::new(
        &s,
        vec![
            [0.2, 0.2],
            [0.75, 0.15],
            [0.85, 0.65],
            [0.5, 0.8],
            [0.15, 0.6],
        ],
        tol(),
    )
    .unwrap();
    let split = p.split_uv_line([0., 0.3], [1., 0.6], tol()).unwrap();
    cut(&split);
    let reverse = p.split_uv_line([1., 0.6], [0., 0.3], tol()).unwrap();
    cut(&reverse);
    assert!((split.negative.volume().unwrap() - reverse.positive.volume().unwrap()).abs() < 1e-10);
    let whole = p.mass_properties(tol()).unwrap();
    let a = split.negative.mass_properties(tol()).unwrap();
    let b = split.positive.mass_properties(tol()).unwrap();
    assert!((a.volume + b.volume - whole.volume).abs() < 1e-10);
    assert!(
        ((a.centroid - whole.centroid) * a.volume + (b.centroid - whole.centroid) * b.volume)
            .norm()
            < 1e-9
    );
}
#[test]
fn contact_nonfinite_outside_collapsed_resources_and_public_corruption_reject() {
    let s = source();
    for (a, b) in [
        ([0., 0.], [1., 1.]),
        ([0., 1e-12], [1., 1. + 1e-12]),
        ([0., 2.], [1., 2.]),
        ([0.5, 0.5], [0.5, 0.5]),
        ([f64::NAN, 0.], [1., 0.]),
        ([0., 0.], [1e-14, 1e-14]),
    ] {
        assert!(s.split_uv_line(a, b, tol()).is_err());
    }
    let split = s.split_uv_line([0., 0.8], [0.8, 0.], tol()).unwrap();
    let mut bad = split.clone();
    bad.negative.solid.vertices[0].point.x += 1e-12;
    assert!(bad.validate(tol()).is_err());
    let mut bad = split.clone();
    bad.section.face.orientation = -1;
    assert!(bad.validate(tol()).is_err());
    let mut bad = split;
    bad.plane = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    assert!(bad.validate(tol()).is_err());
    let mut s = s;
    s.solid.edges[0].vertices = [999, 999];
    assert!(s.split_uv_line([0., 0.8], [0.8, 0.], tol()).is_err());
}
