use hagane::*;
fn profile(scale: f64, t: Tolerance) -> ArcLineRegion {
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
#[test]
fn skew_signed_regions_preserve_exact_endpoints_volume_and_world_placement() {
    let t = Tolerance::default();
    let f = Transform::translation(Vec3::new(12., -7., 3.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
        .unwrap();
    for sign in [-1., 1.] {
        for skew in [1e-9, 1., 10., -10.] {
            for reversed in [false, true] {
                let mut p = profile(1., t);
                if reversed {
                    p.outer = p.outer.iter().rev().map(|s| s.reversed()).collect();
                    p.holes[0] = p.holes[0].iter().rev().map(|s| s.reversed()).collect();
                }
                let d = Vec3::new(skew, -2., 4. * sign);
                let solid = extrude_arc_line_region_in_frame(&p, f.vector(d), f, t).unwrap();
                solid.validate(t).unwrap();
                let volume = (108. - (4. - std::f64::consts::PI) * 0.75) * 4.;
                assert!((solid.volume().unwrap() - volume).abs() < 1e-10);
                for ring in std::iter::once(&p.outer).chain(&p.holes) {
                    for segment in ring {
                        for shift in [Vec3::new(0., 0., 0.), d] {
                            let uv = segment.evaluate(0.);
                            let q = f.point(p.origin + Vec3::new(uv[0], uv[1], 0.) + shift);
                            assert!(solid.vertices.iter().any(|v| (v.point - q).norm() < 1e-10));
                        }
                    }
                }
                verify_mesh(&solid, 0.01, t);
                let moved = solid.transformed(f, t).unwrap();
                assert!((moved.volume().unwrap() - volume).abs() < 1e-10);
                for (a, b) in solid.vertices.iter().zip(&moved.vertices) {
                    assert!((f.point(a.point) - b.point).norm() < 1e-10);
                }
            }
        }
    }
}
fn verify_mesh(s: &Solid, chord: f64, t: Tolerance) {
    let mesh = s.tessellate(chord, t).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut edges = std::collections::BTreeMap::new();
    for (i, tri) in mesh.triangles.iter().enumerate() {
        let p = tri.map(|v| mesh.positions[v]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(mesh.normals[tri[0]]) > 0.);
        for j in 0..3 {
            assert!((mesh.normals[tri[j]].norm() - 1.).abs() < 1e-12);
            let (a, b) = (key(p[j]), key(p[(j + 1) % 3]));
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let e = edges.entry(k).or_insert((0, 0));
            e.0 += 1;
            e.1 += sign;
            if let Surface::ExtrudedCircle {
                frame,
                radius,
                drift,
                ..
            } = s.shell.faces[mesh.face_ids[i]].surface
            {
                let q = frame.local_point((p[j] + p[(j + 1) % 3]) * 0.5);
                // Same-v radial correction constructs a point on the exact surface,
                // with Euclidean displacement bounded by the ordinary circle sagitta.
                let r = (q.x - drift[0] * q.z).hypot(q.y - drift[1] * q.z);
                assert!(radius - r <= chord + 1e-12);
            }
        }
    }
    assert!(edges.values().all(|e| *e == (2, 0)));
    assert!((mesh.signed_volume() - s.volume().unwrap()).abs() / s.volume().unwrap() < 0.003);
}
#[test]
fn explicit_bounds_tiny_dimensions_and_surface_round_trips() {
    let t = Tolerance::default();
    let p = profile(1., t);
    let s = extrude_arc_line_region_along(&p, Vec3::new(3., -2., -4.), t).unwrap();
    assert_eq!(s.bounds().min, Point3::new(-4., -8., -1.));
    assert_eq!(s.bounds().max, Point3::new(11., 4., 3.));
    for face in &s.shell.faces {
        if let Surface::ExtrudedCircle { height, .. } = face.surface {
            for u in [0., 0.3, 1.] {
                for v in [0., height * 0.5, height] {
                    let p = face.surface.evaluate(u, v);
                    let uv = face.surface.parameters(p);
                    assert!(
                        (uv[0] - u)
                            .abs()
                            .min((uv[0] - u - std::f64::consts::TAU).abs())
                            < 1e-12
                            && (uv[1] - v).abs() < 1e-12
                    );
                    let du = (face.surface.evaluate(u + 1e-6, v)
                        - face.surface.evaluate(u - 1e-6, v))
                        * 0.5e6;
                    let dv = face.surface.evaluate(u, v + 1.) - p;
                    assert!(
                        (du.cross(dv).normalized().unwrap() - face.surface.normal(u)).norm() < 1e-8
                    );
                }
            }
        }
    }
    let scale = 1e-6;
    let t = Tolerance::new(1e-14).unwrap();
    let s = extrude_arc_line_region_along(
        &profile(scale, t),
        Vec3::new(3. * scale, -2. * scale, -4. * scale),
        t,
    )
    .unwrap();
    assert!(
        (s.volume().unwrap() / scale.powi(3) - (108. - (4. - std::f64::consts::PI) * 0.75) * 4.)
            .abs()
            < 1e-10
    );
    s.tessellate(0.01 * scale, t).unwrap();
}
#[test]
fn invalid_inputs_and_unimplemented_surface_operations_fail_explicitly() {
    let t = Tolerance::default();
    let p = profile(1., t);
    for d in [
        Vec3::new(1., 0., 0.),
        Vec3::new(1., 0., t.linear),
        Vec3::new(f64::NAN, 0., 4.),
        Vec3::new(f64::MAX, f64::MAX, 4.),
    ] {
        assert!(extrude_arc_line_region_along(&p, d, t).is_err());
    }
    let s = extrude_arc_line_region_along(&p, Vec3::new(3., -2., 4.), t).unwrap();
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(0., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    let surface = s
        .shell
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::ExtrudedCircle { .. }))
        .unwrap();
    assert!(matches!(
        intersect_line_cylinder(
            Point3::new(0., 0., 0.),
            Vec3::new(1., 0., 0.),
            &surface.surface,
            GeometryTolerance::default()
        ),
        Err(Error::Unsupported(_))
    ));
    for invalid in [f64::NAN, f64::INFINITY] {
        let mut bad = s.clone();
        if let Surface::ExtrudedCircle { drift, .. } = &mut bad.shell.faces[3].surface {
            drift[0] = invalid;
        } else {
            panic!("fixture must have curved face");
        }
        assert!(bad.validate(t).is_err());
        assert!(bad.tessellate(0.01, t).is_err());
    }
}
#[test]
fn highly_anisotropic_surface_normals_remain_finite() {
    let surface = Surface::ExtrudedCircle {
        frame: Frame3::IDENTITY,
        radius: 1.,
        height: 1.,
        drift: [0., 1e300],
    };
    for u in [
        0.,
        1e-300,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    ] {
        let n = surface.normal(u);
        assert!(n.finite());
        assert!((n.norm() - 1.).abs() < 1e-12);
    }
}
