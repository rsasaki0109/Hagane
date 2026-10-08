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
fn wall(s: &Solid) -> usize {
    s.shell
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::ExtrudedCircle { .. }))
        .unwrap()
}
fn check_mesh(s: &Solid, error: f64, t: Tolerance) {
    let m = s.tessellate(error, t).unwrap();
    let scale = (s.bounds().max - s.bounds().min).norm();
    let key = |p: Point3| {
        [
            (p.x / scale * 1e9).round() as i64,
            (p.y / scale * 1e9).round() as i64,
            (p.z / scale * 1e9).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for (i, tri) in m.triangles.iter().enumerate() {
        let p = tri.map(|v| m.positions[v]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(m.normals[tri[0]]) > 0.);
        for j in 0..3 {
            let a = key(p[j]);
            let b = key(p[(j + 1) % 3]);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let count = uses.entry(edge).or_insert((0, 0));
            count.0 += 1;
            count.1 += sign;
            if let Surface::ExtrudedCircle {
                frame,
                radius,
                drift,
                ..
            } = s.shell.faces[m.face_ids[i]].surface
            {
                let q = frame.local_point((p[j] + p[(j + 1) % 3]) * 0.5);
                let radial = (q.x - drift[0] * q.z).hypot(q.y - drift[1] * q.z);
                assert!(radius - radial <= error + scale * 1e-10);
            }
        }
    }
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
}
#[test]
fn generator_subdivision_rebases_drift_and_preserves_shared_geometry() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for sign in [-1., 1.] {
            let s = shape(scale, sign, t.absolute());
            let fi = wall(&s);
            let angle = 0.37;
            let r = subdivide_circular_face(&s, fi, angle, t).unwrap();
            r.solid.validate(t.absolute()).unwrap();
            assert_eq!(r.solid.vertices.len(), s.vertices.len() + 2);
            assert_eq!(r.solid.edges.len(), s.edges.len() + 3);
            assert_eq!(r.solid.shell.faces.len(), s.shell.faces.len() + 1);
            assert!(
                (r.solid.volume().unwrap() - s.volume().unwrap()).abs()
                    < s.volume().unwrap() * 1e-12
            );
            assert!((r.solid.bounds().min - s.bounds().min).norm() < scale * 1e-10);
            assert!((r.solid.bounds().max - s.bounds().max).norm() < scale * 1e-10);
            let Surface::ExtrudedCircle { height, .. } = s.shell.faces[fi].surface else {
                panic!()
            };
            for child in 0..2 {
                for u in [
                    0.,
                    0.1,
                    if child == 0 {
                        angle
                    } else {
                        std::f64::consts::FRAC_PI_2 - angle
                    },
                ] {
                    for v in [0., height * 0.43, height] {
                        let original = s.shell.faces[fi]
                            .surface
                            .evaluate(u + if child == 0 { 0. } else { angle }, v);
                        let actual = r.solid.shell.faces[r.faces[child]].surface.evaluate(u, v);
                        assert!((original - actual).norm() < scale * 1e-10);
                        assert!(
                            (s.shell.faces[fi]
                                .surface
                                .normal(u + if child == 0 { 0. } else { angle })
                                - r.solid.shell.faces[r.faces[child]].surface.normal(u))
                            .norm()
                                < 1e-10
                        );
                    }
                }
            }
            let uses: Vec<_> = r
                .solid
                .shell
                .faces
                .iter()
                .flat_map(|f| &f.wires)
                .flat_map(|w| &w.coedges)
                .filter(|c| c.edge == r.generator_edge)
                .collect();
            assert_eq!(uses.len(), 2);
            assert_ne!(uses[0].forward, uses[1].forward);
            check_mesh(&r.solid, scale * 0.01, t.absolute());
            let repeat = subdivide_circular_face(&r.solid, r.faces[1], 0.41, t).unwrap();
            check_mesh(&repeat.solid, scale * 0.01, t.absolute());
        }
    }
}
#[test]
fn cap_cuts_refine_skew_walls_in_both_directions_and_after_placement() {
    let t = GeometryTolerance::default();
    let tr = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    for sign in [-1., 1.] {
        let s = shape(1., sign, t.absolute());
        for fi in [0, 1] {
            let face = &s.shell.faces[fi];
            let anchor = face.surface.evaluate(0., 4.5);
            let Surface::Plane { u, .. } = face.surface else {
                panic!()
            };
            let r = split_planar_face(&s, fi, anchor, u, t).unwrap();
            let placed = s.transformed(tr, t.absolute()).unwrap();
            let rp = split_planar_face(&placed, fi, tr.point(anchor), tr.vector(u), t).unwrap();
            assert_eq!(r.solid.shell.faces.len(), s.shell.faces.len() + 3);
            assert!((r.solid.volume().unwrap() - s.volume().unwrap()).abs() < 1e-10);
            check_mesh(&r.solid, 0.01, t.absolute());
            check_mesh(&rp.solid, 0.01, t.absolute());
            for x in [-5.3, -1., 0., 2.3, 5.2, 7.] {
                for y in [-4.4, 0.1, 2., 5.1] {
                    let p = Point3::new(x + 1.5, y - 1., sign * 2.);
                    let expected = classify_point_in_solid(&s, p, t).unwrap();
                    assert_eq!(classify_point_in_solid(&r.solid, p, t).unwrap(), expected);
                    assert_eq!(
                        classify_point_in_solid(&rp.solid, tr.point(p), t).unwrap(),
                        expected
                    );
                }
            }
        }
    }
}
#[test]
fn bad_queries_reject_atomically_and_normal_cylinders_remain_supported() {
    let t = GeometryTolerance::default();
    let s = shape(1., 1., t.absolute());
    let fi = wall(&s);
    let before = format!("{s:?}");
    for angle in [
        0.,
        -0.1,
        std::f64::consts::FRAC_PI_2,
        1e-9,
        f64::NAN,
        f64::INFINITY,
    ] {
        assert!(subdivide_circular_face(&s, fi, angle, t).is_err());
    }
    assert!(subdivide_circular_face(&s, usize::MAX, 0.3, t).is_err());
    assert!(subdivide_circular_face(&s, 0, 0.3, t).is_err());
    assert_eq!(format!("{s:?}"), before);
    let relative = GeometryTolerance::new(1e-12, 1e-10, 1e-3).unwrap();
    assert!(matches!(
        subdivide_circular_face(&s, fi, 0.05, relative),
        Err(Error::Unsupported(_))
    ));
    let mut bad = s.clone();
    bad.shell.faces[0].orientation *= -1;
    assert!(matches!(
        subdivide_circular_face(&bad, usize::MAX, 0.3, t),
        Err(Error::InvalidTopology(_))
    ));
    let normal = extrude_arc_line(
        &rounded_rectangle_profile(Point3::new(0., 0., 0.), 12., 10., 1., t.absolute()).unwrap(),
        4.,
        t.absolute(),
    )
    .unwrap();
    let fi = normal
        .shell
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::FramedCylinder { .. }))
        .unwrap();
    check_mesh(
        &subdivide_circular_face(&normal, fi, 0.4, t).unwrap().solid,
        0.01,
        t.absolute(),
    );
    let periodic = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 4.,
        },
        t.absolute(),
    )
    .unwrap();
    assert!(subdivide_circular_face(&periodic, 2, 0.4, t).is_err());
}
#[test]
fn repeated_arc_crossings_and_inward_wall_splits_preserve_membership() {
    let t = GeometryTolerance::default();
    let ring = |r| {
        vec![
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: r,
                start_angle: 0.,
                sweep: std::f64::consts::PI,
            },
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: r,
                start_angle: std::f64::consts::PI,
                sweep: std::f64::consts::PI,
            },
        ]
    };
    let s = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: ring(4.),
            holes: vec![ring(2.)],
        },
        Vec3::new(3., -2., 4.),
        t.absolute(),
    )
    .unwrap();
    let r =
        subdivide_planar_face(&s, 0, Point3::new(0., 0.5, 0.), Vec3::new(1., 0., 0.), t).unwrap();
    assert_eq!(r.cut_edges.len(), 2);
    assert!((r.solid.volume().unwrap() - 48. * std::f64::consts::PI).abs() < 1e-10);
    check_mesh(&r.solid, 0.01, t.absolute());
    let fi = r
        .solid
        .shell
        .faces
        .iter()
        .position(|f| f.orientation < 0 && matches!(f.surface, Surface::ExtrudedCircle { .. }))
        .unwrap();
    let split = subdivide_circular_face(&r.solid, fi, 0.1, t).unwrap();
    check_mesh(&split.solid, 0.01, t.absolute());
    for x in [-4.5, -3., -1., 0., 1.3, 2.5, 4.3] {
        for y in [-3.4, -0.1, 1.1, 3.3] {
            let p = Point3::new(x + 1.5, y - 1., 2.);
            assert_eq!(
                classify_point_in_solid(&split.solid, p, t).unwrap(),
                classify_point_in_solid(&s, p, t).unwrap()
            );
        }
    }
}
