use hagane::*;
use std::collections::BTreeMap;
fn tol() -> Tolerance {
    Tolerance::default()
}
fn fact(n: usize) -> f64 {
    (1..=n).map(|n| n as f64).product()
}
fn tri_moment(i: usize, j: usize, a: f64, b: f64, s: f64) -> f64 {
    (0..=i)
        .flat_map(|k| {
            (0..=j).map(move |l| {
                fact(i) / (fact(k) * fact(i - k)) * fact(j) / (fact(l) * fact(j - l))
                    * a.powi((i - k) as i32)
                    * b.powi((j - l) as i32)
                    * s.powi((k + l + 2) as i32)
                    * fact(k)
                    * fact(l)
                    / fact(k + l + 2)
            })
        })
        .sum()
}
fn outer() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [0., 1.]]
}
fn hole() -> Vec<[f64; 2]> {
    vec![[0.2, 0.2], [0.4, 0.2], [0.2, 0.4]]
}
#[test]
fn independent_translated_simplex_polynomial_difference_moments_and_placement() {
    for bulge in [-2., 0., 20.] {
        let source = NurbsGraphSolid::new([20., 12., 3.], bulge, tol()).unwrap();
        let polygon = NurbsGraphPolygonSolid::new(&source, outer(), tol()).unwrap();
        let body = polygon.through_uv_polygon(hole(), tol()).unwrap();
        let p = [
            (0, 0, 3.),
            (1, 1, 4. * bulge),
            (2, 1, -4. * bulge),
            (1, 2, -4. * bulge),
            (2, 2, 4. * bulge),
        ];
        let integral = |i, j| tri_moment(i, j, 0., 0., 1.) - tri_moment(i, j, 0.2, 0.2, 0.2);
        let moment = |di, dj| {
            p.iter()
                .map(|&(i, j, c)| c * integral(i + di, j + dj))
                .sum::<f64>()
        };
        let square = p
            .iter()
            .flat_map(|&(i, j, c)| {
                p.iter()
                    .map(move |&(k, l, d)| c * d * integral(i + k, j + l))
            })
            .sum::<f64>();
        let m = moment(0, 0);
        let c = Point3::new(
            20. * moment(1, 0) / m,
            12. * moment(0, 1) / m,
            square / (2. * m),
        );
        let mass = body.mass_properties(tol()).unwrap();
        assert!((mass.volume - 240. * m).abs() < 1e-10);
        assert!((mass.centroid - c).norm() < 1e-12);
        let tr = Transform::translation(Vec3::new(40., -30., 10.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
            .unwrap();
        let placed =
            NurbsGraphPolygonSolid::new(&source.transformed(tr, tol()).unwrap(), outer(), tol())
                .unwrap()
                .through_uv_polygon(hole(), tol())
                .unwrap()
                .mass_properties(tol())
                .unwrap();
        assert!((placed.volume - mass.volume).abs() < 1e-10);
        assert!((placed.centroid - tr.point(c)).norm() < 1e-11);
    }
}
fn clipped_area(mut polygon: Vec<[f64; 2]>, hole: &[[f64; 2]]) -> f64 {
    for edge in 0..hole.len() {
        let a = hole[edge];
        let b = hole[(edge + 1) % hole.len()];
        let distance = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let old = polygon;
        polygon = vec![];
        if old.is_empty() {
            return 0.;
        }
        for i in 0..old.len() {
            let p = old[i];
            let q = old[(i + 1) % old.len()];
            let x = distance(p);
            let y = distance(q);
            if x >= 0. {
                polygon.push(p);
            }
            if (x < 0.) != (y < 0.) {
                let f = x / (x - y);
                polygon.push([p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1])]);
            }
        }
    }
    if polygon.is_empty() {
        return 0.;
    }
    polygon
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = polygon[(i + 1) % polygon.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        .abs()
        / 2.
}
#[test]
fn exact_annular_topology_inward_walls_and_material_only_conforming_mesh() {
    let source = NurbsGraphSolid::new([20., 12., 3.], 20., tol()).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, outer(), tol())
        .unwrap()
        .through_uv_polygon(hole(), tol())
        .unwrap();
    let b = body.brep();
    assert_eq!(
        (b.vertices.len(), b.edges.len(), b.shell.faces.len()),
        (12, 18, 8)
    );
    assert_eq!(
        b.vertices.len() as isize - b.edges.len() as isize
            + b.shell
                .faces
                .iter()
                .map(|f| 2 - f.wires.len() as isize)
                .sum::<isize>(),
        0
    );
    let mut uses = [(0, 0); 18];
    for face in &b.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
                for i in 0..=20 {
                    let t = i as f64 / 20.;
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
    for cap in 0..2 {
        assert_eq!(b.shell.faces[cap].wires.len(), 2);
    }
    for face in &b.shell.faces[5..] {
        assert_eq!(face.orientation, -1);
    }
    let mesh = body.tessellate_bounded(0.5, 65536, tol()).unwrap();
    let mut nodes = BTreeMap::new();
    let mut incidence = BTreeMap::new();
    let mut area = 0.;
    for (i, &node) in mesh.vertex_nodes.iter().enumerate() {
        if let Some(old) = nodes.insert(node, mesh.mesh.positions[i]) {
            assert_eq!(old, mesh.mesh.positions[i]);
        }
    }
    for (ti, t) in mesh.mesh.triangles.iter().enumerate() {
        for j in 0..3 {
            let a = mesh.vertex_nodes[t[j]];
            let z = mesh.vertex_nodes[t[(j + 1) % 3]];
            let x = incidence.entry((a.min(z), a.max(z))).or_insert((0, 0));
            x.0 += 1;
            x.1 += if a < z { 1 } else { -1 };
        }
        let face = mesh.mesh.face_ids[ti];
        if face < 2 {
            let uv = t.iter().map(|&i| mesh.vertex_uv[i]).collect::<Vec<_>>();
            assert!(clipped_area(uv.clone(), &hole()) < 1e-14);
            if face == 1 {
                let [a, z, c] = [uv[0], uv[1], uv[2]];
                area += ((z[0] - a[0]) * (c[1] - a[1]) - (z[1] - a[1]) * (c[0] - a[0])).abs() / 2.;
            }
        }
        for weights in [[1. / 3.; 3], [0.2, 0.3, 0.5]] {
            let uv = std::array::from_fn::<_, 2, _>(|a| {
                (0..3).map(|j| weights[j] * mesh.vertex_uv[t[j]][a]).sum()
            });
            let exact = b.shell.faces[face]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            let chord = (0..3).fold(Point3::new(0., 0., 0.), |sum, j| {
                sum + (mesh.mesh.positions[t[j]] - Point3::new(0., 0., 0.)) * weights[j]
            });
            assert!((exact - chord).norm() <= mesh.error_bounds[ti] + 1e-10);
        }
    }
    assert!(incidence.values().all(|x| *x == (2, 0)));
    assert!((area - 0.48).abs() < 1e-12);
}
#[test]
fn invalid_openings_contacts_precision_and_public_mutations_reject() {
    let s = NurbsGraphSolid::new([20., 12., 3.], 20., tol()).unwrap();
    let p = NurbsGraphPolygonSolid::new(&s, outer(), tol()).unwrap();
    for h in [
        vec![[0.2, 0.2], [0.2, 0.4], [0.4, 0.2]],
        vec![[0., 0.2], [0.2, 0.2], [0.2, 0.4]],
        vec![[0.6, 0.2], [0.9, 0.2], [0.6, 0.5]],
        vec![[0.2, 0.2], [0.4, 0.2], [0.3, 0.2 + 1e-12]],
        vec![[f64::NAN, 0.2], [0.4, 0.2], [0.2, 0.4]],
    ] {
        assert!(p.through_uv_polygon(h, tol()).is_err());
    }
    let h = p.through_uv_polygon(hole(), tol()).unwrap();
    let mut bad = h.clone();
    bad.solid.vertices[0].point.x += 1e-12;
    assert!(bad.validate(tol()).is_err());
    assert!(bad.volume().is_err());
    assert!(bad.bounds().is_err());
    let mut bad = h.clone();
    bad.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(bad.mass_properties(tol()).is_err());
    let mut bad = h;
    bad.solid.shell.faces[5].orientation = 1;
    assert!(bad.tessellate_bounded(0.5, 65536, tol()).is_err());
}
#[test]
fn near_filled_flat_stock_uses_positive_material_moments_without_subtraction() {
    let tolerance = Tolerance::new(1e-10).unwrap();
    let s = NurbsGraphSolid::new([1., 1., 1.], 0., tolerance).unwrap();
    let e = 1e-5;
    let opening = vec![[e, e], [1. - 2. * e, e], [1. - 2. * e, 1. - e], [e, 1. - e]];
    let h = s.through_uv_polygon(opening, tolerance).unwrap();
    let rectangles = [
        [[0., e], [0., 1.]],
        [[1. - 2. * e, 1.], [0., 1.]],
        [[e, 1. - 2. * e], [0., e]],
        [[e, 1. - 2. * e], [1. - e, 1.]],
    ];
    let mut volume = 0.;
    let mut moments = [0.; 2];
    for r in rectangles {
        let area = (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]);
        volume += area;
        for a in 0..2 {
            moments[a] += area * (r[a][0] + r[a][1]) / 2.;
        }
    }
    let m = h.mass_properties(tolerance).unwrap();
    assert!((m.volume / volume - 1.).abs() < 1e-12);
    assert!((m.centroid.x - moments[0] / volume).abs() < 1e-12);
    assert!((m.centroid.y - moments[1] / volume).abs() < 1e-12);
    assert!((m.centroid.z - 0.5).abs() < 1e-12);
}
