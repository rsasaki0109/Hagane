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

fn check_retained_curves(source: &Solid, kept: &Solid, guard: f64) {
    for e in &source.edges {
        let range = e.curve.range();
        assert!(
            kept.edges.iter().any(|q| {
                let qr = q.curve.range();
                [false, true].into_iter().any(|reverse| {
                    (0..=16).all(|i| {
                        let t = i as f64 / 16.;
                        let u = if reverse { 1. - t } else { t };
                        (e.curve
                            .try_evaluate(range[0] + (range[1] - range[0]) * t)
                            .unwrap()
                            - q.curve.try_evaluate(qr[0] + (qr[1] - qr[0]) * u).unwrap())
                        .norm()
                            <= guard
                    })
                })
            }),
            "retained original line/arc geometry"
        );
    }
}

fn exercise(source: Solid, frame: Transform, scale: f64, expected_source_volume: f64) {
    let tol = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
    let source = source.transformed(frame, tol.absolute()).unwrap();
    let world = source.vertices.iter().fold(30. * scale, |w, v| {
        w.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let before = export_step_bounded_analytic_mm(&source, tol.linear()).unwrap();
    let center = frame.point(Point3::new(-4. * scale, 0., 2. * scale));
    let result = bore_normal_arc_line_prism(&source, center, 1.2 * scale, tol).unwrap();
    assert_eq!(
        export_step_bounded_analytic_mm(&source, tol.linear()).unwrap(),
        before
    );
    assert_eq!(result.hole_faces().len(), 4);
    assert_eq!(euler(result.kept()), 0);
    assert_eq!(euler(result.removed()), 2);
    let removed = PI * (1.2 * scale).powi(2) * (5. * scale);
    assert!((result.direct_removed_volume() - removed).abs() <= removed * 1e-11);
    assert!((result.removed().volume().unwrap() - removed).abs() <= removed * 1e-11);
    assert!(
        (result.kept().volume().unwrap() - (expected_source_volume - removed)).abs()
            <= expected_source_volume * 1e-11
    );
    check_retained_curves(&source, result.kept(), 8192. * f64::EPSILON * world);
    for body in [result.kept(), result.removed()] {
        check_closed(body, scale, world);
        let text = export_step_bounded_analytic_mm(body, tol.linear()).unwrap();
        let imported = import_step_bounded_analytic_mm(&text, tol.absolute()).unwrap();
        check_closed(&imported, scale, world);
        check_retained_curves(body, &imported, 8192. * f64::EPSILON * world);
    }
    for &fi in result.hole_faces() {
        let face = &result.kept().shell.faces[fi];
        let Surface::FramedCylinder {
            frame: wall,
            radius,
            height,
        } = face.surface
        else {
            panic!("circular bore wall")
        };
        assert!((radius - 1.2 * scale).abs() < 1e-10 * scale);
        assert!((height - 5. * scale).abs() < 1e-10 * scale);
        let p = face.surface.try_evaluate(0.37, 0.4 * height).unwrap();
        let radial = wall.vector(Vec3::new(0.37_f64.cos(), 0.37_f64.sin(), 0.));
        assert!(radial.dot(face.surface.normal(0.37) * f64::from(face.orientation)) < -1. + 1e-12);
        assert!(
            (wall.local_point(p).x.hypot(wall.local_point(p).y) - radius).abs() < 1e-10 * scale
        );
    }
    assert_eq!(
        classify_point_in_solid(result.kept(), center, tol).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        classify_point_in_solid(result.removed(), center, tol).unwrap(),
        PointLocation::Inside
    );
    let second = bore_normal_arc_line_prism(
        result.kept(),
        frame.point(Point3::new(4. * scale, 1. * scale, 2. * scale)),
        0.9 * scale,
        tol,
    )
    .unwrap();
    assert_eq!(euler(second.kept()), -2);
    let second_volume = PI * (0.9 * scale).powi(2) * (5. * scale);
    assert!((second.removed().volume().unwrap() - second_volume).abs() <= second_volume * 1e-11);
    assert!(
        (second.kept().volume().unwrap() - (expected_source_volume - removed - second_volume))
            .abs()
            <= expected_source_volume * 1e-11
    );
    check_closed(second.kept(), scale, world);
    check_retained_curves(result.kept(), second.kept(), 8192. * f64::EPSILON * world);
    let second_step = export_step_bounded_analytic_mm(second.kept(), tol.linear()).unwrap();
    let second_import = import_step_bounded_analytic_mm(&second_step, tol.absolute()).unwrap();
    assert_eq!(euler(&second_import), -2);
    check_closed(&second_import, scale, world);
}

#[test]
fn actual_rounded_and_fillet_sources_have_closed_one_and_two_bore_partitions() {
    for scale in [0.001, 1., 1000.] {
        let t = Tolerance::new(1e-6 * scale).unwrap();
        let profile = rounded_rectangle_profile(
            Point3::new(0., 0., 0.),
            20. * scale,
            16. * scale,
            2. * scale,
            t,
        )
        .unwrap();
        let source = extrude_arc_line(&profile, 5. * scale, t).unwrap();
        let volume = (320. - 16. * (1. - PI / 4.)) * 5. * scale.powi(3);
        exercise(source.clone(), Transform::IDENTITY, scale, volume);
        let frame = Transform::translation(Vec3::new(12. * scale, -3. * scale, 5. * scale))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
            .unwrap();
        exercise(source, frame, scale, volume);
    }
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let box_source = make_box(
        BoxSpec {
            min: Point3::new(-10., -8., 0.),
            size: Vec3::new(20., 16., 5.),
        },
        tol.absolute(),
    )
    .unwrap();
    let fillet =
        fillet_parallel_box_edges(&box_source, &[(8, 2.), (9, 2.), (10, 2.), (11, 2.)], tol)
            .unwrap();
    let volume = (320. - 16. * (1. - PI / 4.)) * 5.;
    exercise(fillet.into_solid(), Transform::IDENTITY, 1., volume);
}

#[test]
fn touching_nested_side_crossing_skew_and_precision_fail_without_source_mutation() {
    let tol = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., tol.absolute()).unwrap();
    let source = extrude_arc_line(&profile, 5., tol.absolute()).unwrap();
    let before = export_step_bounded_analytic_mm(&source, tol.linear()).unwrap();
    for (center, radius) in [
        (Point3::new(9., 0., 2.), 1.),
        (Point3::new(9.5, 0., 2.), 1.),
        (Point3::new(8., 6., 2.), 2.),
        (Point3::new(0., 0., 2.), 0.),
        (Point3::new(0., 0., 2.), f64::NAN),
        (Point3::new(1e10, 0., 2.), 1.),
        (Point3::new(f64::INFINITY, 0., 2.), 1.),
    ] {
        assert!(bore_normal_arc_line_prism(&source, center, radius, tol).is_err());
        assert_eq!(
            export_step_bounded_analytic_mm(&source, tol.linear()).unwrap(),
            before
        );
    }
    let prior = bore_normal_arc_line_prism(&source, Point3::new(-4., 0., 2.), 1.2, tol).unwrap();
    let prior_before = export_step_bounded_analytic_mm(prior.kept(), tol.linear()).unwrap();
    for (center, radius) in [
        (Point3::new(-4., 0., 2.), 0.6),
        (Point3::new(-1.6, 0., 2.), 1.2),
        (Point3::new(-2., 0., 2.), 1.2),
        (Point3::new(-4., 0., 2.), 2.),
    ] {
        assert!(bore_normal_arc_line_prism(prior.kept(), center, radius, tol).is_err());
        assert_eq!(
            export_step_bounded_analytic_mm(prior.kept(), tol.linear()).unwrap(),
            prior_before
        );
    }
    let displaced_axis =
        bore_normal_arc_line_prism(&source, Point3::new(-4., 0., -40.), 1.2, tol).unwrap();
    check_retained_curves(prior.kept(), displaced_axis.kept(), 1e-11);
    assert!(
        (prior.kept().volume().unwrap() - displaced_axis.kept().volume().unwrap()).abs() < 1e-9
    );
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
    assert!(bore_normal_arc_line_prism(&skew, Point3::new(0., 0., 2.), 1., tol).is_err());
    let remote = source
        .transformed(
            Transform::translation(Vec3::new(1e10, 0., 0.)).unwrap(),
            tol.absolute(),
        )
        .unwrap();
    assert!(bore_normal_arc_line_prism(&remote, Point3::new(1e10, 0., 2.), 1., tol).is_err());
    let mut malformed = source.clone();
    malformed.shell.faces[0].orientation *= -1;
    assert!(bore_normal_arc_line_prism(&malformed, Point3::new(0., 0., 2.), 1., tol).is_err());
}
