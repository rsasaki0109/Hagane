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
fn equivalent(expected: &Solid, actual: &Solid, scale: f64) {
    let world = expected.vertices.iter().fold(30. * scale, |w, v| {
        w.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let guard = 8192. * f64::EPSILON * world;
    actual.validate(policy(scale).absolute()).unwrap();
    assert_eq!(
        (
            actual.vertices.len(),
            actual.edges.len(),
            actual.shell.faces.len()
        ),
        (
            expected.vertices.len(),
            expected.edges.len(),
            expected.shell.faces.len()
        )
    );
    assert_eq!(euler(actual), euler(expected));
    assert!((actual.volume().unwrap() - expected.volume().unwrap()).abs() < 1e-8 * scale.powi(3));
    assert!((actual.bounds().min - expected.bounds().min).norm() < guard);
    assert!((actual.bounds().max - expected.bounds().max).norm() < guard);
    check_retained_curves(expected, actual, guard);
    check_retained_curves(actual, expected, guard);
    check_closed(actual, scale, world);
}
fn roundtrip(s: &Solid, scale: f64) {
    let text = export_step_bounded_analytic_mm(s, policy(scale).linear()).unwrap();
    let parsed = import_step_bounded_analytic_mm(&text, policy(scale).absolute()).unwrap();
    equivalent(s, &parsed, scale);
    let text2 = export_step_bounded_analytic_mm(&parsed, policy(scale).linear()).unwrap();
    let second = import_step_bounded_analytic_mm(&text2, policy(scale).absolute()).unwrap();
    equivalent(&parsed, &second, scale);
}

fn spec(x: f64, y: f64, r: f64, d: f64, top: bool, scale: f64) -> NormalPrismBlindBoreSpec {
    NormalPrismBlindBoreSpec {
        center: Point3::new(x * scale, y * scale, if top { 5. * scale } else { 0. }),
        radius: r * scale,
        depth: d * scale,
        entry: if top {
            NormalPrismBoreEntry::Positive
        } else {
            NormalPrismBoreEntry::Negative
        },
    }
}
fn initial(stock: &Solid, specs: &[NormalPrismBlindBoreSpec], scale: f64) -> Solid {
    blind_bores_normal_prism(stock, specs, Vec3::new(0., 0., 1.), policy(scale))
        .unwrap()
        .kept()
        .clone()
}
fn append(source: &Solid, specs: &[NormalPrismBlindBoreSpec], axis: Vec3, scale: f64) -> Solid {
    let before = format!("{source:?}");
    let result = append_blind_bores_normal_prism(source, specs, axis, policy(scale)).unwrap();
    assert_eq!(before, format!("{source:?}"));
    assert_eq!(result.removed().len(), specs.len());
    assert_eq!(result.hole_faces().len(), specs.len());
    assert_eq!(result.floor_faces().len(), specs.len());
    assert_eq!(
        result.kept().vertices.len(),
        source.vertices.len() + 8 * specs.len()
    );
    assert_eq!(
        result.kept().edges.len(),
        source.edges.len() + 12 * specs.len()
    );
    assert_eq!(
        result.kept().shell.faces.len(),
        source.shell.faces.len() + 5 * specs.len()
    );
    assert_eq!(
        format!("{:?}", source.vertices),
        format!("{:?}", &result.kept().vertices[..source.vertices.len()])
    );
    assert_eq!(
        format!("{:?}", source.edges),
        format!("{:?}", &result.kept().edges[..source.edges.len()])
    );
    for (old, new) in source.shell.faces.iter().zip(&result.kept().shell.faces) {
        assert_eq!(format!("{:?}", old.surface), format!("{:?}", new.surface));
        assert_eq!(old.orientation, new.orientation);
        assert_eq!(
            format!("{:?}", old.wires),
            format!("{:?}", &new.wires[..old.wires.len()])
        );
    }
    let total: f64 = specs
        .iter()
        .map(|s| PI * s.radius * s.radius * s.depth)
        .sum();
    assert!((result.direct_removed_volume() - total).abs() < total * 1e-11);
    assert!(
        (result.kept().volume().unwrap() - (source.volume().unwrap() - total)).abs()
            < source.volume().unwrap() * 1e-11
    );
    assert_eq!(euler(result.kept()), euler(source));
    let world = source.vertices.iter().fold(30. * scale, |w, v| {
        w.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let guard = 8192. * f64::EPSILON * world;
    let max = axis.x.abs().max(axis.y.abs()).max(axis.z.abs());
    let unit = Vec3::new(axis.x / max, axis.y / max, axis.z / max)
        .normalized()
        .unwrap();
    for (((s, body), wallfaces), &floorid) in specs
        .iter()
        .zip(result.removed())
        .zip(result.hole_faces())
        .zip(result.floor_faces())
    {
        let expected = PI * s.radius * s.radius * s.depth;
        assert!((body.volume().unwrap() - expected).abs() < expected * 1e-11);
        assert_eq!(euler(body), 2);
        check_closed(body, scale, world);
        let n = unit
            * if s.entry == NormalPrismBoreEntry::Positive {
                1.
            } else {
                -1.
            };
        let floor_center = s.center - n * s.depth;
        let face = &result.kept().shell.faces[floorid];
        assert!(floorid >= source.shell.faces.len());
        let Surface::Plane { u, v, .. } = face.surface else {
            panic!("actual floor")
        };
        assert!(
            (u.cross(v).normalized().unwrap() * f64::from(face.orientation)).dot(n) > 1. - 1e-12
        );
        for c in &face.wires[0].coedges {
            for &i in &result.kept().edges[c.edge].vertices {
                assert!(
                    (result.kept().vertices[i].point - floor_center)
                        .dot(n)
                        .abs()
                        < guard
                );
            }
        }
        for &fi in wallfaces {
            assert!(fi >= source.shell.faces.len());
            let f = &result.kept().shell.faces[fi];
            let Surface::FramedCylinder {
                frame,
                radius,
                height,
            } = f.surface
            else {
                panic!("new actual cylinder")
            };
            assert!((radius - s.radius).abs() < guard && (height - s.depth).abs() < guard);
            let a = 0.37_f64;
            let radial = frame.vector(Vec3::new(a.cos(), a.sin(), 0.));
            assert!(radial.dot(f.surface.normal(a) * f64::from(f.orientation)) < -1. + 1e-12);
        }
    }
    check_closed(result.kept(), scale, world);
    roundtrip(result.kept(), scale);
    result.kept().clone()
}
#[test]
fn repeated_append_preserves_exact_prefix_and_matches_fresh_batch_scales_axes_pose() {
    for scale in [1e-4, 1., 10.] {
        let stock = block(scale);
        let specs = [
            spec(-6., -3., 0.6, 1., true, scale),
            spec(6., 3., 0.7, 2., false, scale),
            spec(0., -3., 0.5, 1.5, true, scale),
            spec(0., 3., 0.5, 3., false, scale),
        ];
        let one = initial(&stock, &specs[..1], scale);
        let two = append(&one, &specs[1..2], Vec3::new(0., 0., 1.), scale);
        let four = append(&two, &specs[2..], Vec3::new(0., 0., 1.), scale);
        equivalent(&initial(&stock, &specs, scale), &four, scale);
        let three = initial(&stock, &specs[..3], scale);
        equivalent(
            &initial(&stock, &specs, scale),
            &append(&three, &specs[3..], Vec3::new(0., 0., 1.), scale),
            scale,
        );
        if scale == 1. {
            for magnitude in [1e-300, 1e300, 1e-320] {
                let next = append(&one, &specs[1..], Vec3::new(0., 0., magnitude), scale);
                let flipped = specs[1..]
                    .iter()
                    .map(|s| {
                        let mut s = *s;
                        s.entry = if s.entry == NormalPrismBoreEntry::Positive {
                            NormalPrismBoreEntry::Negative
                        } else {
                            NormalPrismBoreEntry::Positive
                        };
                        s
                    })
                    .collect::<Vec<_>>();
                let opposite = append(&one, &flipped, Vec3::new(0., 0., -magnitude), scale);
                assert_eq!(format!("{next:?}"), format!("{opposite:?}"));
            }
        }
    }
    let stock = block(1.);
    let specs = [
        spec(-4., -2., 1., 2., true, 1.),
        spec(4., 2., 0.75, 1.5, false, 1.),
    ];
    let transform = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap();
    let source = initial(&stock, &specs[..1], 1.)
        .transformed(transform, policy(1.).absolute())
        .unwrap();
    let mut next = specs[1];
    next.center = transform.point(next.center);
    append(
        &source,
        &[next],
        transform.vector(Vec3::new(0., 0., 1.)),
        1.,
    );
}
#[test]
fn imported_reordered_shifted_uv_stock_throughholes_concavity_and_cut_children() {
    let stock = block(1.);
    let specs = [
        spec(-4., -2., 1., 2., true, 1.),
        spec(4., 2., 0.75, 1.5, false, 1.),
    ];
    let mut shifted = initial(&stock, &specs[..1], 1.);
    for f in &mut shifted.shell.faces {
        if let Surface::Plane { origin, u, v } = &mut f.surface {
            *origin = *origin + *u * 0.3 + *v * 0.7;
            for c in f.wires.iter_mut().flat_map(|w| &mut w.coedges) {
                match &mut c.pcurve {
                    PCurve::Affine { origin, .. } => {
                        origin[0] -= 0.3;
                        origin[1] -= 0.7;
                    }
                    PCurve::Arc { center, .. } => {
                        center[0] -= 0.3;
                        center[1] -= 0.7;
                    }
                    _ => panic!("scoped analytic plane UV"),
                }
            }
            for w in &mut f.wires {
                w.coedges.rotate_left(1);
            }
        }
    }
    shifted.shell.faces.rotate_left(4);
    let text = export_step_bounded_analytic_mm(&shifted, 1e-6).unwrap();
    let imported = import_step_bounded_analytic_mm(&text, policy(1.).absolute()).unwrap();
    let appended = append(&imported, &specs[1..], Vec3::new(0., 0., 1.), 1.);
    equivalent(&initial(&stock, &specs, 1.), &appended, 1.);
    let profile =
        rounded_rectangle_profile(Point3::new(0., 0., 0.), 20., 16., 2., policy(1.).absolute())
            .unwrap();
    let curved = extrude_arc_line(&profile, 5., policy(1.).absolute()).unwrap();
    let through = bore_normal_prism(
        &curved,
        Point3::new(4., 0., 0.),
        1.,
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    let old = spec(-4., 0., 1., 2., true, 1.);
    let next = spec(0., 3., 0.5, 1., false, 1.);
    append(
        &initial(through.kept(), &[old], 1.),
        &[next],
        Vec3::new(0., 0., 1.),
        1.,
    );
    let concave = extrude_polygon(
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
    append(
        &initial(&concave, &[spec(16., 20., 1., 2., true, 1.)], 1.),
        &[spec(25., 14., 1., 1., false, 1.)],
        Vec3::new(0., 0., 1.),
        1.,
    );
    let cut = split_normal_prism_by_plane_components(
        &stock,
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.),
    )
    .unwrap();
    append(
        &initial(&cut.negative()[0], &[spec(-4., -3., 1., 2., true, 1.)], 1.),
        &[spec(-4., 3., 1., 1., false, 1.)],
        Vec3::new(0., 0., 1.),
        1.,
    );
}
#[test]
fn total_sixteen_profile_budget_and_source_throughhole_resources() {
    let stock = block(1.);
    let specs: Vec<_> = (0..17)
        .map(|i| {
            spec(
                -8. + 4. * (i % 5) as f64,
                -6. + 4. * (i / 5) as f64,
                0.4,
                1.,
                i % 2 == 0,
                1.,
            )
        })
        .collect();
    let one = initial(&stock, &specs[..1], 1.);
    let sixteen = append(&one, &specs[1..16], Vec3::new(0., 0., 1.), 1.);
    equivalent(&initial(&stock, &specs[..16], 1.), &sixteen, 1.);
    assert!(append_blind_bores_normal_prism(
        &sixteen,
        &specs[16..],
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let outer: Vec<_> = (0..120)
        .map(|i| {
            let t = 2. * PI * i as f64 / 120.;
            [20. * t.cos(), 20. * t.sin()]
        })
        .collect();
    let stock = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: vec![],
        },
        Vec3::new(0., 0., 5.),
        policy(1.).absolute(),
    )
    .unwrap();
    let old = initial(&stock, &[spec(-4., 0., 0.4, 1., true, 1.)], 1.);
    let next = spec(4., 0., 0.4, 1., false, 1.);
    append(&old, &[next], Vec3::new(0., 0., 1.), 1.);
    assert!(append_blind_bores_normal_prism(
        &old,
        &[next, spec(0., 4., 0.4, 1., true, 1.)],
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    let centers: Vec<_> = (0..16)
        .map(|i| [-7.5 + 5. * (i % 4) as f64, -6. + 4. * (i / 4) as f64])
        .collect();
    let holes = centers[..15]
        .iter()
        .map(|&center| {
            (0..4)
                .map(|i| PlanarSegment::Arc {
                    center,
                    radius: 0.4,
                    start_angle: i as f64 * PI / 2.,
                    sweep: PI / 2.,
                })
                .collect()
        })
        .collect();
    let region = ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: vec![
            PlanarSegment::Line {
                a: [-10., -8.],
                b: [10., -8.],
            },
            PlanarSegment::Line {
                a: [10., -8.],
                b: [10., 8.],
            },
            PlanarSegment::Line {
                a: [10., 8.],
                b: [-10., 8.],
            },
            PlanarSegment::Line {
                a: [-10., 8.],
                b: [-10., -8.],
            },
        ],
        holes,
    };
    let stock = extrude_arc_line_region(&region, 5., policy(1.).absolute()).unwrap();
    let old = initial(
        &stock,
        &[spec(centers[15][0], centers[15][1], 0.4, 1., true, 1.)],
        1.,
    );
    assert!(append_blind_bores_normal_prism(
        &old,
        &[spec(0., 0., 0.3, 1., false, 1.)],
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
}
#[test]
fn overlapping_corrupt_wrong_axis_and_precision_candidates_preserve_source() {
    let stock = block(1.);
    let oldspec = spec(-4., 0., 1., 2., true, 1.);
    let source = initial(&stock, &[oldspec], 1.);
    let before = format!("{source:?}");
    let good = spec(4., 0., 1., 1., false, 1.);
    let mut cases = vec![
        vec![],
        vec![good; 17],
        vec![spec(-4., 0., 1., 1., false, 1.)],
        vec![spec(-2., 0., 1., 1., true, 1.)],
        vec![good, good],
    ];
    for depth in [0., 5., 5. - 1e-7, f64::NAN] {
        let mut s = good;
        s.depth = depth;
        cases.push(vec![s]);
    }
    let mut s = good;
    s.center.z = 2.5;
    cases.push(vec![s]);
    for specs in cases {
        assert!(append_blind_bores_normal_prism(
            &source,
            &specs,
            Vec3::new(0., 0., 1.),
            policy(1.)
        )
        .is_err());
        assert_eq!(before, format!("{source:?}"));
    }
    for axis in [
        Vec3::new(1., 1., 1.),
        Vec3::new(0., 0., 0.),
        Vec3::new(f64::NAN, 0., 1.),
    ] {
        assert!(append_blind_bores_normal_prism(&source, &[good], axis, policy(1.)).is_err());
    }
    let oldresult =
        blind_bores_normal_prism(&stock, &[oldspec], Vec3::new(0., 0., 1.), policy(1.)).unwrap();
    let mut corrupt = Vec::new();
    let mut p = source.clone();
    let c = &mut p.shell.faces[oldresult.floor_faces()[0]].wires[0].coedges[0];
    let PCurve::Arc { radius, .. } = &mut c.pcurve else {
        unreachable!()
    };
    *radius *= 1.01;
    corrupt.push(p);
    let mut p = source.clone();
    let Surface::FramedCylinder { height, .. } =
        &mut p.shell.faces[oldresult.hole_faces()[0][0]].surface
    else {
        unreachable!()
    };
    *height += 0.1;
    corrupt.push(p);
    let mut p = source.clone();
    p.shell.faces[oldresult.floor_faces()[0]].orientation *= -1;
    corrupt.push(p);
    let mut p = source.clone();
    let edge = p.shell.faces[oldresult.floor_faces()[0]].wires[0].coedges[0].edge;
    let Curve::Arc { sweep, .. } = &mut p.edges[edge].curve else {
        unreachable!()
    };
    *sweep = PI;
    corrupt.push(p);
    for p in corrupt {
        let before = format!("{p:?}");
        assert!(
            append_blind_bores_normal_prism(&p, &[good], Vec3::new(0., 0., 1.), policy(1.))
                .is_err()
        );
        assert_eq!(before, format!("{p:?}"));
    }
    let far = source
        .transformed(
            Transform::translation(Vec3::new(1e10, 0., 0.)).unwrap(),
            policy(1.).absolute(),
        )
        .unwrap();
    let mut s = good;
    s.center.x += 1e10;
    assert!(
        append_blind_bores_normal_prism(&far, &[s], Vec3::new(0., 0., 1.), policy(1.)).is_err()
    );
    assert!(blind_bores_normal_prism(&source, &[good], Vec3::new(0., 0., 1.), policy(1.)).is_err());
    assert!(blind_bore_normal_prism(
        &source,
        good.center,
        good.radius,
        good.depth,
        Vec3::new(0., 0., 1.),
        good.entry,
        policy(1.)
    )
    .is_err());
    assert!(bore_normal_prism(
        &source,
        good.center,
        good.radius,
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
    assert!(split_normal_prism_by_plane_components(
        &source,
        &plane(0.),
        Vec3::new(0., 0., 1.),
        policy(1.)
    )
    .is_err());
}
