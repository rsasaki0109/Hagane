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

fn tol() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn circle(center: [f64; 2], radius: f64) -> Vec<PlanarSegment> {
    (0..4)
        .map(|i| PlanarSegment::Arc {
            center,
            radius,
            start_angle: i as f64 * PI / 2.,
            sweep: PI / 2.,
        })
        .collect()
}
fn stock(third: bool) -> Solid {
    let p = rounded_rectangle_profile(
        Point3::new(0., 0., 0.),
        20.,
        16.,
        2.,
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    let mut holes = vec![circle([-4., 0.], 1.5), circle([4., 0.], 1.5)];
    if third {
        holes.push(circle([0., 4.], 0.75));
    }
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: p.origin,
            outer: p.segments,
            holes,
        },
        5.,
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap()
}
fn plane(y: f64, angle: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(0., y, 0.),
        u: Vec3::new(-angle.cos(), angle.sin(), 0.),
        v: Vec3::new(0., 0., 1.),
    }
}
fn verify(
    source: &Solid,
    p: &Surface,
    expected_positive: f64,
    nsections: usize,
) -> NormalArcLinePrismPlaneSplitComponents {
    let before = format!("{source:?}");
    let r = split_normal_arc_line_prism_by_plane_components(source, p, tol()).unwrap();
    assert_eq!(before, format!("{source:?}"));
    assert_eq!(r.sections().len(), nsections);
    let positive: f64 = r.positive().iter().map(|b| b.volume().unwrap()).sum();
    let negative: f64 = r.negative().iter().map(|b| b.volume().unwrap()).sum();
    assert!(
        (positive - expected_positive).abs() < 1e-8,
        "{positive} vs {expected_positive}"
    );
    assert!((positive + negative - source.volume().unwrap()).abs() < 1e-8);
    let Surface::Plane { origin, u, v } = *p else {
        unreachable!()
    };
    let normal = u.cross(v).normalized().unwrap();
    for (bodies, side) in [(r.negative(), -1.), (r.positive(), 1.)] {
        for body in bodies {
            check_closed(body, 1., 200.);
            assert!(body
                .vertices
                .iter()
                .all(|v| (v.point - origin).dot(normal) * side >= -1e-10));
            let step = export_step_bounded_analytic_mm(body, 1e-6).unwrap();
            let imported =
                import_step_bounded_analytic_mm(&step, Tolerance::new(1e-6).unwrap()).unwrap();
            assert_eq!(euler(&imported), euler(body));
            assert!((imported.volume().unwrap() - body.volume().unwrap()).abs() < 1e-8);
        }
    }

    for old in &source.edges {
        let range = old.curve.range();
        for k in 0..=16 {
            let point = old
                .curve
                .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / 16.)
                .unwrap();
            assert!(
                r.negative()
                    .iter()
                    .chain(r.positive())
                    .flat_map(|b| &b.edges)
                    .any(|edge| match edge.curve {
                        Curve::Line { a, b } => {
                            let d = b - a;
                            let t = (point - a).dot(d) / d.dot(d);
                            (-1e-10..=1. + 1e-10).contains(&t) && (a + d * t - point).norm() < 1e-10
                        }
                        Curve::Arc {
                            frame,
                            radius,
                            sweep,
                        } => {
                            let q = frame.local_point(point);
                            let mut angle = q.y.atan2(q.x);
                            if angle < -1e-10 {
                                angle += 2. * PI;
                            }
                            q.z.abs() < 1e-10
                                && (q.x.hypot(q.y) - radius).abs() < 1e-10
                                && (-1e-10..=sweep + 1e-10).contains(&angle)
                        }
                        _ => false,
                    }),
                "retained source edge point {point:?}"
            );
        }
    }
    for section in r.sections() {
        assert_eq!(section.rings.len(), 1);
        assert_eq!(section.rings[0].len(), 4);
        let center = section.rings[0]
            .iter()
            .copied()
            .fold(Vec3::new(0., 0., 0.), |a, b| a + b)
            * 0.25;
        for (bodies, side) in [(r.negative(), -1.), (r.positive(), 1.)] {
            let cuts: Vec<_> = bodies
                .iter()
                .flat_map(|b| b.shell.faces.iter().map(move |f| (b, f)))
                .filter(|(b, f)| {
                    f.wires.len() == 1
                        && f.wires[0].coedges.len() == 4
                        && section.rings[0].iter().all(|p| {
                            f.wires[0].coedges.iter().any(|c| {
                                b.edges[c.edge]
                                    .vertices
                                    .iter()
                                    .any(|&i| (b.vertices[i].point - *p).norm() < 1e-10)
                            })
                        })
                })
                .collect();
            assert_eq!(cuts.len(), 1);
            let (body, face) = cuts[0];
            let Surface::Plane { u, v, .. } = face.surface else {
                panic!("cut is actual plane")
            };
            assert!((u.cross(v) * f64::from(face.orientation)).dot(normal) * side < -1. + 1e-12);
            assert_eq!(
                classify_point_in_solid(body, center, tol()).unwrap(),
                PointLocation::Boundary
            );
            assert_eq!(
                classify_point_in_solid(body, center + normal * (side * 0.01), tol()).unwrap(),
                PointLocation::Inside
            );
        }
    }
    r
}
#[test]
fn crossed_quarters_have_three_exact_sections_and_analytic_circle_segment_volume() {
    let s = stock(false);
    let expected = (304. + 4. * PI - 4.5 * PI) * 5.;
    assert!((s.volume().unwrap() - expected).abs() < 1e-8);
    let p = plane(0., 0.2);
    assert!(split_normal_arc_line_prism_by_plane(&s, &p, tol()).is_err());
    let r = verify(&s, &p, expected / 2., 3);
    assert_eq!((r.negative().len(), r.positive().len()), (1, 1));
    assert_eq!(euler(&r.negative()[0]), 2);
    assert_eq!(euler(&r.positive()[0]), 2);
    let s = stock(true);
    let d: f64 = 0.6;
    let radius: f64 = 1.5;
    let cap = radius * radius * (d / radius).acos() - d * (radius * radius - d * d).sqrt();
    let positive = ((304. + 4. * PI) / 2. - 20. * d - 2. * cap - PI * 0.75 * 0.75) * 5.;
    let r = verify(&s, &plane(d, 0.), positive, 3);
    assert_eq!(euler(&r.negative()[0]), 2);
    assert_eq!(euler(&r.positive()[0]), 0);
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let posed = s
        .transformed(transform, Tolerance::new(1e-6).unwrap())
        .unwrap();
    verify(
        &posed,
        &plane(d, 0.).transformed(transform).unwrap(),
        positive,
        3,
    );
}
fn concave() -> Solid {
    let points = [
        [-5., -4.],
        [6., -4.],
        [6., 6.],
        [2., 6.],
        [2., 0.],
        [-2., 0.],
        [-2., 6.],
        [-6., 6.],
        [-6., -3.],
    ];
    let mut outer: Vec<_> = points
        .windows(2)
        .map(|p| PlanarSegment::Line { a: p[0], b: p[1] })
        .collect();
    outer.push(PlanarSegment::Arc {
        center: [-5., -3.],
        radius: 1.,
        start_angle: PI,
        sweep: PI / 2.,
    });
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: vec![],
        },
        5.,
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap()
}
#[test]
fn concave_arc_profile_has_two_disconnected_positive_plain_components() {
    let s = concave();
    assert!((s.volume().unwrap() - (95. + PI / 4.) * 5.).abs() < 1e-8);
    let r = verify(&s, &plane(2., 0.), 160., 2);
    assert_eq!((r.negative().len(), r.positive().len()), (1, 2));
    for b in r.positive() {
        assert!((b.volume().unwrap() - 80.).abs() < 1e-8);
        assert_eq!(euler(b), 2);
        assert!(b
            .edges
            .iter()
            .all(|e| matches!(e.curve, Curve::Line { .. })));
    }
    assert!(r.negative()[0]
        .edges
        .iter()
        .any(|e| matches!(e.curve, Curve::Arc { .. })));
}
#[test]
fn contact_skew_and_malformed_errors_do_not_mutate_source() {
    let s = stock(false);
    let before = format!("{s:?}");
    for p in [
        plane(1.5, 0.),
        plane(0., 0.),
        plane(20., 0.),
        Surface::Plane {
            origin: Point3::new(0., 0., 0.),
            u: Vec3::new(1., 0., 0.),
            v: Vec3::new(0., 0.1, 1.),
        },
    ] {
        assert!(split_normal_arc_line_prism_by_plane_components(&s, &p, tol()).is_err());
        assert_eq!(format!("{s:?}"), before);
    }
    let mut malformed = s.clone();
    malformed.shell.faces[0].orientation = 0;
    assert!(
        split_normal_arc_line_prism_by_plane_components(&malformed, &plane(0., 0.2), tol())
            .is_err()
    );
}
