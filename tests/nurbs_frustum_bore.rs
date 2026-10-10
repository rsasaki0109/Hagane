use hagane::*;
fn policy(scale: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
}
fn frame(scale: f64, posed: bool) -> Frame3 {
    Transform::translation(Vec3::new(100. * scale, -30. * scale, 5. * scale))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), if posed { 0.7 } else { 0. }).unwrap())
        .unwrap()
}
// Positive cylindrical quadrature; Gauss3 integrates axial and radial
// polynomials exactly, eight angular nodes integrate all first/second moments.
fn oracle(radii: [f64; 2], hole: f64, height: f64) -> (f64, Vec3, [[f64; 3]; 3]) {
    let d = 0.6_f64.sqrt() / 2.;
    let nodes = [(0.5 - d, 5. / 18.), (0.5, 4. / 9.), (0.5 + d, 5. / 18.)];
    let mut volume = 0.;
    let mut m = [0.; 3];
    let mut q = [[0.; 3]; 3];
    for (t, wz) in nodes {
        let z = height * t;
        let radius = radii[0] + (radii[1] - radii[0]) * t;
        for (u, wr) in nodes {
            let rho = hole + (radius - hole) * u;
            let dv = wz * height * wr * (radius - hole) * rho * std::f64::consts::TAU / 8.;
            for j in 0..8 {
                let theta = std::f64::consts::TAU * (j as f64 + 0.5) / 8.;
                let p = [rho * theta.cos(), rho * theta.sin(), z];
                volume += dv;
                for i in 0..3 {
                    m[i] += dv * p[i];
                    for k in 0..3 {
                        q[i][k] += dv * p[i] * p[k];
                    }
                }
            }
        }
    }
    let c = m.map(|x| x / volume);
    let cov: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| q[i][j] - volume * c[i] * c[j]));
    let trace = cov[0][0] + cov[1][1] + cov[2][2];
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                trace - cov[i][j]
            } else {
                -cov[i][j]
            }
        })
    });
    (volume, Vec3::new(c[0], c[1], c[2]), inertia)
}
fn close(a: f64, b: f64, scale: f64) {
    assert!((a - b).abs() < 2e-10 * scale, "{a} vs {b}");
}
fn rotate(local: [[f64; 3]; 3], frame: Frame3) -> [[f64; 3]; 3] {
    let axes = frame.axes().map(|v| [v.x, v.y, v.z]);
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|a| {
                    (0..3)
                        .map(|b| axes[a][i] * local[a][b] * axes[b][j])
                        .sum::<f64>()
                })
                .sum()
        })
    })
}
#[test]
fn positive_integral_moments_pose_and_exact_source_topology() {
    for radii in [[16., 8.], [8., 16.], [12., 12.]] {
        for posed in [false, true] {
            let p = policy(1.);
            let f = frame(1., posed);
            let source = NurbsFrustumSolid::new(f, radii, 24., p).unwrap();
            let bored = source.bore_coaxial(4., p).unwrap();
            bored.validate(p).unwrap();
            let solid = bored.solid();
            assert_eq!(
                (
                    solid.vertices.len(),
                    solid.edges.len(),
                    solid.shell.faces.len()
                ),
                (16, 24, 10)
            );
            assert_eq!(
                format!("{:?}", &solid.vertices[..8]),
                format!("{:?}", source.solid().vertices)
            );
            assert_eq!(
                format!("{:?}", &solid.edges[..12]),
                format!("{:?}", source.solid().edges)
            );
            assert_eq!(
                format!("{:?}", &solid.shell.faces[2..6]),
                format!("{:?}", &source.solid().shell.faces[2..6])
            );
            for face in &solid.shell.faces[..2] {
                assert_eq!(face.wires.len(), 2);
            }
            let mut uses = [(0usize, 0usize); 24];
            for face in &solid.shell.faces {
                for wire in &face.wires {
                    for coedge in &wire.coedges {
                        if (face.orientation > 0) == coedge.forward {
                            uses[coedge.edge].0 += 1;
                        } else {
                            uses[coedge.edge].1 += 1;
                        }
                    }
                }
            }
            assert!(uses.iter().all(|&counts| counts == (1, 1)));
            let (v, c, local) = oracle(radii, 4., 24.);
            let mass = bored.mass_properties(p).unwrap();
            let inertia = bored.inertia_properties(p).unwrap();
            close(mass.volume, v, v);
            assert!((mass.centroid - f.point(c)).norm() < 1e-9);
            let expected = rotate(local, f);
            let scale = expected[0][0] + expected[1][1] + expected[2][2];
            for (i, row) in expected.iter().enumerate() {
                for (j, &x) in row.iter().enumerate() {
                    close(inertia.inertia[i][j], x, scale);
                }
            }
            let tool = bored.tool().mass_properties(p).unwrap();
            let parent = source.mass_properties(p).unwrap();
            let whole_inertia = source.inertia_properties(p).unwrap();
            let tool_inertia = bored.tool().inertia_properties(p).unwrap();
            let mut combined = [[0.; 3]; 3];
            for part in [&inertia, &tool_inertia] {
                let displacement = part.centroid - parent.centroid;
                let xyz = [displacement.x, displacement.y, displacement.z];
                for (i, row) in combined.iter_mut().enumerate() {
                    for (j, value) in row.iter_mut().enumerate() {
                        *value += part.inertia[i][j]
                            + part.volume
                                * ((if i == j {
                                    displacement.dot(displacement)
                                } else {
                                    0.
                                }) - xyz[i] * xyz[j]);
                    }
                }
            }
            for (i, row) in combined.iter().enumerate() {
                for (j, &value) in row.iter().enumerate() {
                    close(value, whole_inertia.inertia[i][j], scale);
                }
            }
            close(mass.volume + tool.volume, parent.volume, parent.volume);
            assert!(
                ((mass.centroid * mass.volume + tool.centroid * tool.volume)
                    * (1. / parent.volume)
                    - parent.centroid)
                    .norm()
                    < 1e-9
            );
        }
    }
}
#[test]
fn scales_nearwall_positive_measure_and_invalid_atomic_inputs() {
    for scale in [1e-40, 1., 1e40] {
        let p = policy(scale);
        let source = NurbsFrustumSolid::new(
            frame(scale, true),
            [16. * scale, 8. * scale],
            24. * scale,
            p,
        )
        .unwrap();
        let bored = source.bore_coaxial(4. * scale, p).unwrap();
        let (v, c, i) = oracle([16., 8.], 4., 24.);
        let mass = bored.mass_properties(p).unwrap();
        close(mass.volume / scale.powi(3), v, v);
        assert!((source.frame().local_point(mass.centroid) * (1. / scale) - c).norm() < 1e-8);
        let expected = rotate(i, source.frame());
        let props = bored.inertia_properties(p).unwrap();
        let budget = expected[0][0] + expected[1][1] + expected[2][2];
        for (a, row) in expected.iter().enumerate() {
            for (b, &x) in row.iter().enumerate() {
                close(props.inertia[a][b] / scale.powi(5), x, budget);
            }
        }
    }
    let p = policy(1.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 24., p).unwrap();
    let before = format!("{:?}", source.solid());
    for radius in [0., -1., 12., 12. - 1e-6, f64::NAN, f64::INFINITY] {
        assert!(source.bore_coaxial(radius, p).is_err());
        assert_eq!(format!("{:?}", source.solid()), before);
    }
    let radius = 12. - 1e-3;
    let bored = source.bore_coaxial(radius, p).unwrap();
    let expected = std::f64::consts::PI * 24. * (12. - radius) * (12. + radius);
    close(bored.volume(p).unwrap(), expected, expected);
    assert!(bored.mass_properties(p).unwrap().centroid.finite());
}

#[test]
fn admitted_thin_cylinder_uses_physical_gap_and_stable_inertia() {
    let p = policy(1.);
    let radius = 16. - 2e-5;
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 16.], 24., p).unwrap();
    let bored = source.bore_coaxial(radius, p).unwrap();
    let volume = std::f64::consts::PI * 24. * (16. - radius) * (16. + radius);
    let mass = bored.mass_properties(p).unwrap();
    assert_eq!(mass.volume, bored.volume(p).unwrap());
    close(mass.volume, volume, volume);
    assert!((mass.centroid - Point3::new(0., 0., 12.)).norm() < 1e-10);
    let inertia = bored.inertia_properties(p).unwrap();
    let radial = (16. * 16. + radius * radius) / 4.;
    let transverse = volume * (radial + 24. * 24. / 12.);
    let axial = volume * 2. * radial;
    close(inertia.inertia[0][0], transverse, transverse);
    close(inertia.inertia[1][1], transverse, transverse);
    close(inertia.inertia[2][2], axial, axial);
}
