use hagane::*;
use std::f64::consts::PI;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let rotation =
        Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), policy(s))
            .unwrap();
    NurbsTorusSolid::new(frame, [16. * s, 4. * s], policy(s)).unwrap()
}
// Independent integral of disk-annular horizontal slices: A(z)=4*pi*R*sqrt(r*r-z*z).
fn lower_volume(c: f64, s: f64) -> f64 {
    let lo = -PI / 2.;
    let hi = (c / (4. * s)).asin();
    let n = 4096;
    let h = (hi - lo) / n as f64;
    let mut sum = 0.;
    for i in 0..=n {
        let t = lo + h * i as f64;
        let weight = if i == 0 || i == n {
            1.
        } else if i % 2 == 0 {
            2.
        } else {
            4.
        };
        sum += weight * t.cos().powi(2);
    }
    4. * PI * (16. * s) * (4. * s).powi(2) * sum * h / 3.
}
fn geometry(child: &NurbsTorusHorizontalSolid, s: f64) {
    let solid = child.solid();
    let source = child.source();
    let mut uses = vec![(0i32, 0i32); solid.edges.len()];
    let mut cap_count = 0;
    for face in &solid.shell.faces {
        if let Surface::Nurbs(surface) = &face.surface {
            let d = surface.domain();
            assert_eq!(d[0], [0., 1.]);
            assert!(d[1][0] >= 0. && d[1][1] <= 1. && d[1][0] < d[1][1]);
            let mid = [0.5, (d[1][0] + d[1][1]) / 2.];
            let midpoint = surface.evaluate(mid[0], mid[1]).unwrap();
            let original = source
                .solid()
                .shell
                .faces
                .iter()
                .find_map(|f| match &f.surface {
                    Surface::Nurbs(net)
                        if (net.evaluate(mid[0], mid[1]).unwrap() - midpoint).norm() < 1e-8 * s =>
                    {
                        Some(net)
                    }
                    _ => None,
                })
                .expect("restriction must retain an original patch and its original UV domain");
            for i in 0..=6 {
                for j in 0..=6 {
                    let u = if i == 6 {
                        d[0][1]
                    } else {
                        d[0][0] + (d[0][1] - d[0][0]) * i as f64 / 6.
                    };
                    let v = if j == 6 {
                        d[1][1]
                    } else {
                        d[1][0] + (d[1][1] - d[1][0]) * j as f64 / 6.
                    };
                    let p = surface.evaluate(u, v).unwrap();
                    assert!((p - original.evaluate(u, v).unwrap()).norm() < 1e-8 * s);
                    let local = source.frame().local_point(p);
                    assert!(
                        ((local.x.hypot(local.y) - 16. * s).hypot(local.z) - 4. * s).abs()
                            < 1e-8 * s
                    );
                    assert!(if child.is_lower() {
                        local.z <= child.cut_height() + 1e-8 * s
                    } else {
                        local.z >= child.cut_height() - 1e-8 * s
                    });
                }
            }
        } else {
            cap_count += 1;
            assert_eq!(face.wires.len(), 2);
            assert_eq!(face.wires.iter().map(|w| w.coedges.len()).sum::<usize>(), 8);
            let normal = face.surface.normal_at(0., 0.).unwrap() * face.orientation as f64;
            assert!(
                normal.dot(source.frame().axes()[2]) * (if child.is_lower() { 1. } else { -1. })
                    > 1. - 1e-10
            );
        }
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
                let range = solid.edges[c.edge].curve.range();
                for k in 0..=16 {
                    let t = range[0] + (range[1] - range[0]) * k as f64 / 16.;
                    let uv = c.pcurve.try_evaluate(t).unwrap();
                    let a = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                    let b = solid.edges[c.edge].curve.try_evaluate(t).unwrap();
                    assert!((a - b).norm() < 1e-8 * s);
                }
            }
        }
    }
    assert_eq!(cap_count, 1);
    assert!(uses.iter().all(|&u| u == (2, 0)));
    for vertex in &solid.vertices {
        let p = source.frame().local_point(vertex.point);
        assert!(if child.is_lower() {
            p.z <= child.cut_height() + 1e-8 * s
        } else {
            p.z >= child.cut_height() - 1e-8 * s
        });
    }
    let step = child.export_step_mm(policy(s)).unwrap();
    assert!(step.contains("MANIFOLD_SOLID_BREP"));
    assert_eq!(
        step.matches("RATIONAL_B_SPLINE_SURFACE(").count(),
        solid.shell.faces.len() - 1
    );
}
#[test]
fn exact_zero_and_nonzero_slices_match_independent_slice_integral_and_actual_boundaries() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let before = format!("{:?}", source.solid());
            let step = source.export_step_mm(policy(s)).unwrap();
            let total = 2. * PI * PI * 16. * 16. * s.powi(3);
            for fraction in [0., 0.4, -0.4, 0.8, -0.8] {
                let c = fraction * 4. * s;
                let split = source.split_by_local_z(c, policy(s)).unwrap();
                for child in [&split.lower, &split.upper] {
                    child.validate(policy(s)).unwrap();
                    geometry(child, s);
                    let expected = if child.is_lower() {
                        lower_volume(c, s)
                    } else {
                        total - lower_volume(c, s)
                    };
                    assert!((child.volume(policy(s)).unwrap() - expected).abs() < 2e-11 * total);
                }
                assert!(
                    (split.lower.volume(policy(s)).unwrap()
                        + split.upper.volume(policy(s)).unwrap()
                        - total)
                        .abs()
                        < 2e-11 * total
                );
            }
            assert_eq!(before, format!("{:?}", source.solid()));
            assert_eq!(step, source.export_step_mm(policy(s)).unwrap());
        }
    }
}
#[test]
fn unsupported_contacts_nonfinite_and_unresolved_near_zero_strip_reject_then_recover() {
    let source = body(1., false);
    let before = format!("{:?}", source.solid());
    for c in [
        4.,
        -4.,
        4. - 1e-10,
        -4. + 1e-10,
        5.,
        1e-12,
        -1e-12,
        f64::NAN,
        f64::INFINITY,
        1e12,
    ] {
        assert!(
            source.split_by_local_z(c, policy(1.)).is_err(),
            "unexpected success at {c}"
        );
    }
    let broad = GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap();
    assert!(source.split_by_local_z(3.9, broad).is_err());
    let split = source.split_by_local_z(0., policy(1.)).unwrap();
    geometry(&split.lower, 1.);
    geometry(&split.upper, 1.);
    assert_eq!(before, format!("{:?}", source.solid()));
}

#[test]
fn small_admitted_cap_has_positive_own_volume_against_smooth_slice_quadrature() {
    let source = body(1., true);
    for sign in [-1., 1.] {
        let c = sign * (4. - 1e-3);
        let split = source.split_by_local_z(c, policy(1.)).unwrap();
        let child = if sign > 0. {
            &split.upper
        } else {
            &split.lower
        };
        let lo = (c.abs() / 4.).asin();
        let hi = PI / 2.;
        let n = 4096;
        let h = (hi - lo) / n as f64;
        let mut sum = 0.;
        for i in 0..=n {
            let t = lo + h * i as f64;
            let w = if i == 0 || i == n {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            };
            sum += w * t.cos().powi(2);
        }
        let expected = 4. * PI * 16. * 16. * sum * h / 3.;
        let actual = child.volume(policy(1.)).unwrap();
        assert!(actual > 0. && (actual - expected).abs() < expected * 1e-9);
        geometry(child, 1.);
    }
}
