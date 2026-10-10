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
fn verify(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    scale: f64,
    common: f64,
    counts: [usize; 3],
) -> NormalPrismRegionBoolean {
    let before = (format!("{source:?}"), format!("{tool:?}"));
    let result = boolean_normal_prism_regions(source, tool, axis, policy(scale)).unwrap();
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

fn annulus(center: [f64; 2], outer: f64, inner: f64, scale: f64) -> Solid {
    disk(
        center,
        outer,
        0.17,
        scale,
        vec![quarters(
            [center[0] * scale, center[1] * scale],
            inner * scale,
            0.09,
        )],
    )
}
fn square_with_hole(radius: f64) -> Solid {
    let points = [[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]];
    let outer = (0..4)
        .map(|i| PlanarSegment::Line {
            a: points[i],
            b: points[(i + 1) % 4],
        })
        .collect();
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: vec![quarters([0., 0.], radius, 0.07)],
        },
        20.,
        policy(1.).absolute(),
    )
    .unwrap()
}
#[test]
fn contained_and_enclosing_annuli_partition_islands_and_repair_union_holes() {
    let source = rounded();
    let axis = Vec3::new(0., 0., 1.);
    let cutter = annulus([0., 0.], 8., 3., 1.);
    let r = verify(&source, &cutter, axis, 1., 1100. * PI, [2, 1, 1]);
    let mut parts: Vec<_> = r
        .difference()
        .iter()
        .map(|s| (euler(s), s.volume().unwrap()))
        .collect();
    parts.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(parts[0].0, 2);
    close(parts[0].1, 180. * PI);
    assert_eq!(parts[1].0, 0);
    close(parts[1].1, (4800. - (4. - PI) * 64. - 64. * PI) * 20.);
    assert_eq!(euler(&r.intersection()[0]), 0);
    assert_eq!(euler(&r.union()[0]), 2);
    assert!(boolean_normal_arc_line_prisms(&source, &cutter, axis, policy(1.)).is_err());
    let enclosing = annulus([0., 0.], 60., 6., 1.);
    let common = (4800. - (4. - PI) * 64. - 36. * PI) * 20.;
    let r = verify(&source, &enclosing, axis, 1., common, [1, 1, 1]);
    close(volume(r.difference()), 720. * PI);
    close(volume(r.union()), 72000. * PI);
    assert_eq!(euler(&r.intersection()[0]), 0);
    assert_eq!(euler(&r.union()[0]), 2);
}
#[test]
fn proper_disk_annulus_crossings_match_difference_of_complete_disk_lenses() {
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let cutter = annulus([6., 4.], 6., 2., 1.);
    let common = 20. * (lens(8., 6., 52_f64.sqrt()) - lens(8., 2., 52_f64.sqrt()));
    let r = verify(
        &source,
        &cutter,
        Vec3::new(0., 0., 1.),
        1.,
        common,
        [2, 1, 1],
    );
    for piece in r.difference() {
        assert_eq!(euler(piece), 2);
    }
    assert_eq!(euler(&r.intersection()[0]), 2);
    assert_eq!(euler(&r.union()[0]), 0);
    let source = disk([0., 0.], 8., 0., 1., vec![quarters([0., 0.], 1., 0.07)]);
    let cutter = annulus([0., 0.], 6., 2., 1.);
    let r = verify(
        &source,
        &cutter,
        Vec3::new(0., 0., 1.),
        1.,
        640. * PI,
        [2, 1, 1],
    );
    assert!(r.difference().iter().all(|s| euler(s) == 0));
    assert_eq!(euler(&r.intersection()[0]), 0);
    assert_eq!(euler(&r.union()[0]), 0);
    close(volume(r.union()), 1260. * PI);
}
#[test]
fn rectangular_frame_tools_and_annular_islands_get_the_correct_hole_parent() {
    let source = polygon(vec![[-10., -8.], [10., -8.], [10., 8.], [-10., 8.]]);
    let frame = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-5., -4.], [5., -4.], [5., 4.], [-5., 4.]],
            holes: vec![vec![[-2., -1.], [2., -1.], [2., 1.], [-2., 1.]]],
        },
        Vec3::new(0., 0., 20.),
        policy(1.).absolute(),
    )
    .unwrap();
    let r = verify(&source, &frame, Vec3::new(0., 0., 1.), 1., 1440., [2, 1, 1]);
    assert_eq!(euler(&r.intersection()[0]), 0);
    close(volume(r.union()), 6400.);
    let source = square_with_hole(6.);
    let cutter = annulus([0., 0.], 4., 2., 1.);
    let r = verify(&source, &cutter, Vec3::new(0., 0., 1.), 1., 0., [1, 0, 2]);
    assert!(r.union().iter().all(|s| euler(s) == 0));
    let mut volumes: Vec<_> = r.union().iter().map(|s| s.volume().unwrap()).collect();
    volumes.sort_by(f64::total_cmp);
    close(volumes[0], 240. * PI);
    close(volumes[1], 8000. - 720. * PI);
    for (x, hits) in [(0., 0), (3., 1), (5., 0), (8., 1)] {
        assert_eq!(
            r.union()
                .iter()
                .filter(
                    |s| classify_point_in_solid(s, Point3::new(x, 0., 10.), policy(1.)).unwrap()
                        == PointLocation::Inside
                )
                .count(),
            hits
        );
    }
    assert!(
        boolean_normal_arc_line_prisms(&source, &cutter, Vec3::new(0., 0., 1.), policy(1.))
            .is_err()
    );
}
#[test]
fn nested_regions_are_rigid_covariant_scaled_and_support_empty_common_components() {
    let axis = Vec3::new(0., 0., 1.);
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let separated = annulus([20., 0.], 3., 1., 1.);
    verify(&source, &separated, axis, 1., 0., [1, 0, 2]);
    let center_island = disk([0., 0.], 1., 0., 1., vec![]);
    let ring = annulus([0., 0.], 4., 2., 1.);
    let r = verify(&center_island, &ring, axis, 1., 0., [1, 0, 2]);
    close(volume(r.union()), 260. * PI);
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = source
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let cutter = annulus([6., 4.], 6., 2., 1.)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    verify(
        &source,
        &cutter,
        transform.vector(axis),
        1.,
        20. * (lens(8., 6., 52_f64.sqrt()) - lens(8., 2., 52_f64.sqrt())),
        [2, 1, 1],
    );
    for scale in [1e-4, 10.] {
        let source = disk([0., 0.], 8., 0., scale, vec![]);
        let cutter = annulus([6., 4.], 6., 2., scale);
        verify(
            &source,
            &cutter,
            Vec3::new(0., 0., -1e-300),
            scale,
            20. * (lens(8., 6., 52_f64.sqrt()) - lens(8., 2., 52_f64.sqrt())) * scale.powi(3),
            [2, 1, 1],
        );
    }
}
#[test]
fn contacts_corrupted_holes_and_mismatched_prisms_reject_without_mutation() {
    let source = disk([0., 0.], 8., 0., 1., vec![]);
    let axis = Vec3::new(0., 0., 1.);
    let coincident = annulus([0., 0.], 12., 8., 1.);
    let unresolved = annulus([0., 0.], 10., 8. - 1e-7, 1.);
    let tangent = annulus([6. * 0.37_f64.cos(), 6. * 0.37_f64.sin()], 12., 2., 1.);
    let valid = annulus([6., 4.], 6., 2., 1.);
    let shifted = valid
        .transformed(
            Transform::translation(Vec3::new(0., 0., 1.)).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    let skewed = valid
        .transformed(
            Transform::rotation(Vec3::new(1., 0., 0.), 0.01).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    let mut malformed = valid.clone();
    if let PCurve::Arc { radius, .. } = &mut malformed.shell.faces[0].wires[1].coedges[0].pcurve {
        *radius += 0.01;
    } else {
        panic!("expected actual inner cap arc");
    }
    for cutter in [
        &coincident,
        &unresolved,
        &tangent,
        &shifted,
        &skewed,
        &malformed,
    ] {
        let before = (format!("{source:?}"), format!("{cutter:?}"));
        assert!(boolean_normal_prism_regions(&source, cutter, axis, policy(1.)).is_err());
        assert_eq!(before, (format!("{source:?}"), format!("{cutter:?}")));
    }
    assert!(boolean_normal_arc_line_prisms(&source, &valid, axis, policy(1.)).is_err());
    let blind = blind_bore_normal_prism(
        &source,
        Point3::new(0., 0., 20.),
        0.5,
        3.,
        axis,
        NormalPrismBoreEntry::Positive,
        policy(1.),
    )
    .unwrap();
    assert!(boolean_normal_prism_regions(blind.kept(), &valid, axis, policy(1.)).is_err());
}
