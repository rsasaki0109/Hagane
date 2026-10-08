use hagane::*;
fn shape(scale: f64, sign: f64, t: Tolerance) -> Solid {
    extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: rounded_rectangle_profile(
                Point3::new(0., 0., 0.),
                12. * scale,
                10. * scale,
                scale,
                t,
            )
            .unwrap()
            .segments,
            holes: vec![
                rounded_rectangle_profile(
                    Point3::new(0., 0., 0.),
                    4. * scale,
                    3. * scale,
                    0.5 * scale,
                    t,
                )
                .unwrap()
                .segments,
            ],
        },
        Vec3::new(3. * scale, -2. * scale, sign * 4. * scale),
        t,
    )
    .unwrap()
}
fn closed_mesh(s: &Solid, error: f64, t: Tolerance) {
    let mesh = s.tessellate(error, t).unwrap();
    let scale = (s.bounds().max - s.bounds().min).norm();
    let key = |p: Point3| {
        [
            (p.x / scale * 1e9).round() as i64,
            (p.y / scale * 1e9).round() as i64,
            (p.z / scale * 1e9).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for (i, tri) in mesh.triangles.iter().enumerate() {
        let p = tri.map(|j| mesh.positions[j]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for j in 0..3 {
            let a = key(p[j]);
            let b = key(p[(j + 1) % 3]);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let count = uses.entry(edge).or_insert((0, 0));
            count.0 += 1;
            count.1 += sign;
            let face = &s.shell.faces[mesh.face_ids[i]];
            if let Surface::ExtrudedCircle { frame, drift, .. } = face.surface {
                let midpoint = (p[j] + p[(j + 1) % 3]) * 0.5;
                let local = frame.local_point(midpoint);
                let u = (local.y - drift[1] * local.z)
                    .atan2(local.x - drift[0] * local.z)
                    .rem_euclid(std::f64::consts::TAU);
                let on_surface = face.surface.evaluate(u, local.z);
                assert!((midpoint - on_surface).norm() <= error + scale * 1e-10);
            }
        }
    }
    assert!(
        uses.values().all(|&(n, s)| n == 2 && s == 0),
        "nonconforming shared mesh edges"
    );
    assert!((mesh.signed_volume() - s.volume().unwrap()).abs() < s.volume().unwrap() * 0.02);
}
#[test]
fn oblique_sections_preserve_exact_ellipses_closed_topology_and_metrics() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for sign in [-1., 1.] {
            let s = shape(scale, sign, t.absolute());
            let origin = Point3::new(1.5 * scale, -scale, 2. * sign * scale);
            let normal = Vec3::new(0.1, -0.08, sign);
            let r = subdivide_extrusion_boundary_by_plane(&s, origin, normal, t).unwrap();
            r.solid.validate(t.absolute()).unwrap();
            assert_eq!(r.split_faces.len(), 16);
            assert_eq!(r.section_edges.len(), 16);
            assert_eq!(r.solid.shell.faces.len(), 34);
            assert_eq!(r.solid.edges.len(), 80);
            assert!(
                (r.solid.volume().unwrap() - s.volume().unwrap()).abs()
                    < s.volume().unwrap() * 1e-12
            );
            assert!((r.solid.bounds().min - s.bounds().min).norm() < scale * 1e-10);
            assert!((r.solid.bounds().max - s.bounds().max).norm() < scale * 1e-10);
            let mut degree = std::collections::BTreeMap::new();
            let mut ellipses = 0;
            for &index in &r.section_edges {
                let edge = &r.solid.edges[index];
                for v in edge.vertices {
                    *degree.entry(v).or_insert(0) += 1;
                }
                let range = edge.curve.range();
                for i in 0..=24 {
                    let point = edge.curve.evaluate(range[1] * i as f64 / 24.);
                    assert!(normal.normalized().unwrap().dot(point - origin).abs() < scale * 1e-10);
                }
                if let Curve::EllipseArc { cosine, sine, .. } = edge.curve {
                    ellipses += 1;
                    assert!(
                        cosine
                            .normalized()
                            .unwrap()
                            .cross(sine.normalized().unwrap())
                            .norm()
                            > 0.9
                    );
                }
                let uses: Vec<_> = r
                    .solid
                    .shell
                    .faces
                    .iter()
                    .flat_map(|f| &f.wires)
                    .flat_map(|w| &w.coedges)
                    .filter(|c| c.edge == index)
                    .collect();
                assert_eq!(uses.len(), 2);
                assert_ne!(uses[0].forward, uses[1].forward);
            }
            assert_eq!(ellipses, 8);
            assert!(degree.values().all(|&n| n == 2));
            closed_mesh(&r.solid, scale * 0.01, t.absolute());
            // Existing rectangular query APIs must refuse the new trim domain.
            assert!(matches!(
                classify_point_in_solid(&r.solid, Point3::new(100. * scale, 0., 0.), t),
                Err(Error::Unsupported(_))
            ));
        }
    }
}
#[test]
fn placed_and_normal_extrusions_support_oblique_or_level_planes() {
    let t = GeometryTolerance::default();
    let tr = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    for direction in [Vec3::new(0., 0., 4.), Vec3::new(3., -2., 4.)] {
        let s = extrude_arc_line(
            &rounded_rectangle_profile(Point3::new(0., 0., 0.), 12., 10., 1., t.absolute())
                .unwrap(),
            4.,
            t.absolute(),
        )
        .unwrap();
        let s = if direction.x != 0. {
            shape(1., 1., t.absolute())
        } else {
            s
        };
        for normal in [Vec3::new(0., 0., 1.), Vec3::new(0.1, -0.08, 1.)] {
            let origin = Point3::new(direction.x / 2., direction.y / 2., 2.);
            let placed = s.transformed(tr, t.absolute()).unwrap();
            let result = subdivide_extrusion_boundary_by_plane(
                &placed,
                tr.point(origin),
                tr.vector(normal),
                t,
            )
            .unwrap();
            assert!((result.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
            closed_mesh(&result.solid, 0.01, t.absolute());
            for &edge in &result.section_edges {
                for u in [
                    0.,
                    result.solid.edges[edge].curve.range()[1] * 0.37,
                    result.solid.edges[edge].curve.range()[1],
                ] {
                    assert!(
                        tr.vector(normal)
                            .dot(result.solid.edges[edge].curve.evaluate(u) - tr.point(origin))
                            .abs()
                            < 1e-10
                    );
                }
            }
        }
    }
}
#[test]
fn contacts_partial_rim_crossings_invalid_and_unimplemented_queries_fail_atomically() {
    let t = GeometryTolerance::default();
    let s = shape(1., 1., t.absolute());
    let original = format!("{s:?}");
    for (origin, normal) in [
        (Point3::new(0., 0., 0.), Vec3::new(0., 0., 1.)),
        (Point3::new(0., 0., 1e-9), Vec3::new(0., 0., 1.)),
        (Point3::new(1.5, -1., 2.), Vec3::new(3., 0., 1.)),
        (Point3::new(1.5, -1., 2.), Vec3::new(0., 2., 1.)),
        (Point3::new(f64::NAN, 0., 2.), Vec3::new(0., 0., 1.)),
        (Point3::new(0., 0., 2.), Vec3::new(0., 0., 0.)),
    ] {
        assert!(subdivide_extrusion_boundary_by_plane(&s, origin, normal, t).is_err());
    }
    assert_eq!(format!("{s:?}"), original);
    let result = subdivide_extrusion_boundary_by_plane(
        &s,
        Point3::new(1.5, -1., 2.),
        Vec3::new(0.1, 0., 1.),
        t,
    )
    .unwrap();
    assert!(subdivide_extrusion_boundary_by_plane(
        &result.solid,
        Point3::new(1.5, -1., 2.),
        Vec3::new(0., 0., 1.),
        t
    )
    .is_err());
    let mut bad = result.solid.clone();
    let fi = bad
        .shell
        .faces
        .iter()
        .position(|f| {
            f.wires
                .iter()
                .flat_map(|w| &w.coedges)
                .any(|c| matches!(c.pcurve, PCurve::HeightGraph { .. }))
        })
        .unwrap();
    for coedge in &mut bad.shell.faces[fi].wires[0].coedges {
        if let PCurve::HeightGraph { offset, .. } = &mut coedge.pcurve {
            *offset += 1.;
            break;
        }
    }
    assert!(bad.validate(t.absolute()).is_err());
    assert!(bad.tessellate(0.01, t.absolute()).is_err());
    let periodic = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(subdivide_extrusion_boundary_by_plane(
        &periodic,
        Point3::new(0., 0., 2.),
        Vec3::new(0.1, 0., 1.),
        t
    )
    .is_err());
    assert!(oblique_boundary_demo(3., 0.).is_err());
    assert!(oblique_boundary_demo(f64::NAN, 0.).is_err());
    assert!(oblique_boundary_demo(0.1, f64::INFINITY).is_err());
}
#[test]
fn ellipse_chords_respect_error_and_invalid_curve_transform_fails() {
    let t = Tolerance::default();
    let solid = oblique_boundary_demo(0.13, 0.7).unwrap();
    let error = 0.003;
    let mesh = solid.tessellate(error, t).unwrap();
    for (index, edge) in solid.edges.iter().enumerate() {
        let Curve::EllipseArc { sweep, .. } = edge.curve else {
            continue;
        };
        let fi = solid
            .shell
            .faces
            .iter()
            .position(|f| {
                f.wires
                    .iter()
                    .flat_map(|w| &w.coedges)
                    .any(|c| c.edge == index)
            })
            .unwrap();
        let count = mesh.face_ids.iter().filter(|&&i| i == fi).count() / 2;
        for i in 0..count {
            let a = sweep * i as f64 / count as f64;
            let b = sweep * (i + 1) as f64 / count as f64;
            for fraction in [0.25, 0.5, 0.75] {
                let chord =
                    edge.curve.evaluate(a) * (1. - fraction) + edge.curve.evaluate(b) * fraction;
                assert!(
                    (chord - edge.curve.evaluate(a + (b - a) * fraction)).norm() <= error + 1e-10
                );
            }
        }
    }
    let ellipse = Curve::EllipseArc {
        center: Point3::new(f64::MAX, 0., 0.),
        cosine: Vec3::new(2., 0., 0.),
        sine: Vec3::new(0., 1., 0.),
        sweep: 1.,
    };
    assert!(ellipse
        .transformed(Transform::translation(Vec3::new(f64::MAX, 0., 0.)).unwrap())
        .is_err());
}
#[test]
fn scaled_normals_and_unresolvable_world_coordinates() {
    let t = GeometryTolerance::default();
    let s = shape(1., 1., t.absolute());
    let origin = Point3::new(1.5, -1., 2.);
    let normal = Vec3::new(0.1, -0.08, 1.);
    for scale in [1e-250, 1e250] {
        let result = subdivide_extrusion_boundary_by_plane(&s, origin, normal * scale, t).unwrap();
        assert_eq!(result.section_edges.len(), 16);
    }
    let offset = Vec3::new(1e12, 1e12, 1e12);
    let huge = s
        .transformed(
            Transform::translation(offset).unwrap(),
            Tolerance::new(0.01).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        subdivide_extrusion_boundary_by_plane(
            &huge,
            origin + offset,
            normal,
            GeometryTolerance::new(0.001, 1e-10, 0.).unwrap()
        ),
        Err(Error::Unsupported(_))
    ));
}
