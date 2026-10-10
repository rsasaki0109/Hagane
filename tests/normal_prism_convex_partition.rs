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
fn stock(scale: f64, holes: bool) -> Solid {
    let p = rounded_rectangle_profile(
        Point3::new(0., 0., 0.),
        20. * scale,
        16. * scale,
        2. * scale,
        policy(scale).absolute(),
    )
    .unwrap();
    let holes = if holes {
        vec![(0..4)
            .map(|i| PlanarSegment::Arc {
                center: [0., 0.],
                radius: 0.75 * scale,
                start_angle: i as f64 * PI / 2.,
                sweep: PI / 2.,
            })
            .collect()]
    } else {
        vec![]
    };
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: p.origin,
            outer: p.segments,
            holes,
        },
        5. * scale,
        policy(scale).absolute(),
    )
    .unwrap()
}
fn tool(bounds: [f64; 4], scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(bounds[0] * scale, bounds[1] * scale, 0.),
            size: Vec3::new(
                (bounds[2] - bounds[0]) * scale,
                (bounds[3] - bounds[1]) * scale,
                5. * scale,
            ),
        },
        policy(scale).absolute(),
    )
    .unwrap()
}
fn total(bodies: &[Solid]) -> f64 {
    bodies.iter().map(|s| s.volume().unwrap()).sum()
}
fn assert_volume(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(f64::MIN_POSITIVE),
        "{actual} vs {expected}"
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
fn verify(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    scale: f64,
    expected: f64,
    difference_count: usize,
    intersection_count: usize,
) -> NormalPrismConvexPartition {
    let before = (format!("{source:?}"), format!("{tool:?}"));
    let result = partition_normal_prism_by_convex_tool(source, tool, axis, policy(scale)).unwrap();
    assert_eq!(before, (format!("{source:?}"), format!("{tool:?}")));
    assert_eq!(result.difference().len(), difference_count);
    assert_eq!(result.intersection().len(), intersection_count);
    assert_volume(total(result.intersection()), expected);
    assert_volume(
        total(result.difference()) + total(result.intersection()),
        source.volume().unwrap(),
    );
    let world = source
        .vertices
        .iter()
        .chain(&tool.vertices)
        .fold(30. * scale, |m, v| {
            m.max(v.point.x.abs())
                .max(v.point.y.abs())
                .max(v.point.z.abs())
        });
    for body in result.difference().iter().chain(result.intersection()) {
        check_closed(body, scale, world);
        geometry_support(
            body,
            source,
            tool,
            axis.normalized().unwrap(),
            8192. * f64::EPSILON * world,
        );
        let step = export_step_bounded_analytic_mm(body, 1e-6 * scale).unwrap();
        let imported = import_step_bounded_analytic_mm(&step, policy(scale).absolute()).unwrap();
        assert_volume(imported.volume().unwrap(), body.volume().unwrap());
        assert_eq!(euler(&imported), euler(body));
        check_closed(&imported, scale, world);
    }
    result
}
#[test]
fn cylinder_crossing_side_and_corner_partitions_have_independent_circle_segment_volumes() {
    let source = stock(1., false);
    let axis = Vec3::new(0., 0., 1.);
    let side = verify(
        &source,
        &tool([9., -12., 12., 12.], 1.),
        axis,
        1.,
        5. * (12. + 4. * PI / 3. - 3_f64.sqrt()),
        1,
        1,
    );
    assert!(side.intersection()[0]
        .edges
        .iter()
        .any(|e| matches!(e.curve, Curve::Arc { .. })));
    let corner = verify(
        &source,
        &tool([9., 7., 12., 12.], 1.),
        axis,
        1.,
        5. * (1. + PI / 3. - 3_f64.sqrt()),
        1,
        1,
    );
    assert_eq!(euler(&corner.difference()[0]), 2);
    for (bodies, point) in [
        (side.intersection(), Point3::new(9.5, 0., 2.5)),
        (side.difference(), Point3::new(0., 0., 2.5)),
    ] {
        assert_eq!(
            classify_point_in_solid(&bodies[0], point, policy(1.)).unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn band_components_and_containment_empty_results_preserve_actual_material() {
    let source = stock(1., false);
    let axis = Vec3::new(0., 0., 1.);
    let band = verify(
        &source,
        &tool([-1., -12., 1., 12.], 1.),
        axis,
        1.,
        160.,
        2,
        1,
    );
    for body in band.difference() {
        assert_eq!(euler(body), 2);
    }
    let all_line = tool([-10., -8., 10., 8.], 1.);
    let corner_tool = tool([9., 7., 12., 12.], 1.);
    let forward = verify(&all_line, &corner_tool, axis, 1., 5., 1, 1);
    let reverse = verify(&corner_tool, &all_line, axis, 1., 5., 1, 1);
    assert_volume(total(forward.intersection()), total(reverse.intersection()));
    let inner = verify(&source, &tool([-1., -1., 1., 1.], 1.), axis, 1., 20., 1, 1);
    assert_eq!(euler(&inner.difference()[0]), 0);
    let outside = verify(&source, &tool([12., -1., 14., 1.], 1.), axis, 1., 0., 1, 0);
    let retained = &outside.difference()[0];
    assert_eq!(retained.vertices.len(), source.vertices.len());
    let guard = 8192. * f64::EPSILON * 30.;
    for vertex in &source.vertices {
        assert_eq!(
            retained
                .vertices
                .iter()
                .filter(|v| (v.point - vertex.point).norm() <= guard)
                .count(),
            1
        );
    }
    geometry_support(
        &source,
        retained,
        &tool([12., -1., 14., 1.], 1.),
        axis,
        guard,
    );
    verify(
        &source,
        &tool([-12., -10., 12., 10.], 1.),
        axis,
        1.,
        source.volume().unwrap(),
        0,
        1,
    );
}
#[test]
fn crossed_existing_opening_and_posed_scaled_tools_keep_exact_geometry() {
    let source = stock(1., true);
    let axis = Vec3::new(0., 0., 1.);
    let a = 0.25_f64;
    let r = 0.75_f64;
    let removed_hole = r * r * (a / r).acos() - a * (r * r - a * a).sqrt();
    let right = verify(
        &source,
        &tool([a, -12., 12., 12.], 1.),
        axis,
        1.,
        ((304. + 4. * PI) / 2. - 16. * a - removed_hole) * 5.,
        1,
        1,
    );
    assert_eq!(euler(&right.difference()[0]), 2);
    assert_eq!(euler(&right.intersection()[0]), 2);
    let rotated = Transform::translation(Vec3::new(0.25, -0.3, 0.))
        .unwrap()
        .compose(Transform::rotation(axis, 0.23).unwrap())
        .unwrap();
    let rotated_tool = tool([-1., -12., 1., 12.], 1.)
        .transformed(rotated, policy(1.).absolute())
        .unwrap();
    verify(
        &stock(1., false),
        &rotated_tool,
        axis,
        1.,
        160. / 0.23_f64.cos(),
        2,
        1,
    );
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = stock(1., false)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let cutter = tool([-1., -12., 1., 12.], 1.)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    verify(&source, &cutter, transform.vector(axis), 1., 160., 2, 1);
    for scale in [1e-4, 10.] {
        let source = stock(scale, false);
        let cutter = tool([9., -12., 12., 12.], scale);
        for magnitude in [1e-300, -1e300, 1e-320] {
            verify(
                &source,
                &cutter,
                Vec3::new(0., 0., magnitude),
                scale,
                5. * (12. + 4. * PI / 3. - 3_f64.sqrt()) * scale.powi(3),
                1,
                1,
            );
        }
    }
}
#[test]
fn unsupported_contacts_frames_and_nonconvex_tools_do_not_mutate_inputs() {
    let source = stock(1., false);
    let axis = Vec3::new(0., 0., 1.);
    let curved = stock(1., false);
    let nonconvex = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-1., -1.], [2., -1.], [2., 1.], [0., 0.], [-1., 1.]],
            holes: vec![],
        },
        Vec3::new(0., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    let mismatched = make_box(
        BoxSpec {
            min: Point3::new(-1., -12., 0.),
            size: Vec3::new(2., 24., 4.),
        },
        policy(1.).absolute(),
    )
    .unwrap();
    let holed_tool = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            holes: vec![vec![
                [-0.25, -0.25],
                [0.25, -0.25],
                [0.25, 0.25],
                [-0.25, 0.25],
            ]],
        },
        Vec3::new(0., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    let skewed_tool = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-1., -12.], [1., -12.], [1., 12.], [-1., 12.]],
            holes: vec![],
        },
        Vec3::new(0.1, 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    let tangent = tool([10., -12., 12., 12.], 1.);
    let vertex = tool([8., 7., 12., 12.], 1.);
    for cutter in [
        &curved,
        &nonconvex,
        &mismatched,
        &tangent,
        &vertex,
        &holed_tool,
        &skewed_tool,
    ] {
        let before = (format!("{source:?}"), format!("{cutter:?}"));
        assert!(partition_normal_prism_by_convex_tool(&source, cutter, axis, policy(1.)).is_err());
        assert_eq!(before, (format!("{source:?}"), format!("{cutter:?}")));
    }
    let cutter = tool([-1., -12., 1., 12.], 1.);
    for axis in [
        Vec3::new(0., 0., 0.),
        Vec3::new(0.01, 0., 1.),
        Vec3::new(f64::NAN, 0., 1.),
    ] {
        assert!(partition_normal_prism_by_convex_tool(&source, &cutter, axis, policy(1.)).is_err());
    }
    let mut bad_curve = source.clone();
    let index = bad_curve
        .edges
        .iter()
        .position(|e| matches!(e.curve, Curve::Arc { .. }))
        .unwrap();
    if let Curve::Arc { radius, .. } = &mut bad_curve.edges[index].curve {
        *radius += 0.01;
    }
    let mut bad_uv = source.clone();
    if let PCurve::Affine { origin, .. } = &mut bad_uv.shell.faces[0].wires[0].coedges[0].pcurve {
        origin[0] += 0.01;
    } else {
        panic!("expected first straight cap pcurve");
    }
    for malformed in [&bad_curve, &bad_uv] {
        let before = format!("{malformed:?}");
        assert!(
            partition_normal_prism_by_convex_tool(malformed, &cutter, axis, policy(1.)).is_err()
        );
        assert_eq!(before, format!("{malformed:?}"));
    }
    assert!(partition_normal_prism_by_convex_tool(
        &source,
        &cutter,
        axis,
        GeometryTolerance::new(1., 1e-10, 0.).unwrap()
    )
    .is_err());
    let blind = blind_bore_normal_prism(
        &source,
        Point3::new(0., 0., 5.),
        1.,
        2.,
        axis,
        NormalPrismBoreEntry::Positive,
        policy(1.),
    )
    .unwrap();
    assert!(
        partition_normal_prism_by_convex_tool(blind.kept(), &cutter, axis, policy(1.)).is_err()
    );
}
