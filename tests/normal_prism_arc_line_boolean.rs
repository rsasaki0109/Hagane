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

fn geometry_support(body: &Solid, source: &Solid, tool: &Solid, axis: Vec3, guard: f64) {
    for edge in &body.edges {
        if let Curve::Line { a: start, b: end } = edge.curve {
            if (end - start).cross(axis).norm() <= guard {
                continue;
            }
        }
        for k in 0..=12 {
            let r = edge.curve.range();
            let p = edge
                .curve
                .try_evaluate(r[0] + (r[1] - r[0]) * k as f64 / 12.)
                .unwrap();
            let supported =
                source
                    .edges
                    .iter()
                    .chain(&tool.edges)
                    .any(|original| match original.curve {
                        Curve::Line { a: start, b: end } => {
                            let d = end - start;
                            let q = (p - start).dot(d) / d.dot(d);
                            (p - (start + d * q)).norm() <= guard
                                && (-1e-10..=1. + 1e-10).contains(&q)
                        }
                        Curve::Arc {
                            frame,
                            radius,
                            sweep,
                        } => {
                            let d = frame.local_point(p);
                            let t = d.y.atan2(d.x);
                            let angular_guard = guard / radius;
                            d.z.abs() <= guard
                                && (d.x.hypot(d.y) - radius).abs() <= guard
                                && t >= sweep.min(0.) - angular_guard
                                && t <= sweep.max(0.) + angular_guard
                        }
                        _ => false,
                    });
            assert!(
                supported,
                "output edge point has no actual source/tool support: {p:?}"
            );
        }
    }
}

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn quarters(center: [f64; 2], radius: f64, phase: f64) -> Vec<PlanarSegment> {
    (0..4)
        .map(|i| PlanarSegment::Arc {
            center,
            radius,
            start_angle: phase + i as f64 * PI / 2.,
            sweep: PI / 2.,
        })
        .collect()
}
fn disk(
    center: [f64; 2],
    radius: f64,
    phase: f64,
    scale: f64,
    holes: Vec<Vec<PlanarSegment>>,
) -> Solid {
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: quarters(
                [center[0] * scale, center[1] * scale],
                radius * scale,
                phase,
            ),
            holes,
        },
        20. * scale,
        policy(scale).absolute(),
    )
    .unwrap()
}
fn rounded() -> Solid {
    let p = rounded_rectangle_profile(Point3::new(0., 0., 0.), 80., 60., 8., policy(1.).absolute())
        .unwrap();
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: p.origin,
            outer: p.segments,
            holes: vec![],
        },
        20.,
        policy(1.).absolute(),
    )
    .unwrap()
}
fn polygon(points: Vec<[f64; 2]>) -> Solid {
    extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: points,
            holes: vec![],
        },
        Vec3::new(0., 0., 20.),
        policy(1.).absolute(),
    )
    .unwrap()
}
fn volume(bodies: &[Solid]) -> f64 {
    bodies.iter().map(|b| b.volume().unwrap()).sum()
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 2e-10 * expected.abs().max(f64::MIN_POSITIVE),
        "{actual} vs {expected}"
    );
}
// Independent intersection of two complete Euclidean disks, from two sectors
// minus the two triangles; no kernel area or arrangement routine is used.
fn lens(r: f64, s: f64, d: f64) -> f64 {
    r * r * ((d * d + r * r - s * s) / (2. * d * r)).acos()
        + s * s * ((d * d + s * s - r * r) / (2. * d * s)).acos()
        - 0.5 * ((-d + r + s) * (d + r - s) * (d - r + s) * (d + r + s)).sqrt()
}
fn circle_primitive(r: f64, x: f64) -> f64 {
    0.5 * (x * (r * r - x * x).max(0.).sqrt() + r * r * (x / r).asin())
}
fn verify(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    scale: f64,
    common: f64,
    counts: [usize; 3],
) -> NormalPrismArcLineBoolean {
    let before = (format!("{source:?}"), format!("{tool:?}"));
    let result = boolean_normal_arc_line_prisms(source, tool, axis, policy(scale)).unwrap();
    assert_eq!(before, (format!("{source:?}"), format!("{tool:?}")));
    assert_eq!(
        [
            result.difference().len(),
            result.intersection().len(),
            result.union().len()
        ],
        counts
    );
    close(volume(result.intersection()), common);
    close(
        volume(result.difference()) + common,
        source.volume().unwrap(),
    );
    close(
        volume(result.union()) + common,
        source.volume().unwrap() + tool.volume().unwrap(),
    );
    let world = source
        .vertices
        .iter()
        .chain(&tool.vertices)
        .fold(50. * scale, |m, v| {
            m.max(v.point.x.abs())
                .max(v.point.y.abs())
                .max(v.point.z.abs())
        });
    for body in result
        .difference()
        .iter()
        .chain(result.intersection())
        .chain(result.union())
    {
        check_closed(body, scale, world);
        geometry_support(
            body,
            source,
            tool,
            axis.normalized().unwrap(),
            8192. * f64::EPSILON * world,
        );
        let step = export_step_bounded_analytic_mm(body, 1e-6 * scale).unwrap();
        let restored = import_step_bounded_analytic_mm(&step, policy(scale).absolute()).unwrap();
        close(restored.volume().unwrap(), body.volume().unwrap());
        assert_eq!(euler(&restored), euler(body));
        check_closed(&restored, scale, world);
    }
    result
}
#[test]
fn exact_two_disk_lens_and_operand_symmetry_match_independent_sectors() {
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let tool = disk([6., 4.], 6., 0.17, 1., vec![]);
    let common = 20. * lens(8., 6., 52_f64.sqrt());
    let axis = Vec3::new(0., 0., 1.);
    let r = verify(&source, &tool, axis, 1., common, [1, 1, 1]);
    let reverse = verify(&tool, &source, axis, 1., common, [1, 1, 1]);
    close(volume(r.union()), volume(reverse.union()));
    for point in [Point3::new(4., 2., 10.), Point3::new(6., 4., 10.)] {
        assert_eq!(
            classify_point_in_solid(&r.intersection()[0], point, policy(1.)).unwrap(),
            PointLocation::Inside
        );
    }
    for body in r
        .difference()
        .iter()
        .chain(r.intersection())
        .chain(r.union())
    {
        assert_eq!(euler(body), 2);
    }
    assert!(partition_normal_prism_by_convex_tool(&source, &tool, axis, policy(1.)).is_err());
}
#[test]
fn rounded_corner_circle_and_nonconvex_line_tools_use_the_new_actual_arrangement() {
    let source = rounded();
    let cutter = disk([38., 26.], 6., 0.17, 1., vec![]);
    let d = 52_f64.sqrt();
    let along = 40. / d;
    let height = (64. - along * along).sqrt();
    let x = 32. + along * 6. / d + height * 4. / d;
    // Complete-circle lens plus the stock's straight-side strip below y=22,
    // outside its corner circle. This strip is integrated independently.
    let bonus = -4. * (40. - x) - (circle_primitive(8., 8.) - circle_primitive(8., x - 32.))
        + (circle_primitive(6., 2.) - circle_primitive(6., x - 38.));
    verify(
        &source,
        &cutter,
        Vec3::new(0., 0., 1.),
        1.,
        20. * (lens(8., 6., d) + bonus),
        [1, 1, 1],
    );
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let nonconvex = polygon(vec![
        [-3., -2.],
        [3., -2.],
        [3., 0.],
        [0., 0.],
        [0., 3.],
        [-3., 3.],
    ]);
    let r = verify(
        &source,
        &nonconvex,
        Vec3::new(0., 0., 1.),
        1.,
        420.,
        [1, 1, 1],
    );
    assert_eq!(euler(&r.difference()[0]), 0);
    assert!(partition_normal_prism_by_convex_tool(
        &source,
        &nonconvex,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let band = polygon(vec![[-1., -12.], [1., -12.], [1., 12.], [-1., 12.]]);
    let area = 2. * (63_f64.sqrt() + 64. * (1_f64 / 8.).asin());
    let r = verify(
        &source,
        &band,
        Vec3::new(0., 0., 1.),
        1.,
        20. * area,
        [2, 1, 1],
    );
    let old =
        partition_normal_prism_by_convex_tool(&source, &band, Vec3::new(0., 0., 1.), policy(1.))
            .unwrap();
    close(volume(r.intersection()), volume(old.intersection()));
    close(volume(r.difference()), volume(old.difference()));
}
#[test]
fn containment_empty_sets_and_existing_circle_openings_have_correct_material_genus() {
    let axis = Vec3::new(0., 0., 1.);
    let outer = disk([0., 0.], 8., 0., 1., vec![]);
    let inside = disk([0., 0.], 3., 0.17, 1., vec![]);
    let r = verify(&outer, &inside, axis, 1., 180. * PI, [1, 1, 1]);
    assert_eq!(euler(&r.difference()[0]), 0);
    verify(&inside, &outer, axis, 1., 180. * PI, [0, 1, 1]);
    verify(
        &outer,
        &disk([20., 0.], 3., 0.17, 1., vec![]),
        axis,
        1.,
        0.,
        [1, 0, 2],
    );
    let holed = disk([0., 0.], 8., 0., 1., vec![quarters([0., 0.], 1., 0.07)]);
    let r = verify(&holed, &inside, axis, 1., 160. * PI, [1, 1, 1]);
    assert_eq!(euler(&r.intersection()[0]), 0);
    assert_eq!(euler(&r.union()[0]), 2);
    let holed = disk([0., 0.], 8., 0., 1., vec![quarters([2., 0.], 2., 0.07)]);
    let crossing = disk([4., 1.], 3., 0.17, 1., vec![]);
    let common = 20. * (9. * PI - lens(2., 3., 5_f64.sqrt()));
    let r = verify(&holed, &crossing, axis, 1., common, [1, 1, 1]);
    assert_eq!(euler(&r.difference()[0]), 0);
    assert_eq!(euler(&r.intersection()[0]), 2);
    assert_eq!(euler(&r.union()[0]), 0);
}
#[test]
fn lens_geometry_is_rigid_covariant_and_scaled_with_unoriented_axis_hints() {
    let axis = Vec3::new(0., 0., 1.);
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = disk([0., 0.], 8., 0., 1., vec![])
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let cutter = disk([6., 4.], 6., 0.17, 1., vec![])
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    verify(
        &source,
        &cutter,
        transform.vector(axis),
        1.,
        20. * lens(8., 6., 52_f64.sqrt()),
        [1, 1, 1],
    );
    for scale in [1e-4, 10.] {
        let source = disk([0., 0.], 8., 0., scale, vec![]);
        let cutter = disk([6., 4.], 6., 0.17, scale, vec![]);
        for magnitude in [1e-300, -1e300, 1e-320] {
            verify(
                &source,
                &cutter,
                Vec3::new(0., 0., magnitude),
                scale,
                20. * lens(8., 6., 52_f64.sqrt()) * scale.powi(3),
                [1, 1, 1],
            );
        }
    }
}
#[test]
fn contacts_periodic_tools_holes_and_nonprismatic_sources_are_immutable_rejections() {
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let axis = Vec3::new(0., 0., 1.);
    let tangent = disk(
        [14. * 0.37_f64.cos(), 14. * 0.37_f64.sin()],
        6.,
        0.17,
        1.,
        vec![],
    );
    let coincident = disk([0., 0.], 8., 0.17, 1., vec![]);
    let vertex = disk([8., 2.], 2., 0.17, 1., vec![]);
    let periodic = make_cylinder(
        CylinderSpec {
            base: Point3::new(6., 4., 0.),
            radius: 6.,
            height: 20.,
        },
        policy(1.).absolute(),
    )
    .unwrap();
    let holed = disk([0., 0.], 3., 0.17, 1., vec![quarters([0., 0.], 1., 0.07)]);
    for cutter in [&tangent, &coincident, &vertex, &periodic, &holed] {
        let before = (format!("{source:?}"), format!("{cutter:?}"));
        assert!(boolean_normal_arc_line_prisms(&source, cutter, axis, policy(1.)).is_err());
        assert_eq!(before, (format!("{source:?}"), format!("{cutter:?}")));
    }
    let cutter = disk([6., 4.], 6., 0.17, 1., vec![]);
    assert!(boolean_normal_arc_line_prisms(&periodic, &cutter, axis, policy(1.)).is_err());
    let shifted = cutter
        .transformed(
            Transform::translation(Vec3::new(0., 0., 1.)).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    let tilted = cutter
        .transformed(
            Transform::rotation(Vec3::new(1., 0., 0.), 0.01).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    for unsupported in [&shifted, &tilted] {
        let before = format!("{unsupported:?}");
        assert!(boolean_normal_arc_line_prisms(&source, unsupported, axis, policy(1.)).is_err());
        assert_eq!(before, format!("{unsupported:?}"));
    }
    let mut malformed = source.clone();
    if let Curve::Arc { radius, .. } = &mut malformed.edges[0].curve {
        *radius += 0.01;
    }
    let before = format!("{malformed:?}");
    assert!(boolean_normal_arc_line_prisms(&malformed, &cutter, axis, policy(1.)).is_err());
    assert_eq!(before, format!("{malformed:?}"));
    for axis in [
        Vec3::new(0., 0., 0.),
        Vec3::new(0.01, 0., 1.),
        Vec3::new(f64::NAN, 0., 1.),
    ] {
        assert!(boolean_normal_arc_line_prisms(&source, &cutter, axis, policy(1.)).is_err());
    }
    let blind = blind_bore_normal_prism(
        &source,
        Point3::new(0., 0., 20.),
        1.,
        3.,
        axis,
        NormalPrismBoreEntry::Positive,
        policy(1.),
    )
    .unwrap();
    assert!(boolean_normal_arc_line_prisms(blind.kept(), &cutter, axis, policy(1.)).is_err());
}
