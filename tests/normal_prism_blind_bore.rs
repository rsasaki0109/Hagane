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

fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn block(scale: f64) -> Solid {
    make_box(
        BoxSpec {
            min: Point3::new(-10. * scale, -8. * scale, 0.),
            size: Vec3::new(20. * scale, 16. * scale, 5. * scale),
        },
        policy(scale).absolute(),
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
fn exercise(
    source: &Solid,
    center: Point3,
    axis: Vec3,
    entry: NormalPrismBoreEntry,
    scale: f64,
    source_volume: f64,
    genus: usize,
) -> NormalPrismBlindBore {
    let radius = scale;
    let depth = 2. * scale;
    let before = format!("{source:?}");
    let result =
        blind_bore_normal_prism(source, center, radius, depth, axis, entry, policy(scale)).unwrap();
    assert_eq!(before, format!("{source:?}"));
    let removed = PI * radius * radius * depth;
    assert!((result.direct_removed_volume() - removed).abs() < removed * 1e-11);
    assert!((result.removed().volume().unwrap() - removed).abs() < removed * 1e-11);
    assert!(
        (result.kept().volume().unwrap() - (source_volume - removed)).abs() < source_volume * 1e-11
    );
    assert_eq!(euler(result.kept()), 2 - 2 * genus as isize);
    assert_eq!(euler(result.removed()), 2);
    let world = source.vertices.iter().fold(30. * scale, |w, p| {
        w.max(p.point.x.abs())
            .max(p.point.y.abs())
            .max(p.point.z.abs())
    });
    let guard = 8192. * f64::EPSILON * world;
    check_retained_curves(source, result.kept(), guard);
    for (old, new) in source.edges.iter().zip(&result.kept().edges) {
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
    }
    for (old, new) in source.shell.faces.iter().zip(&result.kept().shell.faces) {
        assert_eq!(format!("{:?}", old.surface), format!("{:?}", new.surface));
        assert_eq!(old.orientation, new.orientation);
        for (wire, newwire) in old.wires.iter().zip(&new.wires) {
            assert_eq!(format!("{wire:?}"), format!("{newwire:?}"));
        }
    }
    let unit = axis.normalized().unwrap();
    let outward = unit
        * if matches!(entry, NormalPrismBoreEntry::Positive) {
            1.
        } else {
            -1.
        };
    let floor_center = center - outward * depth;
    let floor = &result.kept().shell.faces[result.floor_face()];
    let Surface::Plane { u, v, .. } = floor.surface else {
        panic!("actual planar floor")
    };
    assert!(
        (u.cross(v).normalized().unwrap() * f64::from(floor.orientation)).dot(outward) > 1. - 1e-12
    );
    assert_eq!(floor.wires.len(), 1);
    assert_eq!(floor.wires[0].coedges.len(), 4);
    for c in &floor.wires[0].coedges {
        for &i in &result.kept().edges[c.edge].vertices {
            assert!(
                (result.kept().vertices[i].point - floor_center)
                    .dot(outward)
                    .abs()
                    < guard
            );
        }
    }
    for &fi in result.hole_faces() {
        let face = &result.kept().shell.faces[fi];
        let Surface::FramedCylinder {
            frame,
            radius: r,
            height,
        } = face.surface
        else {
            panic!("actual quartercylinder")
        };
        assert!((r - radius).abs() < guard);
        assert!((height - depth).abs() < guard);
        let radial = frame.vector(Vec3::new(0.37_f64.cos(), 0.37_f64.sin(), 0.));
        assert!(radial.dot(face.surface.normal(0.37) * f64::from(face.orientation)) < -1. + 1e-12);
    }
    for body in [result.kept(), result.removed()] {
        check_closed(body, scale, world);
    }
    for v in &result.removed().vertices {
        let axial = (center - v.point).dot(outward);
        assert!((-guard..=depth + guard).contains(&axial));
    }
    assert!(bore_normal_prism(result.kept(), center, radius, axis, policy(scale)).is_err());
    assert!(
        split_normal_prism_by_plane_components(result.kept(), &plane(0.), axis, policy(scale))
            .is_err()
    );
    let text = export_step_bounded_analytic_mm(result.kept(), policy(scale).linear()).unwrap();
    let imported_kept = import_step_bounded_analytic_mm(&text, policy(scale).absolute()).unwrap();
    assert!(
        (imported_kept.volume().unwrap() - (source_volume - removed)).abs() < source_volume * 1e-11
    );
    assert_eq!(euler(&imported_kept), euler(result.kept()));
    check_retained_curves(result.kept(), &imported_kept, guard);
    check_closed(&imported_kept, scale, world);
    let text = export_step_bounded_analytic_mm(result.removed(), policy(scale).linear()).unwrap();
    let imported = import_step_bounded_analytic_mm(&text, policy(scale).absolute()).unwrap();
    assert!((imported.volume().unwrap() - removed).abs() < removed * 1e-11);
    result
}
#[test]
fn box_top_bottom_signed_axes_scales_and_rigid_placement() {
    for scale in [1e-4, 1., 10.] {
        let source = block(scale);
        let top = exercise(
            &source,
            Point3::new(-4. * scale, 0., 5. * scale),
            Vec3::new(0., 0., 1.),
            NormalPrismBoreEntry::Positive,
            scale,
            1600. * scale.powi(3),
            0,
        );
        let flipped = exercise(
            &source,
            Point3::new(-4. * scale, 0., 5. * scale),
            Vec3::new(0., 0., -1.),
            NormalPrismBoreEntry::Negative,
            scale,
            1600. * scale.powi(3),
            0,
        );
        assert_eq!(format!("{:?}", top.kept()), format!("{:?}", flipped.kept()));
        if scale == 1. {
            for magnitude in [1e-300, 1e300, 1e-320] {
                for (sign, entry) in [
                    (1., NormalPrismBoreEntry::Positive),
                    (-1., NormalPrismBoreEntry::Negative),
                ] {
                    let equivalent = blind_bore_normal_prism(
                        &source,
                        Point3::new(-4., 0., 5.),
                        1.,
                        2.,
                        Vec3::new(0., 0., sign * magnitude),
                        entry,
                        policy(1.),
                    )
                    .unwrap();
                    assert_eq!(
                        format!("{:?}", equivalent.kept()),
                        format!("{:?}", top.kept())
                    );
                    assert_eq!(
                        format!("{:?}", equivalent.removed()),
                        format!("{:?}", top.removed())
                    );
                }
            }
        }

        exercise(
            &source,
            Point3::new(-4. * scale, 0., 0.),
            Vec3::new(0., 0., 1.),
            NormalPrismBoreEntry::Negative,
            scale,
            1600. * scale.powi(3),
            0,
        );
    }
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = block(1.)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    exercise(
        &source,
        transform.point(Point3::new(-4., 0., 5.)),
        transform.vector(Vec3::new(0., 0., 1.)),
        NormalPrismBoreEntry::Positive,
        1.,
        1600.,
        0,
    );
}
#[test]
fn concave_noncentered_through_hole_curved_stock_and_plain_cut_child() {
    let source = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![
                [10., 10.],
                [30., 10.],
                [30., 18.],
                [20., 18.],
                [20., 26.],
                [10., 26.],
            ],
            holes: vec![vec![[12., 12.], [14., 12.], [14., 14.], [12., 14.]]],
        },
        Vec3::new(0., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    exercise(
        &source,
        Point3::new(16., 20., 5.),
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        1.,
        1180.,
        1,
    );
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., policy(1.).absolute())
            .unwrap();
    let source = extrude_arc_line(&profile, 5., policy(1.).absolute()).unwrap();
    exercise(
        &source,
        Point3::new(-4., 0., 5.),
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        1.,
        (304. + 4. * PI) * 5.,
        0,
    );
    let through = bore_normal_prism(
        &block(1.),
        Point3::new(-4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    exercise(
        through.kept(),
        Point3::new(4., 0., 5.),
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        1.,
        1600. - 5. * PI,
        1,
    );
    let split = split_normal_prism_by_plane_components(
        &block(1.),
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    exercise(
        &split.negative()[0],
        Point3::new(-4., 0., 5.),
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        1.,
        800.,
        0,
    );
}
#[test]
fn blind_depth_entry_side_contact_existing_hole_and_precision_rejections_are_immutable() {
    let source = block(1.);
    let before = format!("{source:?}");
    for (center, radius, depth, axis, tol) in [
        (
            Point3::new(0., 0., 5.),
            1.,
            0.,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            1.,
            5.,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            1.,
            5. - 1e-7,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 2.5),
            1.,
            2.,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(9., 0., 5.),
            1.,
            2.,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            1.,
            2.,
            Vec3::new(0.001, 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            1.,
            2.,
            Vec3::new(0., 0., 0.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            f64::NAN,
            2.,
            Vec3::new(0., 0., 1.),
            policy(1.),
        ),
        (
            Point3::new(0., 0., 5.),
            1.,
            2.,
            Vec3::new(0., 0., 1.),
            GeometryTolerance::new(1., 1e-10, 0.).unwrap(),
        ),
    ] {
        assert!(blind_bore_normal_prism(
            &source,
            center,
            radius,
            depth,
            axis,
            NormalPrismBoreEntry::Positive,
            tol
        )
        .is_err());
        assert_eq!(format!("{source:?}"), before);
    }
    let through = bore_normal_prism(
        &source,
        Point3::new(-4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    for x in [-4., -2.] {
        assert!(blind_bore_normal_prism(
            through.kept(),
            Point3::new(x, 0., 5.),
            1.,
            2.,
            Vec3::new(0., 0., 1.),
            NormalPrismBoreEntry::Positive,
            policy(1.)
        )
        .is_err());
    }
    let mut malformed = source.clone();
    malformed.shell.faces[0].orientation = 0;
    assert!(blind_bore_normal_prism(
        &malformed,
        Point3::new(0., 0., 5.),
        1.,
        2.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        policy(1.)
    )
    .is_err());
    let skew_source = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer: vec![[-10., -8.], [10., -8.], [10., 8.], [-10., 8.]],
            holes: vec![],
        },
        Vec3::new(1., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    assert!(blind_bore_normal_prism(
        &skew_source,
        Point3::new(1., 0., 5.),
        1.,
        2.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        policy(1.)
    )
    .is_err());
    assert!(blind_bore_normal_prism(
        &source,
        Point3::new(0., 0., 0.),
        1.,
        2.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        policy(1.)
    )
    .is_err());
    let periodic = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 10.,
            height: 5.,
        },
        policy(1.).absolute(),
    )
    .unwrap();
    assert!(blind_bore_normal_prism(
        &periodic,
        Point3::new(0., 0., 5.),
        1.,
        2.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        policy(1.)
    )
    .is_err());
    let far = source
        .transformed(
            Transform::translation(Vec3::new(1e10, 0., 0.)).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    assert!(blind_bore_normal_prism(
        &far,
        Point3::new(1e10, 0., 5.),
        1.,
        2.,
        Vec3::new(0., 0., 1.),
        NormalPrismBoreEntry::Positive,
        policy(1.)
    )
    .is_err());
}
