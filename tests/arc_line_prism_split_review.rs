use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

fn euler(s: &Solid) -> isize {
    s.vertices.len() as isize - s.edges.len() as isize
        + s.shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>()
}
fn check_closed(s: &Solid, scale: f64, world_scale: f64) {
    let tol = Tolerance::new(1e-6 * scale).unwrap();
    s.validate(tol).unwrap();
    let guard = 8192. * f64::EPSILON * world_scale;
    let mut uses = vec![Vec::new(); s.edges.len()];
    for (fi, f) in s.shell.faces.iter().enumerate() {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push((fi, c.forward == (f.orientation == 1)));
            let curve = &s.edges[c.edge].curve;
            let range = curve.range();
            for i in 0..=16 {
                let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (f.surface.try_evaluate(uv[0], uv[1]).unwrap()
                        - curve.try_evaluate(t).unwrap())
                    .norm()
                        <= guard
                );
            }
        }
    }
    assert!(uses.iter().all(|u| u.len() == 2 && u[0].1 != u[1].1));
    let error = 0.02 * scale;
    let mesh = s.tessellate(error, tol).unwrap();
    let mut nodes: Vec<_> = s.vertices.iter().map(|v| v.point).collect();
    for (ei, e) in s.edges.iter().enumerate() {
        if let Curve::Arc { radius, sweep, .. } = e.curve {
            let fi = uses[ei]
                .iter()
                .map(|u| u.0)
                .find(|&i| matches!(s.shell.faces[i].surface, Surface::FramedCylinder { .. }))
                .unwrap();
            let n = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
            assert!(radius * (1. - (sweep / (2. * n as f64)).cos()) <= error);
            let range = e.curve.range();
            for k in 1..n {
                nodes.push(
                    e.curve
                        .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / n as f64)
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
            assert_eq!(found.len(), 1);
            found[0]
        })
        .collect();
    let mut incidence = BTreeMap::new();
    for (tri, &fi) in mesh.triangles.iter().zip(&mesh.face_ids) {
        let p = tri.map(|i| mesh.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for k in 0..3 {
            let a = ids[tri[k]];
            let b = ids[tri[(k + 1) % 3]];
            assert_ne!(a, b);
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = incidence.entry(key).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
        let midpoint = (p[0] + p[1] + p[2]) * (1. / 3.);
        match s.shell.faces[fi].surface {
            Surface::FramedCylinder { frame, radius, .. } => {
                let q = frame.local_point(midpoint);
                assert!((q.x.hypot(q.y) - radius).abs() <= error + guard);
            }
            Surface::Plane { origin, u, v } => {
                assert!((midpoint - origin).dot(u.cross(v)).abs() <= guard)
            }
            _ => panic!("actual analytic BRep"),
        }
    }
    assert!(incidence.values().all(|&(n, d)| n == 2 && d == 0));
    assert_eq!(
        nodes.len() as isize - incidence.len() as isize + mesh.triangles.len() as isize,
        euler(s)
    );
}

fn plane(x: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(x, 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    }
}
fn source(with_hole: bool) -> Solid {
    let t = Tolerance::new(1e-6).unwrap();
    let profile = rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., t).unwrap();
    let holes = if with_hole {
        vec![(0..4)
            .map(|i| PlanarSegment::Arc {
                center: [-4., 0.],
                radius: 1.,
                start_angle: i as f64 * PI / 2.,
                sweep: PI / 2.,
            })
            .collect()]
    } else {
        vec![]
    };
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: profile.origin,
            outer: profile.segments,
            holes,
        },
        5.,
        t,
    )
    .unwrap()
}
fn test_split(
    source: &Solid,
    plane: &Surface,
    positive_volume: f64,
    negative_genus: usize,
    positive_genus: usize,
) {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let before = export_step_bounded_analytic_mm(source, tol.linear()).unwrap();
    let result = split_normal_arc_line_prism_by_plane(source, plane, tol).unwrap();
    assert_eq!(
        export_step_bounded_analytic_mm(source, tol.linear()).unwrap(),
        before
    );
    assert!((result.positive().volume().unwrap() - positive_volume).abs() < 1e-8);
    assert!(
        (result.negative().volume().unwrap() - (source.volume().unwrap() - positive_volume)).abs()
            < 1e-8
    );
    let Surface::Plane { origin, u, v } = *plane else {
        unreachable!()
    };
    let normal = u.cross(v).normalized().unwrap();
    assert_eq!(result.section().rings.len(), 1);
    assert_eq!(result.section().rings[0].len(), 4);
    let section = &result.section().rings[0];
    let midpoint = section.iter().fold(Vec3::new(0., 0., 0.), |a, p| a + *p) * (1. / 4.);
    for (body, genus, side) in [
        (result.negative(), negative_genus, -1.),
        (result.positive(), positive_genus, 1.),
    ] {
        assert_eq!(euler(body), 2 - 2 * genus as isize);
        assert_eq!(
            classify_point_in_solid(body, midpoint, tol).unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            classify_point_in_solid(body, midpoint + normal * (side * 0.1), tol).unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            classify_point_in_solid(body, midpoint - normal * (side * 0.1), tol).unwrap(),
            PointLocation::Outside
        );
        check_closed(body, 1., 200.);
        assert!(body
            .vertices
            .iter()
            .all(|p| (p.point - origin).dot(normal) * side >= -1e-10));
        let cut =
            body.shell
                .faces
                .iter()
                .find(|f| {
                    f.wires.len() == 1
                        && f.wires[0].coedges.iter().all(|c| {
                            body.edges[c.edge].vertices.iter().all(|&i| {
                                (body.vertices[i].point - origin).dot(normal).abs() < 1e-10
                            })
                        })
                })
                .unwrap();
        assert_eq!(cut.wires[0].coedges.len(), 4);
        let Surface::Plane { u: cu, v: cv, .. } = cut.surface else {
            unreachable!()
        };
        assert!((cu.cross(cv) * f64::from(cut.orientation)).dot(normal) * side < -1. + 1e-12);
        for p in section {
            assert!(cut.wires[0].coedges.iter().any(|c| body.edges[c.edge]
                .vertices
                .iter()
                .any(|&i| (body.vertices[i].point - *p).norm() < 1e-10)));
        }
        for edge in &body.edges {
            let range = edge.curve.range();
            for k in 0..=12 {
                let p = edge
                    .curve
                    .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / 12.)
                    .unwrap();
                if (p - origin).dot(normal).abs() < 1e-10 {
                    continue;
                }
                assert!(
                    source.edges.iter().any(|old| match old.curve {
                        Curve::Line { a, b } => {
                            let direction = b - a;
                            let q = (p - a).dot(direction) / direction.dot(direction);
                            (-1e-10..=1. + 1e-10).contains(&q)
                                && (a + direction * q - p).norm() < 1e-10
                        }
                        Curve::Arc {
                            frame,
                            radius,
                            sweep,
                        } => {
                            let q = frame.local_point(p);
                            let mut t = q.y.atan2(q.x);
                            if t < -1e-10 {
                                t += 2. * PI;
                            }
                            q.z.abs() < 1e-10
                                && (q.x.hypot(q.y) - radius).abs() < 1e-10
                                && t >= -1e-10
                                && t <= sweep + 1e-10
                        }
                        _ => false,
                    }),
                    "every uncut boundary point retains an actual source edge"
                );
            }
        }
        let text = export_step_bounded_analytic_mm(body, tol.linear()).unwrap();
        let imported = import_step_bounded_analytic_mm(&text, tol.absolute()).unwrap();
        check_closed(&imported, 1., 200.);
        assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-8);
    }
}

#[test]
fn axial_round_stock_splits_have_independent_areas_arc_retention_and_hole_ownership() {
    let stock = source(false);
    let total = (320. - 16. * (1. - PI / 4.)) * 5.;
    assert!((stock.volume().unwrap() - total).abs() < 1e-9);
    test_split(&stock, &plane(0.), total / 2., 0, 0);
    let angle = 0.6_f64;
    let diagonal = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(-angle.sin(), angle.cos(), 0.),
        v: Vec3::new(0., 0., 1.),
    };
    test_split(&stock, &diagonal, total / 2., 0, 0);
    test_split(&stock, &plane(3.), total / 2. - 3. * 16. * 5., 0, 0);
    let d = 1.4_f64;
    let cap = 2. * (d / 2.).acos() - 0.5 * d * (4. - d * d).sqrt();
    let right = (12. * 0.6 + 2. * cap) * 5.;
    test_split(&stock, &plane(9.4), right, 0, 0);
    let holed = source(true);
    test_split(&holed, &plane(3.), total / 2. - 3. * 16. * 5., 1, 0);
    let pose = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    test_split(
        &holed
            .transformed(pose, Tolerance::new(1e-6).unwrap())
            .unwrap(),
        &plane(3.).transformed(pose).unwrap(),
        total / 2. - 3. * 16. * 5.,
        1,
        0,
    );
}

#[test]
fn valid_split_can_produce_a_real_line_only_child() {
    let t = Tolerance::new(1e-6).unwrap();
    let segments = vec![
        PlanarSegment::Line {
            a: [0., -4.],
            b: [20., -4.],
        },
        PlanarSegment::Line {
            a: [20., -4.],
            b: [20., 4.],
        },
        PlanarSegment::Line {
            a: [20., 4.],
            b: [0., 4.],
        },
        PlanarSegment::Arc {
            center: [0., 0.],
            radius: 4.,
            start_angle: PI / 2.,
            sweep: PI / 2.,
        },
        PlanarSegment::Arc {
            center: [0., 0.],
            radius: 4.,
            start_angle: PI,
            sweep: PI / 2.,
        },
    ];
    let stock = extrude_arc_line(
        &ArcLineProfile {
            origin: Point3::new(0., 0., 0.),
            segments,
        },
        5.,
        t,
    )
    .unwrap();
    test_split(&stock, &plane(10.), 400., 0, 0);
}

#[test]
fn contacts_multiple_intervals_tilt_and_world_precision_reject_without_mutation() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let stock = source(false);
    let before = export_step_bounded_analytic_mm(&stock, tol.linear()).unwrap();
    let tilted = plane(0.)
        .transformed(Transform::rotation(Vec3::new(0., 1., 0.), 0.01).unwrap())
        .unwrap();
    let remote = Surface::Plane {
        origin: Point3::new(0., 1e10, 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    };
    for candidate in [plane(10.), plane(8.), plane(11.), tilted, remote] {
        assert!(split_normal_arc_line_prism_by_plane(&stock, &candidate, tol).is_err());
        assert_eq!(
            export_step_bounded_analytic_mm(&stock, tol.linear()).unwrap(),
            before
        );
    }
    let holed = source(true);
    assert!(split_normal_arc_line_prism_by_plane(&holed, &plane(-4.), tol).is_err());
    assert!(split_normal_arc_line_prism_by_plane(&holed, &plane(-3.), tol).is_err());
    let far = stock
        .transformed(
            Transform::translation(Vec3::new(1e10, 0., 0.)).unwrap(),
            tol.absolute(),
        )
        .unwrap();
    assert!(split_normal_arc_line_prism_by_plane(&far, &plane(1e10), tol).is_err());
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., tol.absolute()).unwrap();
    let skew = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: profile.origin,
            outer: profile.segments,
            holes: vec![],
        },
        Vec3::new(1., 0., 5.),
        tol.absolute(),
    )
    .unwrap();
    assert!(split_normal_arc_line_prism_by_plane(&skew, &plane(0.), tol).is_err());
    let mut malformed = stock.clone();
    malformed.shell.faces[0].orientation *= -1;
    assert!(split_normal_arc_line_prism_by_plane(&malformed, &plane(0.), tol).is_err());
}
