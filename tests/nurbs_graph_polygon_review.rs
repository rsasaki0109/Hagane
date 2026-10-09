use hagane::*;
use std::collections::BTreeMap;
fn fact(n: usize) -> f64 {
    (1..=n).map(|n| n as f64).product()
}
fn monomial(i: usize, j: usize) -> f64 {
    fact(i) * fact(j) / fact(i + j + 2)
}
fn source(b: f64) -> NurbsGraphSolid {
    NurbsGraphSolid::new([20., 12., 3.], b, Tolerance::default()).unwrap()
}
fn triangle() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [0., 1.]]
}
#[test]
fn independent_simplex_polynomial_volume_centroid_and_rigid_placement() {
    for b in [-2., 0., 20.] {
        let s = source(b);
        let p = NurbsGraphPolygonSolid::new(&s, triangle(), Tolerance::default()).unwrap();
        let polynomial = [
            (0, 0, 3.),
            (1, 1, 4. * b),
            (2, 1, -4. * b),
            (1, 2, -4. * b),
            (2, 2, 4. * b),
        ];
        let integral = |di, dj| {
            polynomial
                .iter()
                .map(|&(i, j, c)| c * monomial(i + di, j + dj))
                .sum::<f64>()
        };
        let m = integral(0, 0);
        let square = polynomial
            .iter()
            .flat_map(|&(i, j, c)| {
                polynomial
                    .iter()
                    .map(move |&(k, l, d)| c * d * monomial(i + k, j + l))
            })
            .sum::<f64>();
        let expected = Point3::new(
            20. * integral(1, 0) / m,
            12. * integral(0, 1) / m,
            square / (2. * m),
        );
        let actual = p.mass_properties(Tolerance::default()).unwrap();
        assert!((actual.volume - 240. * m).abs() < 1e-10);
        assert!((actual.centroid - expected).norm() < 1e-12);
        let tr = Transform::translation(Vec3::new(40., -30., 10.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
            .unwrap();
        let placed = NurbsGraphPolygonSolid::new(
            &s.transformed(tr, Tolerance::default()).unwrap(),
            triangle(),
            Tolerance::default(),
        )
        .unwrap()
        .mass_properties(Tolerance::default())
        .unwrap();
        assert!((placed.volume - actual.volume).abs() < 1e-10);
        assert!((placed.centroid - tr.point(expected)).norm() < 1e-11);
    }
}
#[test]
fn exact_shared_brep_pcurves_and_conforming_closed_mesh_with_independent_bounds() {
    let p = NurbsGraphPolygonSolid::new(&source(20.), triangle(), Tolerance::default()).unwrap();
    let b = p.brep();
    assert_eq!(
        (b.vertices.len(), b.edges.len(), b.shell.faces.len()),
        (6, 9, 5)
    );
    let mut uses = [(0, 0); 9];
    for face in &b.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
                let range = b.edges[c.edge].curve.range();
                for i in 0..=50 {
                    let t = range[0] + (range[1] - range[0]) * i as f64 / 50.;
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
    let m = p
        .tessellate_bounded(0.5, 65536, Tolerance::default())
        .unwrap();
    let mut nodes = BTreeMap::new();
    let mut incidence = BTreeMap::new();
    for (i, &node) in m.vertex_nodes.iter().enumerate() {
        let position = m.mesh.positions[i];
        if let Some(old) = nodes.insert(node, position) {
            assert_eq!(old, position);
        }
    }
    for (ti, t) in m.mesh.triangles.iter().enumerate() {
        for k in 0..3 {
            let a = m.vertex_nodes[t[k]];
            let b = m.vertex_nodes[t[(k + 1) % 3]];
            assert_ne!(a, b);
            let entry = incidence.entry((a.min(b), a.max(b))).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
        let fi = m.vertex_faces[t[0]];
        for weights in [[1. / 3.; 3], [0.2, 0.3, 0.5], [0.1, 0.8, 0.1]] {
            let uv = std::array::from_fn::<_, 2, _>(|a| {
                (0..3).map(|j| weights[j] * m.vertex_uv[t[j]][a]).sum()
            });
            let surface = b.shell.faces[fi]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            let chord = (0..3).fold(Point3::new(0., 0., 0.), |sum, j| {
                sum + (m.mesh.positions[t[j]] - Point3::new(0., 0., 0.)) * weights[j]
            });
            assert!((surface - chord).norm() <= m.error_bounds[ti] + 1e-10);
        }
    }
    assert!(incidence.values().all(|x| *x == (2, 0)));
}
#[test]
fn invalid_polygon_contact_resources_and_canonical_mutations_reject() {
    let s = source(20.);
    for polygon in [
        vec![[0., 0.], [0., 1.], [1., 0.]],
        vec![[0., 0.], [1., 0.], [0.2, 0.2], [0., 1.]],
        vec![[0., 0.], [1., 0.], [0.5, 1e-12]],
        vec![[0., 0.], [1.1, 0.], [0., 1.]],
        vec![[f64::NAN, 0.], [1., 0.], [0., 1.]],
    ] {
        assert!(NurbsGraphPolygonSolid::new(&s, polygon, Tolerance::default()).is_err());
    }
    let p = NurbsGraphPolygonSolid::new(&s, triangle(), Tolerance::default()).unwrap();
    assert!(p
        .tessellate_bounded(1e-12, 12, Tolerance::default())
        .is_err());
    assert!(p
        .tessellate_bounded(f64::NAN, 65536, Tolerance::default())
        .is_err());
    let mut broken = p.clone();
    broken.solid.vertices[0].point.x += 1e-12;
    assert!(broken.validate(Tolerance::default()).is_err());
    assert!(broken.volume().is_err());
    assert!(broken.bounds().is_err());
    let mut broken = p.clone();
    broken.solid.shell.faces[0].wires[0].coedges[0].edge = 999;
    assert!(broken.mass_properties(Tolerance::default()).is_err());
    let mut broken = p;
    broken.solid.shell.faces[2].orientation = -1;
    assert!(broken
        .tessellate_bounded(0.5, 65536, Tolerance::default())
        .is_err());
}
