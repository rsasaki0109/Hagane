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
fn spec(
    x: f64,
    y: f64,
    radius: f64,
    depth: f64,
    top: bool,
    scale: f64,
) -> NormalPrismBlindBoreSpec {
    NormalPrismBlindBoreSpec {
        center: Point3::new(x * scale, y * scale, if top { 5. * scale } else { 0. }),
        radius: radius * scale,
        depth: depth * scale,
        entry: if top {
            NormalPrismBoreEntry::Positive
        } else {
            NormalPrismBoreEntry::Negative
        },
    }
}
fn exercise(
    source: &Solid,
    specs: &[NormalPrismBlindBoreSpec],
    axis: Vec3,
    scale: f64,
    expected_source: f64,
    genus: usize,
) -> Solid {
    let before = format!("{source:?}");
    let result = blind_bores_normal_prism(source, specs, axis, policy(scale)).unwrap();
    assert_eq!(before, format!("{source:?}"));
    assert_eq!(result.removed().len(), specs.len());
    assert_eq!(result.hole_faces().len(), specs.len());
    assert_eq!(result.floor_faces().len(), specs.len());
    let total: f64 = specs
        .iter()
        .map(|s| PI * s.radius * s.radius * s.depth)
        .sum();
    assert!((result.direct_removed_volume() - total).abs() < total * 1e-11);
    assert!(
        (result.kept().volume().unwrap() - (expected_source - total)).abs()
            < expected_source * 1e-11
    );
    assert_eq!(euler(result.kept()), 2 - 2 * genus as isize);
    let world = source.vertices.iter().fold(30. * scale, |w, v| {
        w.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
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
    let largest = axis.x.abs().max(axis.y.abs()).max(axis.z.abs());
    let unit = Vec3::new(axis.x / largest, axis.y / largest, axis.z / largest)
        .normalized()
        .unwrap();
    for (((s, removed), faces), &floor_index) in specs
        .iter()
        .zip(result.removed())
        .zip(result.hole_faces())
        .zip(result.floor_faces())
    {
        let expected = PI * s.radius * s.radius * s.depth;
        assert!((removed.volume().unwrap() - expected).abs() < expected * 1e-11);
        assert_eq!(euler(removed), 2);
        check_closed(removed, scale, world);
        let outward = unit
            * if s.entry == NormalPrismBoreEntry::Positive {
                1.
            } else {
                -1.
            };
        let floor_center = s.center - outward * s.depth;
        let floor = &result.kept().shell.faces[floor_index];
        let Surface::Plane { u, v, .. } = floor.surface else {
            panic!("actual floor plane")
        };
        assert!(
            (u.cross(v).normalized().unwrap() * f64::from(floor.orientation)).dot(outward)
                > 1. - 1e-12
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
        for &fi in faces {
            let f = &result.kept().shell.faces[fi];
            let Surface::FramedCylinder {
                frame,
                radius,
                height,
            } = f.surface
            else {
                panic!("actual cylinder")
            };
            assert!((radius - s.radius).abs() < guard);
            assert!((height - s.depth).abs() < guard);
            for angle in [0.1_f64, 0.37, 1.2] {
                let radial = frame.vector(Vec3::new(angle.cos(), angle.sin(), 0.));
                assert!(
                    radial.dot(f.surface.normal(angle) * f64::from(f.orientation)) < -1. + 1e-12
                );
            }
        }
        let text = export_step_bounded_analytic_mm(removed, policy(scale).linear()).unwrap();
        let parsed = import_step_bounded_analytic_mm(&text, policy(scale).absolute()).unwrap();
        assert!((parsed.volume().unwrap() - expected).abs() < expected * 1e-11);
    }
    check_closed(result.kept(), scale, world);
    let text = export_step_bounded_analytic_mm(result.kept(), policy(scale).linear()).unwrap();
    if specs.len() > 1 {
        assert!(matches!(
            import_step_bounded_analytic_mm(&text, policy(scale).absolute()),
            Err(Error::Unsupported(_))
        ));
    }
    assert!(blind_bores_normal_prism(result.kept(), specs, axis, policy(scale)).is_err());
    result.kept().clone()
}
#[test]
fn mixed_entry_pockets_scales_pose_axis_magnitudes_and_input_order() {
    for scale in [1e-4, 1., 10.] {
        let source = block(scale);
        let specs = [
            spec(-4., -2., 1., 2., true, scale),
            spec(4., 2., 0.75, 1.5, false, scale),
            spec(0., 3., 0.5, 3., true, scale),
        ];
        let kept = exercise(
            &source,
            &specs,
            Vec3::new(0., 0., 1.),
            scale,
            1600. * scale.powi(3),
            0,
        );
        let mut reversed = specs;
        reversed.reverse();
        let reordered = exercise(
            &source,
            &reversed,
            Vec3::new(0., 0., 1.),
            scale,
            1600. * scale.powi(3),
            0,
        );
        check_retained_curves(&kept, &reordered, 1e-10 * scale);
        check_retained_curves(&reordered, &kept, 1e-10 * scale);
        if scale == 1. {
            for magnitude in [1e-300, 1e300, 1e-320] {
                let equivalent = exercise(
                    &source,
                    &specs,
                    Vec3::new(0., 0., magnitude),
                    scale,
                    1600.,
                    0,
                );
                assert_eq!(format!("{kept:?}"), format!("{equivalent:?}"));
                let flipped = specs.map(|mut s| {
                    s.entry = if s.entry == NormalPrismBoreEntry::Positive {
                        NormalPrismBoreEntry::Negative
                    } else {
                        NormalPrismBoreEntry::Positive
                    };
                    s
                });
                let equivalent = exercise(
                    &source,
                    &flipped,
                    Vec3::new(0., 0., -magnitude),
                    scale,
                    1600.,
                    0,
                );
                assert_eq!(format!("{kept:?}"), format!("{equivalent:?}"));
            }
        }
    }
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = block(1.)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let specs = [
        spec(-4., -2., 1., 2., true, 1.),
        spec(4., 2., 0.75, 1.5, false, 1.),
    ]
    .map(|mut s| {
        s.center = transform.point(s.center);
        s
    });
    exercise(
        &source,
        &specs,
        transform.vector(Vec3::new(0., 0., 1.)),
        1.,
        1600.,
        0,
    );
    let source = block(1.);
    let single = spec(-4., 0., 1., 2., true, 1.);
    let old = blind_bore_normal_prism(
        &source,
        single.center,
        single.radius,
        single.depth,
        Vec3::new(0., 0., 1.),
        single.entry,
        policy(1.),
    )
    .unwrap();
    let additive =
        blind_bores_normal_prism(&source, &[single], Vec3::new(0., 0., 1.), policy(1.)).unwrap();
    assert_eq!(
        format!("{:?}", old.kept()),
        format!("{:?}", additive.kept())
    );
}
#[test]
fn sixteen_pockets_existing_openings_noncentered_concavity_and_cut_child() {
    let source = block(1.);
    let specs: Vec<_> = (0..16)
        .map(|i| {
            spec(
                -7.5 + 5. * (i % 4) as f64,
                -6. + 4. * (i / 4) as f64,
                0.6,
                1. + 0.5 * (i % 3) as f64,
                i % 2 == 0,
                1.,
            )
        })
        .collect();
    exercise(&source, &specs, Vec3::new(0., 0., 1.), 1., 1600., 0);
    let through = bore_normal_prism(
        &source,
        Point3::new(4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    exercise(
        through.kept(),
        &[
            spec(-4., 0., 1., 2., true, 1.),
            spec(0., 3., 0.5, 1., false, 1.),
        ],
        Vec3::new(0., 0., 1.),
        1.,
        1600. - 5. * PI,
        1,
    );
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
        &[
            spec(16., 20., 1., 2., true, 1.),
            spec(25., 14., 1., 1., false, 1.),
        ],
        Vec3::new(0., 0., 1.),
        1.,
        1180.,
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
        &[
            spec(-4., -3., 1., 2., true, 1.),
            spec(-4., 3., 1., 1., false, 1.),
        ],
        Vec3::new(0., 0., 1.),
        1.,
        800.,
        0,
    );
    let p = rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., policy(1.).absolute())
        .unwrap();
    let rounded = extrude_arc_line(&p, 5., policy(1.).absolute()).unwrap();
    exercise(
        &rounded,
        &[
            spec(-4., 0., 1., 2., true, 1.),
            spec(4., 0., 1., 1., false, 1.),
        ],
        Vec3::new(0., 0., 1.),
        1.,
        (304. + 4. * PI) * 5.,
        0,
    );
}
#[test]
fn disk_floor_count_profile_budget_and_nonprism_rejections_are_immutable() {
    let source = block(1.);
    let before = format!("{source:?}");
    let good = spec(-1., 0., 1., 1., true, 1.);
    let mut bad = vec![vec![], vec![good; 17]];
    for second in [
        spec(1., 0., 1., 1., true, 1.),
        spec(1. + 1e-7, 0., 1., 1., true, 1.),
        spec(-1., 0., 0.5, 1., true, 1.),
        spec(-1., 0., 1., 1., false, 1.),
        spec(10., 0., 1., 1., true, 1.),
    ] {
        bad.push(vec![good, second]);
    }
    for depth in [0., -1., 5., 5. - 1e-7, f64::NAN] {
        let mut s = good;
        s.depth = depth;
        bad.push(vec![s]);
    }
    let mut invalid = good;
    invalid.center.z = 2.5;
    bad.push(vec![invalid]);
    let mut invalid = good;
    invalid.radius = f64::INFINITY;
    bad.push(vec![invalid]);
    for specs in bad {
        assert!(
            blind_bores_normal_prism(&source, &specs, Vec3::new(0., 0., 1.), policy(1.)).is_err()
        );
        assert_eq!(format!("{source:?}"), before);
    }
    let root = blind_bore_normal_prism(
        &source,
        good.center,
        good.radius,
        good.depth,
        Vec3::new(0., 0., 1.),
        good.entry,
        policy(1.),
    )
    .unwrap();
    assert!(blind_bores_normal_prism(
        root.kept(),
        &[spec(4., 0., 1., 1., true, 1.)],
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let outer: Vec<_> = (0..125)
        .map(|i| {
            let t = 2. * PI * i as f64 / 125.;
            [20. * t.cos(), 20. * t.sin()]
        })
        .collect();
    let oversized = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: vec![],
        },
        Vec3::new(0., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    assert!(blind_bores_normal_prism(
        &oversized,
        &[spec(0., 0., 1., 1., true, 1.)],
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let through = bore_normal_prism(
        &source,
        Point3::new(0., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    let sixteen: Vec<_> = (0..16)
        .map(|i| {
            spec(
                -7.5 + 5. * (i % 4) as f64,
                -6. + 4. * (i / 4) as f64,
                0.6,
                1.,
                i % 2 == 0,
                1.,
            )
        })
        .collect();
    assert!(matches!(
        blind_bores_normal_prism(through.kept(), &sixteen, Vec3::new(0., 0., 1.), policy(1.)),
        Err(Error::Unsupported(_))
    ));
    let mut malformed = source.clone();
    malformed.shell.faces[0].orientation = 0;
    assert!(
        blind_bores_normal_prism(&malformed, &[good], Vec3::new(0., 0., 1.), policy(1.)).is_err()
    );
}
