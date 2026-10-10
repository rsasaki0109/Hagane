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

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn block(scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-10. * scale, -8. * scale, 0.),
            size: Vec3::new(20. * scale, 16. * scale, 5. * scale),
        },
        Tolerance::new(1e-6 * scale).unwrap(),
    )
    .unwrap()
}
fn plane(x: f64) -> Surface {
    Surface::Plane {
        origin: Point3::new(x, 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0., 0., 1.),
    }
}
fn retained_lines(source: &Solid, kept: &Solid, scale: f64) {
    for edge in &source.edges {
        let range = edge.curve.range();
        for k in 0..=16 {
            let point = edge
                .curve
                .try_evaluate(range[0] + (range[1] - range[0]) * k as f64 / 16.)
                .unwrap();
            assert!(kept
                .edges
                .iter()
                .any(|e| if let Curve::Line { a, b } = e.curve {
                    let d = b - a;
                    let t = (point - a).dot(d) / d.dot(d);
                    (-1e-10..=1. + 1e-10).contains(&t) && (a + d * t - point).norm() < 1e-10 * scale
                } else {
                    false
                }));
        }
    }
}
fn check(body: &Solid, volume: f64, genus: usize, scale: f64) {
    assert!((body.volume().unwrap() - volume).abs() < 1e-9 * scale.powi(3));
    assert_eq!(euler(body), 2 - 2 * genus as isize);
    check_closed(body, scale, 200. * scale);
    let encoded = export_step_bounded_analytic_mm(body, 1e-6 * scale).unwrap();
    let imported =
        import_step_bounded_analytic_mm(&encoded, Tolerance::new(1e-6 * scale).unwrap()).unwrap();
    assert!((imported.volume().unwrap() - volume).abs() < 1e-8 * scale.powi(3));
    assert_eq!(euler(&imported), euler(body));
}
#[test]
fn plain_box_reordered_caps_pose_and_micro_bore_are_actual_closed_geometry() {
    for scale in [1., 1e-4] {
        let mut source = block(scale);
        source.shell.faces.rotate_left(3);
        let before = format!("{source:?}");
        let center = Point3::new(-4. * scale, 0., 13. * scale);
        assert!(bore_normal_arc_line_prism(&source, center, scale, policy(scale)).is_err());
        assert!(split_normal_arc_line_prism_by_plane_components(
            &source,
            &plane(0.),
            policy(scale)
        )
        .is_err());
        let r = bore_normal_prism(&source, center, scale, Vec3::new(0., 0., 1.), policy(scale))
            .unwrap();
        retained_lines(&source, r.kept(), scale);
        check(r.kept(), (1600. - 5. * PI) * scale.powi(3), 1, scale);
        check(r.removed(), 5. * PI * scale.powi(3), 0, scale);
        assert!(
            (r.direct_removed_volume() - 5. * PI * scale.powi(3)).abs() < 1e-10 * scale.powi(3)
        );
        assert_eq!(format!("{source:?}"), before);
    }
    let scale = 1.;
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = block(scale)
        .transformed(transform, Tolerance::new(1e-6).unwrap())
        .unwrap();
    let r = bore_normal_prism(
        &source,
        transform.point(Point3::new(-4., 0., 13.)),
        1.,
        transform.vector(Vec3::new(0., 0., 1.)),
        policy(scale),
    )
    .unwrap();
    check(r.kept(), 1600. - 5. * PI, 1, scale);
    let cut = split_normal_prism_by_plane_components(
        r.kept(),
        &plane(0.).transformed(transform).unwrap(),
        transform.vector(Vec3::new(0., 0., 1.)),
        policy(scale),
    )
    .unwrap();
    sections_are_actual(&cut);
    assert_eq!((cut.negative().len(), cut.positive().len()), (1, 1));
    check(&cut.negative()[0], 800. - 5. * PI, 1, 1.);
    check(&cut.positive()[0], 800., 0, 1.);
}
fn sections_are_actual(r: &NormalArcLinePrismPlaneSplitComponents) {
    let Surface::Plane { u, v, .. } = *r.plane() else {
        unreachable!()
    };
    let normal = u.cross(v).normalized().unwrap();
    for section in r.sections() {
        assert_eq!(section.rings.len(), 1);
        assert_eq!(section.rings[0].len(), 4);
        for (bodies, side) in [(r.negative(), -1.), (r.positive(), 1.)] {
            let faces: Vec<_> = bodies
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
            assert_eq!(faces.len(), 1);
            let Surface::Plane { u, v, .. } = faces[0].1.surface else {
                panic!("actual cut plane")
            };
            assert!(
                (u.cross(v) * f64::from(faces[0].1.orientation)).dot(normal) * side < -1. + 1e-12
            );
        }
    }
}
fn polygon(outer: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>>) -> Solid {
    extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes,
        },
        Vec3::new(0., 0., 5.),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap()
}
#[test]
fn plain_concave_multi_components_hole_ownership_and_cut_bore_cut_chain() {
    let s = polygon(
        vec![
            [-6., -4.],
            [6., -4.],
            [6., 6.],
            [2., 6.],
            [2., 0.],
            [-2., 0.],
            [-2., 6.],
            [-6., 6.],
        ],
        vec![],
    );
    let p = Surface::Plane {
        origin: Point3::new(0., 2., 0.),
        u: Vec3::new(-1., 0., 0.),
        v: Vec3::new(0., 0., 1.),
    };
    let r =
        split_normal_prism_by_plane_components(&s, &p, Vec3::new(0., 0., 1.), policy(1.)).unwrap();
    assert_eq!(
        (r.negative().len(), r.positive().len(), r.sections().len()),
        (1, 2, 2)
    );
    sections_are_actual(&r);
    check(&r.negative()[0], 320., 0, 1.);
    for child in r.positive() {
        check(child, 80., 0, 1.);
    }
    let s = polygon(
        vec![[-10., -8.], [10., -8.], [10., 8.], [-10., 8.]],
        vec![vec![[-6., -1.], [-4., -1.], [-4., 1.], [-6., 1.]]],
    );
    let r =
        split_normal_prism_by_plane_components(&s, &plane(0.), Vec3::new(0., 0., 1.), policy(1.))
            .unwrap();
    sections_are_actual(&r);
    check(&r.negative()[0], 780., 1, 1.);
    check(&r.positive()[0], 800., 0, 1.);
    let bore = bore_normal_prism(
        &r.positive()[0],
        Point3::new(6., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    check(bore.kept(), 800. - 5. * PI, 1, 1.);
    let again = split_normal_prism_by_plane_components(
        bore.kept(),
        &plane(3.),
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    sections_are_actual(&again);
    check(&again.negative()[0], 240., 0, 1.);
    check(&again.positive()[0], 560. - 5. * PI, 1, 1.);
    let angle: f64 = 0.2;
    let p = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(-angle.sin(), angle.cos(), 0.),
        v: Vec3::new(0., 0., 1.),
    };
    let r =
        split_normal_prism_by_plane_components(&block(1.), &p, Vec3::new(0., 0., 1.), policy(1.))
            .unwrap();
    sections_are_actual(&r);
    check(&r.negative()[0], 800., 0, 1.);
    check(&r.positive()[0], 800., 0, 1.);
}
#[test]
fn invalid_plain_sources_and_excessive_policy_reject_without_mutation() {
    let source = block(1.);
    let before = format!("{source:?}");
    for (center, radius, tol) in [
        (Point3::new(10., 0., 0.), 1., policy(1.)),
        (Point3::new(0., 0., 0.), 0., policy(1.)),
        (
            Point3::new(0., 0., 0.),
            1.,
            GeometryTolerance::new(1., 1e-10, 0.).unwrap(),
        ),
    ] {
        assert!(bore_normal_prism(&source, center, radius, Vec3::new(0., 0., 1.), tol).is_err());
    }
    let skew = Surface::Plane {
        origin: Point3::new(0., 0., 0.),
        u: Vec3::new(0., 1., 0.),
        v: Vec3::new(0.1, 0., 1.),
    };
    assert!(split_normal_prism_by_plane_components(
        &source,
        &skew,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let mut malformed = source.clone();
    malformed.shell.faces[0].orientation = 0;
    assert!(bore_normal_prism(
        &malformed,
        Point3::new(0., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    assert!(split_normal_prism_by_plane_components(
        &malformed,
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    assert_eq!(format!("{source:?}"), before);
    let skew_source = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-10., -8.], [10., -8.], [10., 8.], [-10., 8.]],
            holes: vec![],
        },
        Vec3::new(1., 0., 5.),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    assert!(bore_normal_prism(
        &skew_source,
        Point3::new(0., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
}

#[test]
fn explicit_axis_selects_actual_box_family_and_rejects_invalid_hints() {
    let source = block(1.);
    for (axis, height) in [
        (Vec3::new(1., 0., 0.), 20.),
        (Vec3::new(0., 1., 0.), 16.),
        (Vec3::new(0., 0., 1.), 5.),
    ] {
        let r = bore_normal_prism(&source, Point3::new(0., 0., 2.5), 1., axis, policy(1.)).unwrap();
        check(r.kept(), 1600. - PI * height, 1, 1.);
        check(r.removed(), PI * height, 0, 1.);
    }
    let positive = bore_normal_prism(
        &source,
        Point3::new(0., 0., 2.5),
        1.,
        Vec3::new(1., 0., 0.),
        policy(1.),
    )
    .unwrap();
    for axis in [Vec3::new(-1e100, 0., 0.), Vec3::new(1e-100, 0., 0.)] {
        let same =
            bore_normal_prism(&source, Point3::new(0., 0., 2.5), 1., axis, policy(1.)).unwrap();
        assert_eq!(
            format!("{:?}", same.kept()),
            format!("{:?}", positive.kept())
        );
    }
    for axis in [
        Vec3::new(1., 1., 1.),
        Vec3::new(0., 0., 0.),
        Vec3::new(0.0001, 0., 1.),
        Vec3::new(f64::NAN, 0., 1.),
    ] {
        assert!(
            bore_normal_prism(&source, Point3::new(0., 0., 2.5), 1., axis, policy(1.)).is_err()
        );
        assert!(
            split_normal_prism_by_plane_components(&source, &plane(0.), axis, policy(1.)).is_err()
        );
    }
}
