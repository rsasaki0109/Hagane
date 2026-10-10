use hagane::*;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn body(radii: [f64; 2], posed: bool) -> NurbsFrustumSolid {
    let frame = if posed {
        Transform::translation(Vec3::new(12., -3., 5.))
            .unwrap()
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
            .unwrap()
    } else {
        Frame3::IDENTITY
    };
    NurbsFrustumSolid::new(frame, radii, 24., policy()).unwrap()
}
fn plane(source: &NurbsFrustumSolid, c: f64, a: f64, b: f64, reverse: bool) -> Surface {
    let u = Vec3::new(1., 0., -a).normalized().unwrap();
    let n = Vec3::new(a, b, 1.).normalized().unwrap();
    let v = n.cross(u).normalized().unwrap() * if reverse { -1. } else { 1. };
    Surface::Plane {
        origin: source.frame().point(Point3::new(0., 0., c)),
        u: source.frame().vector(u),
        v: source.frame().vector(v),
    }
}
// Independent positive integration in source cylindrical coordinates. This
// does not use the production cone/ellipse formula or subtract near-equal cones.
fn lower_volume(radii: [f64; 2], c: f64, a: f64, b: f64) -> f64 {
    let nodes = [
        0.1834346424956498,
        0.525532409916329,
        0.7966664774136267,
        0.9602898564975363,
    ];
    let weights = [
        0.362683783378362,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    ];
    let s = (radii[1] - radii[0]) / 24.;
    let mut sum = 0.;
    let dz = (0.6_f64).sqrt() / 2.;
    for i in 0..4 {
        for sign in [-1., 1.] {
            let rho = (1. + sign * nodes[i]) / 2.;
            let weight = weights[i] / 2.;
            for j in 0..512 {
                let theta = std::f64::consts::TAU * (j as f64 + 0.5) / 512.;
                let q = rho * (a * theta.cos() + b * theta.sin());
                let z = (c - radii[0] * q) / (1. + s * q);
                let radial_integral = [(0.5 - dz, 5. / 18.), (0.5, 4. / 9.), (0.5 + dz, 5. / 18.)]
                    .iter()
                    .map(|(t, w)| w * (radii[0] + s * z * t).powi(2))
                    .sum::<f64>()
                    * z;
                sum += weight * rho * radial_integral * std::f64::consts::TAU / 512.;
            }
        }
    }
    sum
}
fn check_child(source: &NurbsFrustumSolid, child: &NurbsObliqueFrustumSolid) {
    child.validate(policy()).unwrap();
    let solid = child.solid();
    assert_eq!(
        (
            solid.vertices.len(),
            solid.edges.len(),
            solid.shell.faces.len()
        ),
        (8, 12, 6)
    );
    let mut uses = [(0, 0); 12];
    for face in &solid.shell.faces {
        for coedge in &face.wires[0].coedges {
            uses[coedge.edge].0 += 1;
            uses[coedge.edge].1 += face.orientation as i32 * if coedge.forward { 1 } else { -1 };
            let curve = &solid.edges[coedge.edge].curve;
            for i in 0..=32 {
                let t = i as f64 / 32.;
                let uv = coedge.pcurve.try_evaluate(t).unwrap();
                assert!(
                    (curve.try_evaluate(t).unwrap()
                        - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                    .norm()
                        < 1e-8
                );
            }
        }
    }
    assert!(uses.iter().all(|u| *u == (2, 0)));
    for q in 0..4 {
        for u in [0., 0.2, 0.7, 1.] {
            for v in [0., 0.2, 0.6, 1.] {
                let point = solid.shell.faces[2 + q].surface.try_evaluate(u, v).unwrap();
                let local = source.frame().local_point(point);
                let expected = source.solid().shell.faces[2 + q]
                    .surface
                    .try_evaluate(
                        u,
                        if v == 0. && child.is_lower() {
                            0.
                        } else if v == 1. && !child.is_lower() {
                            1.
                        } else {
                            local.z / 24.
                        },
                    )
                    .unwrap();
                assert!((point - expected).norm() < 1e-8);
            }
        }
    }
    assert!(child
        .export_step_mm(policy())
        .unwrap()
        .contains("MANIFOLD_SOLID_BREP"));
}

#[test]
fn independent_positive_volume_and_actual_closed_source_coverage() {
    for radii in [[16., 8.], [8., 16.], [12., 12.], [12., 12. + 1e-10]] {
        for posed in [false, true] {
            let source = body(radii, posed);
            let cut = plane(&source, 12., 0.1, -0.05, false);
            let split = source.split_by_plane(&cut, policy()).unwrap();
            check_child(&source, &split.lower);
            check_child(&source, &split.upper);
            let expected = lower_volume(radii, 12., 0.1, -0.05);
            let actual = split.lower.volume(policy()).unwrap();
            assert!(
                (actual - expected).abs() < 2e-10 * expected,
                "{actual} vs {expected}"
            );
            let total = std::f64::consts::PI
                * 24.
                * (radii[0].powi(2) + radii[0] * radii[1] + radii[1].powi(2))
                / 3.;
            assert!((actual + split.upper.volume(policy()).unwrap() - total).abs() < 1e-10 * total);
            let Surface::Plane { u: a, v: b, .. } = split.lower.solid().shell.faces[1].surface
            else {
                panic!("cut cap");
            };
            let Surface::Plane { u: c, v: d, .. } = split.upper.solid().shell.faces[0].surface
            else {
                panic!("cut cap");
            };
            assert!(a.cross(b).dot(c.cross(d)) > 0.);
            assert_eq!(split.lower.solid().shell.faces[1].orientation, 1);
            assert_eq!(split.upper.solid().shell.faces[0].orientation, -1);
        }
    }
}

#[test]
fn horizontal_and_reversed_plane_preserve_partition_geometry_and_volume() {
    let source = body([16., 8.], true);
    let axial = source.split_axial(12., policy()).unwrap();
    let horizontal = source
        .split_by_plane(&plane(&source, 12., 0., 0., false), policy())
        .unwrap();
    assert!(
        (axial.lower.volume(policy()).unwrap() - horizontal.lower.volume(policy()).unwrap()).abs()
            < 1e-8
    );
    let forward = source
        .split_by_plane(&plane(&source, 12., 0.1, -0.05, false), policy())
        .unwrap();
    let reverse = source
        .split_by_plane(&plane(&source, 12., 0.1, -0.05, true), policy())
        .unwrap();
    assert!(
        (forward.lower.volume(policy()).unwrap() - reverse.lower.volume(policy()).unwrap()).abs()
            < 1e-8
    );
    for (a, b) in forward
        .lower
        .solid()
        .edges
        .iter()
        .zip(&reverse.lower.solid().edges)
    {
        for i in 0..=32 {
            let t = i as f64 / 32.;
            assert!(
                (a.curve.try_evaluate(t).unwrap() - b.curve.try_evaluate(t).unwrap()).norm() < 1e-8
            );
        }
    }
}

#[test]
fn contacts_crosscaps_and_unresolved_planes_reject_without_source_mutation() {
    let source = body([16., 8.], false);
    let original = source.export_step_mm(policy()).unwrap();
    for surface in [
        plane(&source, 0., 0., 0., false),
        plane(&source, 24., 0., 0., false),
        plane(&source, 12., 5., 0., false),
        Surface::Plane {
            origin: Point3::new(1e12, 0., 12.),
            u: Vec3::new(1., 0., 0.),
            v: Vec3::new(0., 1., 0.),
        },
    ] {
        assert!(source.split_by_plane(&surface, policy()).is_err());
    }
    assert_eq!(source.export_step_mm(policy()).unwrap(), original);
    source
        .split_by_plane(&plane(&source, 12., 0.1, -0.05, false), policy())
        .unwrap();
}

#[test]
fn scaled_and_thin_resolved_parts_keep_positive_analytic_volumes() {
    let reference = lower_volume([16., 8.], 12., 0.1, -0.05);
    for scale in [1e-50, 1e50] {
        let p = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
        let source =
            NurbsFrustumSolid::new(Frame3::IDENTITY, [16. * scale, 8. * scale], 24. * scale, p)
                .unwrap();
        let split = source
            .split_by_plane(&plane(&source, 12. * scale, 0.1, -0.05, false), p)
            .unwrap();
        let normalized = split.lower.volume(p).unwrap() / (scale * scale * scale);
        assert!((normalized - reference).abs() < 2e-10 * reference);
        assert!(split.upper.volume(p).unwrap() > 0.);
    }
    let cylinder = body([12., 12.], false);
    for c in [1e-4, 24. - 1e-4] {
        let split = cylinder
            .split_by_plane(&plane(&cylinder, c, 1e-6, 0., false), policy())
            .unwrap();
        let lower = std::f64::consts::PI * 144. * c;
        let upper = std::f64::consts::PI * 144. * (24. - c);
        assert!((split.lower.volume(policy()).unwrap() - lower).abs() < 1e-9 * lower);
        assert!((split.upper.volume(policy()).unwrap() - upper).abs() < 1e-9 * upper);
    }
}
