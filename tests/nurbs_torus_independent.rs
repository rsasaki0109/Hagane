use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn tensor(surface: &NurbsSurface, u: f64, v: f64) -> Point3 {
    let bu = [(1. - u).powi(2), 2. * u * (1. - u), u * u];
    let bv = [(1. - v).powi(2), 2. * v * (1. - v), v * v];
    let mut p = Vec3::new(0., 0., 0.);
    let mut w = 0.;
    for (i, &basis_u) in bu.iter().enumerate() {
        for (j, &basis_v) in bv.iter().enumerate() {
            let weight = basis_u * basis_v * surface.weights()[i * 3 + j];
            p = p + surface.control_points()[i * 3 + j] * weight;
            w += weight;
        }
    }
    p * (1. / w)
}
fn topology(solid: &Solid, scale: f64) {
    assert_eq!(
        (
            solid.vertices.len(),
            solid.edges.len(),
            solid.shell.faces.len()
        ),
        (16, 32, 16)
    );
    assert_eq!(
        solid.vertices.len() as isize - solid.edges.len() as isize
            + solid.shell.faces.len() as isize,
        0
    );
    let mut uses = [(0, 0); 32];
    for face in &solid.shell.faces {
        assert_eq!(face.wires.len(), 1);
        assert_eq!(face.wires[0].coedges.len(), 4);
        for c in &face.wires[0].coedges {
            uses[c.edge].0 += 1;
            uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
            for k in 0..=16 {
                let t = k as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                let actual = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let curve = solid.edges[c.edge].curve.try_evaluate(t).unwrap();
                assert!((actual - curve).norm() < 1e-8 * scale);
            }
        }
    }
    assert!(uses.iter().all(|v| *v == (2, 0)));
}
#[test]
fn actual_rational_tensor_coefficients_support_closed_regular_torus_under_pose_scale() {
    for scale in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let p = policy(scale);
            let rotation =
                Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.37 } else { 0. }).unwrap();
            let frame =
                Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * scale, rotation.axes(), p)
                    .unwrap();
            let shape = NurbsTorusSolid::new(frame, [20. * scale, 5. * scale], p).unwrap();
            let before = format!("{:?}", shape.solid());
            shape.validate(p).unwrap();
            topology(shape.solid(), scale);
            let step = shape.export_step_mm(p).unwrap();
            assert_eq!(step.matches("RATIONAL_B_SPLINE_SURFACE(").count(), 16);
            assert!(step.contains("MANIFOLD_SOLID_BREP"));
            let expected = 2. * std::f64::consts::PI.powi(2) * 20. * 25. * scale.powi(3);
            assert!((shape.volume(p).unwrap() - expected).abs() < 1e-12 * expected);
            for face in &shape.solid().shell.faces {
                let Surface::Nurbs(surface) = &face.surface else {
                    panic!("not actualNURBS")
                };
                assert_eq!(surface.degrees(), [2, 2]);
                assert_eq!(surface.control_counts(), [3, 3]);
                assert_eq!(surface.domain(), [[0., 1.], [0., 1.]]);
                for i in 0..3 {
                    for j in 0..3 {
                        let weights = [1., 0.5_f64.sqrt(), 1.];
                        assert!(
                            (surface.weights()[i * 3 + j] - weights[i] * weights[j]).abs() < 1e-15
                        );
                    }
                }
                for i in 0..=8 {
                    for j in 0..=8 {
                        let u = i as f64 / 8.;
                        let v = j as f64 / 8.;
                        let actual = surface.evaluate(u, v).unwrap();
                        assert!((actual - tensor(surface, u, v)).norm() < 1e-10 * scale);
                        let local = frame.local_point(actual);
                        let rho = local.x.hypot(local.y);
                        assert!(
                            ((rho - 20. * scale).hypot(local.z) - 5. * scale).abs() < 1e-10 * scale
                        );
                        let expected = frame
                            .vector(Vec3::new(
                                (rho - 20. * scale) * local.x / rho,
                                (rho - 20. * scale) * local.y / rho,
                                local.z,
                            ))
                            .normalized()
                            .unwrap();
                        let normal =
                            face.surface.normal_at(u, v).unwrap() * face.orientation as f64;
                        assert!(normal.dot(expected) > 1. - 1e-10);
                    }
                }
            }
            assert_eq!(before, format!("{:?}", shape.solid()));
        }
    }
}
#[test]
fn horn_spindle_unresolved_dimensions_and_tampered_actual_coefficients_reject() {
    let p = policy(1.);
    for radii in [
        [0., 1.],
        [20., 0.],
        [5., 5.],
        [4., 5.],
        [5. + 1e-7, 5.],
        [f64::NAN, 5.],
        [20., f64::INFINITY],
    ] {
        assert!(NurbsTorusSolid::new(Frame3::IDENTITY, radii, p).is_err());
    }
    let shape = NurbsTorusSolid::new(Frame3::IDENTITY, [20., 5.], p).unwrap();
    let before = format!("{:?}", shape.solid());
    let mut malformed = shape.solid().clone();
    malformed.shell.faces[0].orientation = -1;
    assert!(NurbsTorusSolid::from_brep(malformed, Frame3::IDENTITY, [20., 5.], p).is_err());
    assert!(NurbsTorusSolid::new(
        Transform::translation(Vec3::new(1e12, 0., 0.)).unwrap(),
        [20., 5.],
        p
    )
    .is_err());
    assert_eq!(before, format!("{:?}", shape.solid()));
    assert!(NurbsTorusSolid::new(Frame3::IDENTITY, [20., 5.], p).is_ok());
}
fn quarter_parameter(angle: f64) -> f64 {
    let half = (angle / 2.).tan();
    half / (0.5_f64.sqrt() + (1. - 0.5_f64.sqrt()) * half)
}
#[test]
fn actual_display_triangles_bound_dense_surface_interior_and_weld_every_seam() {
    use std::collections::BTreeMap;
    let p = policy(1.);
    let shape = NurbsTorusSolid::new(Frame3::IDENTITY, [20., 5.], p).unwrap();
    let mut previous = 0;
    for error in [0.5, 0.05] {
        let mesh = shape.tessellate(error, p).unwrap();
        assert!(mesh.triangles.len() > previous);
        previous = mesh.triangles.len();
        let n = ((mesh.triangles.len() / 32) as f64).sqrt() as usize;
        assert_eq!(mesh.triangles.len(), 32 * n * n);
        assert!(n.is_power_of_two());
        let key = |p: Point3| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
        let mut edges = BTreeMap::new();
        for tri in &mesh.triangles {
            for k in 0..3 {
                let a = key(mesh.positions[tri[k]]);
                let b = key(mesh.positions[tri[(k + 1) % 3]]);
                assert_ne!(a, b);
                let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let usage = edges.entry(edge).or_insert((0, 0));
                usage.0 += 1;
                usage.1 += sign;
            }
        }
        assert!(edges.values().all(|v| *v == (2, 0)));
        for face in 0..16 {
            let Surface::Nurbs(surface) = &shape.solid().shell.faces[face].surface else {
                panic!("NURBS")
            };
            let tris: Vec<_> = mesh
                .triangles
                .iter()
                .zip(&mesh.face_ids)
                .filter_map(|(t, id)| (*id == face).then_some(*t))
                .collect();
            assert_eq!(tris.len(), 2 * n * n);
            for i in 0..n {
                for j in 0..n {
                    for a in 0..=4 {
                        for b in 0..=4 {
                            let x = a as f64 / 4.;
                            let y = b as f64 / 4.;
                            let (offset, weights) = if y <= x {
                                (0, [1. - x, x - y, y])
                            } else {
                                (1, [1. - y, x, y - x])
                            };
                            let tri = tris[2 * (i * n + j) + offset];
                            let chord = mesh.positions[tri[0]] * weights[0]
                                + mesh.positions[tri[1]] * weights[1]
                                + mesh.positions[tri[2]] * weights[2];
                            let u = quarter_parameter(
                                std::f64::consts::FRAC_PI_2 * (i as f64 + x) / n as f64,
                            );
                            let v = quarter_parameter(
                                std::f64::consts::FRAC_PI_2 * (j as f64 + y) / n as f64,
                            );
                            let actual = surface.evaluate(u, v).unwrap();
                            assert!(
                                (actual - chord).norm() <= error,
                                "face{face} error{}",
                                (actual - chord).norm()
                            );
                        }
                    }
                }
            }
        }
        let volume = shape.volume(p).unwrap();
        assert!(mesh.signed_volume() > 0.);
        assert!((mesh.signed_volume() - volume).abs() < volume * 0.05);
    }
    assert!(shape.tessellate(1e-14, p).is_err());
}
#[test]
fn independent_uniform_torus_inertia_matches_rotated_full_world_tensor() {
    for scale in [1e-4, 1., 10.] {
        let p = policy(scale);
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let frame =
            Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * scale, rotation.axes(), p)
                .unwrap();
        let torus = NurbsTorusSolid::new(frame, [20. * scale, 5. * scale], p).unwrap();
        let mass = torus.mass_properties(p).unwrap();
        let inertia = torus.inertia_properties(p).unwrap();
        let volume = 2. * std::f64::consts::PI.powi(2) * 20. * 25. * scale.powi(3);
        assert!((mass.volume - volume).abs() < 1e-12 * volume);
        assert!((mass.centroid - frame.origin()).norm() < 1e-12 * scale);
        let local = [
            volume * (0.5 * 400. + 0.625 * 25.) * scale * scale,
            volume * (0.5 * 400. + 0.625 * 25.) * scale * scale,
            volume * (400. + 0.75 * 25.) * scale * scale,
        ];
        let axes = frame.axes().map(|v| [v.x, v.y, v.z]);
        for i in 0..3 {
            for j in 0..3 {
                let expected = (0..3)
                    .map(|k| axes[k][i] * local[k] * axes[k][j])
                    .sum::<f64>();
                assert!(
                    (inertia.inertia[i][j] - expected).abs()
                        < 1e-12 * volume * scale * scale * 400.
                );
            }
        }
    }
}
