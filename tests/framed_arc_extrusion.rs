use hagane::*;
fn region(scale: f64, t: Tolerance) -> ArcLineRegion {
    ArcLineRegion {
        origin: Point3::new(2. * scale, -scale, 3. * scale),
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
    }
}
fn frame() -> Frame3 {
    Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap()
}
fn mesh_closed(s: &Solid, chord: f64, t: Tolerance) {
    let mesh = s.tessellate(chord, t).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut edges = std::collections::BTreeMap::new();
    for tri in &mesh.triangles {
        let p = tri.map(|i| mesh.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for i in 0..3 {
            let (a, b) = (key(p[i]), key(p[(i + 1) % 3]));
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let v = edges.entry(edge).or_insert((0, 0));
            v.0 += 1;
            v.1 += sign;
        }
    }
    assert!(edges.values().all(|v| *v == (2, 0)));
}
#[test]
fn both_normal_signs_preserve_curved_holes_volume_endpoints_and_closure() {
    let t = GeometryTolerance::default();
    let f = frame();
    for sign in [-1., 1.] {
        for reversed in [false, true] {
            let mut p = region(1., t.absolute());
            if reversed {
                p.outer = p.outer.iter().rev().map(|s| s.reversed()).collect();
                p.holes[0] = p.holes[0].iter().rev().map(|s| s.reversed()).collect();
            }
            let d = f.vector(Vec3::new(0., 0., sign * 4.));
            let s = extrude_arc_line_region_in_frame(&p, d, f, t.absolute()).unwrap();
            s.validate(t.absolute()).unwrap();
            let expected = (108. - (4. - std::f64::consts::PI) * 0.75) * 4.;
            assert!((s.volume().unwrap() - expected).abs() < 1e-10);
            for z in [0., sign * 4.] {
                for segment in &p.outer {
                    let uv = segment.evaluate(0.);
                    let point = f.point(p.origin + Vec3::new(uv[0], uv[1], z));
                    assert!(s.vertices.iter().any(|v| (v.point - point).norm() < 1e-10));
                    assert_eq!(
                        classify_point_in_solid(&s, point, t).unwrap(),
                        PointLocation::Boundary
                    );
                }
            }
            for (x, y, location) in [
                (4., 0., PointLocation::Inside),
                (0., 0., PointLocation::Outside),
                (6., 0., PointLocation::Boundary),
                (7., 0., PointLocation::Outside),
            ] {
                assert_eq!(
                    classify_point_in_solid(&s, f.point(p.origin + Vec3::new(x, y, sign * 2.)), t)
                        .unwrap(),
                    location
                );
            }
            mesh_closed(&s, 0.01, t.absolute());
        }
    }
}
#[test]
fn microscopic_normal_extrusion_and_single_ring_wrapper() {
    let scale = 1e-6;
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let f = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let p = region(scale, t.absolute());
    let s = extrude_arc_line_region_in_frame(
        &p,
        f.vector(Vec3::new(0., 0., -4. * scale)),
        f,
        t.absolute(),
    )
    .unwrap();
    assert!(
        (s.volume().unwrap() / scale.powi(3) - (108. - (4. - std::f64::consts::PI) * 0.75) * 4.)
            .abs()
            < 1e-10
    );
    assert_eq!(
        classify_point_in_solid(
            &s,
            f.point(p.origin + Vec3::new(4. * scale, 0., -2. * scale)),
            t
        )
        .unwrap(),
        PointLocation::Inside
    );
    let profile = ArcLineProfile {
        origin: p.origin,
        segments: p.outer,
    };
    let s = extrude_arc_line_in_frame(
        &profile,
        f.vector(Vec3::new(0., 0., 4. * scale)),
        f,
        t.absolute(),
    )
    .unwrap();
    assert!(
        (s.volume().unwrap() / scale.powi(3) - (120. - (4. - std::f64::consts::PI)) * 4.).abs()
            < 1e-10
    );
}
#[test]
fn invalid_skew_nearly_normal_and_unresolved_span_are_rejected() {
    let t = Tolerance::default();
    let f = frame();
    let p = region(1., t);
    for d in [
        Vec3::new(1., 0., 4.),
        Vec3::new(1e-9, 0., 4.),
        Vec3::new(1., 0., 0.),
    ] {
        assert!(matches!(
            extrude_arc_line_region_in_frame(&p, f.vector(d), f, t),
            Err(Error::Unsupported(_))
        ));
    }
    for d in [
        Vec3::new(0., 0., 0.),
        Vec3::new(0., 0., t.linear),
        Vec3::new(f64::NAN, 0., 4.),
        Vec3::new(0., 0., f64::INFINITY),
    ] {
        assert!(extrude_arc_line_region_in_frame(&p, d, Frame3::IDENTITY, t).is_err());
    }
    let mut bad = p.clone();
    bad.origin.z = f64::MAX;
    assert!(extrude_arc_line_region_in_frame(
        &bad,
        Vec3::new(0., 0., f64::MAX),
        Frame3::IDENTITY,
        t
    )
    .is_err());
    let mut bad = p.clone();
    bad.holes[0] = bad.outer.clone();
    assert!(
        extrude_arc_line_region_in_frame(&bad, f.vector(Vec3::new(0., 0., -4.)), f, t).is_err()
    );
}
#[test]
fn perpendicular_plane_bounds_and_arc_sagitta_are_preserved() {
    let t = Tolerance::default();
    let f = Frame3::new(
        Point3::new(10., 20., 30.),
        [
            Vec3::new(1., 0., 0.),
            Vec3::new(0., 0., 1.),
            Vec3::new(0., -1., 0.),
        ],
        t,
    )
    .unwrap();
    for (sign, lo, hi) in [(1., 13., 17.), (-1., 17., 21.)] {
        let s = extrude_arc_line_region_in_frame(
            &region(1., t),
            f.vector(Vec3::new(0., 0., sign * 4.)),
            f,
            t,
        )
        .unwrap();
        assert_eq!(s.bounds().min, Point3::new(6., lo, 24.));
        assert_eq!(s.bounds().max, Point3::new(18., hi, 34.));
        let mesh = s.tessellate(0.01, t).unwrap();
        for (i, tri) in mesh.triangles.iter().enumerate() {
            if let Surface::FramedCylinder { frame, radius, .. } =
                s.shell.faces[mesh.face_ids[i]].surface
            {
                for j in 0..3 {
                    let p = frame.local_point(
                        (mesh.positions[tri[j]] + mesh.positions[tri[(j + 1) % 3]]) * 0.5,
                    );
                    assert!(radius - p.x.hypot(p.y) <= 0.01 + 1e-12);
                }
            }
        }
    }
}
