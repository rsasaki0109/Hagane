use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, PI};

fn review(result: &ParallelBoxEdgeFillets, height: f64, request: &[(usize, f64)]) {
    let s = result.solid();
    let tol = Tolerance::new(1e-6).unwrap();
    s.validate(tol).unwrap();
    let removed = request
        .iter()
        .map(|(_, r)| r * r * (1. - PI / 4.) * height)
        .sum::<f64>();
    assert!((result.removed_volume() - removed).abs() < 1e-8);
    assert!((s.volume().unwrap() - (80. * 60. * 20. - removed)).abs() < 1e-7);
    assert_eq!(result.selections(), request);
    assert_eq!(result.fillet_faces().len(), request.len());
    let mut uses = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for (fi, face) in s.shell.faces.iter().enumerate() {
        for c in &face.wires[0].coedges {
            uses.entry(c.edge)
                .or_default()
                .push((fi, c.forward == (face.orientation == 1)));
            let range = s.edges[c.edge].curve.range();
            for k in 0..=12 {
                let t = range[0] + (range[1] - range[0]) * k as f64 / 12.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - s.edges[c.edge].curve.try_evaluate(t).unwrap())
                    .norm()
                        < 1e-9
                );
            }
        }
    }
    assert_eq!(uses.len(), s.edges.len());
    assert!(uses.values().all(|u| u.len() == 2 && u[0].1 != u[1].1));
    let error = 0.03;
    let mesh = s.tessellate(error, tol).unwrap();
    let world = mesh
        .positions
        .iter()
        .fold(1_f64, |w, p| w.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
    let guard = 4096. * f64::EPSILON * world;
    // Canonical sample identities come from actual edge parameters, with exact
    // BRep vertex identities at endpoints; no coordinate bins are used.
    let mut nodes: Vec<_> = s.vertices.iter().map(|v| v.point).collect();
    for (ei, edge) in s.edges.iter().enumerate() {
        if let Curve::Arc { radius, sweep, .. } = edge.curve {
            assert!((sweep - FRAC_PI_2).abs() < 1e-12);
            let fi = uses[&ei]
                .iter()
                .map(|u| u.0)
                .find(|i| result.fillet_faces().contains(i))
                .unwrap();
            let n = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
            assert!(radius * (1. - (sweep / (2. * n as f64)).cos()) <= error);
            for k in 1..n {
                nodes.push(
                    edge.curve
                        .try_evaluate(sweep * k as f64 / n as f64)
                        .unwrap(),
                );
            }
        }
    }
    let ids: Vec<_> = mesh
        .positions
        .iter()
        .map(|p| {
            let found: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, q)| (**q - *p).norm() <= guard)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(found.len(), 1, "unique actual edge-parameter sample");
            found[0]
        })
        .collect();
    let mut incidence = BTreeMap::new();
    let mut areas = BTreeMap::<usize, f64>::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        assert!(cross.dot(mesh.normals[tri[0]]) > 0.);
        *areas.entry(fi).or_default() += cross.norm() * 0.5;
        if result.fillet_faces().contains(&fi) {
            let uv = p.map(|q| {
                let mut uv = s.shell.faces[fi].surface.parameters(q);
                if uv[0] > PI {
                    uv[0] -= 2. * PI;
                }
                uv
            });
            let a = (uv[0][0] + uv[1][0] + uv[2][0]) / 3.;
            let b = (uv[0][1] + uv[1][1] + uv[2][1]) / 3.;
            let point = (p[0] + p[1] + p[2]) * (1. / 3.);
            assert!(
                (s.shell.faces[fi].surface.try_evaluate(a, b).unwrap() - point).norm()
                    <= error + guard
            );
        }
        for k in 0..3 {
            let a = ids[tri[k]];
            let b = ids[tri[(k + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = incidence.entry(key).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(incidence.values().all(|&(n, d)| n == 2 && d == 0));
    for &fi in result.fillet_faces() {
        let Surface::FramedCylinder {
            radius,
            height: actual_height,
            ..
        } = s.shell.faces[fi].surface
        else {
            panic!("actual cylinder")
        };
        assert!((actual_height - height).abs() < 1e-9);
        let n = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
        let chord_area = height * 2. * radius * n as f64 * (FRAC_PI_2 / (2. * n as f64)).sin();
        assert!((areas[&fi] - chord_area).abs() < 1e-8);
        assert!(areas[&fi] < FRAC_PI_2 * radius * height);
        for c in &s.shell.faces[fi].wires[0].coedges {
            if !matches!(s.edges[c.edge].curve, Curve::Line { .. }) {
                continue;
            }
            let other = uses[&c.edge]
                .iter()
                .map(|u| u.0)
                .find(|&i| i != fi)
                .unwrap();
            let uv = c.pcurve.try_evaluate(0.5).unwrap();
            let n1 =
                s.shell.faces[fi].surface.normal(uv[0]) * f64::from(s.shell.faces[fi].orientation);
            let n2 = s.shell.faces[other].surface.normal(0.)
                * f64::from(s.shell.faces[other].orientation);
            assert!(n1.dot(n2) > 1. - 1e-12, "tangent source plane");
        }
    }
    assert!(export_step_mm(s, tol).is_err());
    let step = export_step_bounded_analytic_mm(s, tol.linear).unwrap();
    assert!(step.contains("CIRCLE") && step.contains("CYLINDRICAL_SURFACE"));
}

#[test]
fn all_parallel_subsets_axes_and_arbitrary_pose_retain_actual_circular_geometry() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tol.absolute(),
    )
    .unwrap();
    for (edges, height) in [
        ([0, 2, 4, 6], 80.),
        ([1, 3, 5, 7], 60.),
        ([8, 9, 10, 11], 20.),
    ] {
        for subset in 1..16 {
            let request: Vec<_> = edges
                .iter()
                .enumerate()
                .filter(|(i, _)| subset & (1 << i) != 0)
                .map(|(i, &edge)| (edge, 2. + i as f64 * 0.25))
                .collect();
            review(
                &fillet_parallel_box_edges(&source, &request, tol).unwrap(),
                height,
                &request,
            );
        }
    }
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    review(
        &fillet_parallel_box_edges(
            &source.transformed(frame, tol.absolute()).unwrap(),
            &[(8, 2.), (9, 3.), (10, 4.), (11, 2.5)],
            tol,
        )
        .unwrap(),
        20.,
        &[(8, 2.), (9, 3.), (10, 4.), (11, 2.5)],
    );
}

#[test]
fn nonboxes_mixed_axes_contact_and_unresolved_coordinates_are_rejected() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tol.absolute(),
    )
    .unwrap();
    let before = export_step_mm(&source, tol.absolute()).unwrap();
    for request in [
        vec![],
        vec![(8, 2.), (0, 2.)],
        vec![(8, 2.), (8, 3.)],
        vec![(8, 0.)],
        vec![(8, f64::NAN)],
        vec![(99, 2.)],
        vec![(8, 30.), (9, 30.), (10, 30.), (11, 30.)],
    ] {
        assert!(fillet_parallel_box_edges(&source, &request, tol).is_err());
        assert_eq!(export_step_mm(&source, tol.absolute()).unwrap(), before);
    }
    let skew = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[0., 0.], [80., 0.], [85., 60.], [5., 60.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 20.),
        tol.absolute(),
    )
    .unwrap();
    assert_eq!(
        (
            skew.vertices.len(),
            skew.edges.len(),
            skew.shell.faces.len()
        ),
        (8, 12, 6)
    );
    assert!(fillet_parallel_box_edges(&skew, &[(8, 2.)], tol).is_err());
    let remote = source
        .transformed(
            Transform::translation(Vec3::new(1e10, 0., 0.)).unwrap(),
            tol.absolute(),
        )
        .unwrap();
    assert!(fillet_parallel_box_edges(&remote, &[(8, 2.)], tol).is_err());
    let mut malformed = source.clone();
    let Surface::Plane { ref mut u, .. } = malformed.shell.faces[0].surface else {
        unreachable!()
    };
    *u = *u * 1.01;
    assert!(fillet_parallel_box_edges(&malformed, &[(8, 2.)], tol).is_err());
}
