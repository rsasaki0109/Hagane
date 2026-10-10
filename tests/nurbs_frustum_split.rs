use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-5 * s, 1e-10, 0.).unwrap()
}
fn pose(s: f64) -> Frame3 {
    Transform::translation(Vec3::new(12. * s, -3. * s, 5. * s))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.31).unwrap())
        .unwrap()
}
fn coord(v: Vec3, i: usize) -> f64 {
    [v.x, v.y, v.z][i]
}
fn close(a: f64, b: f64, scale: f64) {
    assert!((a - b).abs() <= 3e-11 * scale, "{a} != {b}, scale {scale}");
}
// Integrate polynomial radius powers by expanded monomials, independently of
// the production mass routines and their Gauss quadrature.
fn integral(a: f64, delta: f64, power: usize, moment: usize) -> f64 {
    let mut binomial = 1.;
    let mut sum = 0.;
    for k in 0..=power {
        if k > 0 {
            binomial *= (power + 1 - k) as f64 / k as f64;
        }
        sum +=
            binomial * a.powi((power - k) as i32) * delta.powi(k as i32) / (moment + k + 1) as f64;
    }
    sum
}
fn independent_metrics(part: &NurbsFrustumSolid) -> NurbsGraphInertiaProperties {
    let [r0, r1] = part.radii();
    let h = part.height();
    let a = integral(r0, r1 - r0, 2, 0);
    let b = integral(r0, r1 - r0, 2, 1);
    let c = integral(r0, r1 - r0, 2, 2);
    let d = integral(r0, r1 - r0, 4, 0);
    let volume = PI * h * a;
    let centroid = part.frame().point(Point3::new(0., 0., h * b / a));
    let transverse = PI * h * d / 4. + PI * h.powi(3) * (c - b * b / a);
    let local = [transverse, transverse, PI * h * d / 2.];
    let axes = part.frame().axes();
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|k| coord(axes[k], i) * coord(axes[k], j) * local[k])
                .sum()
        })
    });
    NurbsGraphInertiaProperties {
        volume,
        centroid,
        inertia,
    }
}
fn actual_closed(part: &NurbsFrustumSolid, tol: GeometryTolerance) {
    part.validate(tol).unwrap();
    let solid = part.solid();
    assert_eq!(
        (
            solid.vertices.len(),
            solid.edges.len(),
            solid.shell.faces.len()
        ),
        (8, 12, 6)
    );
    assert_eq!(
        solid.vertices.len() as isize - solid.edges.len() as isize
            + solid.shell.faces.len() as isize,
        2
    );
    let mut uses = [(0, 0); 12];
    let guard = 8192.
        * f64::EPSILON
        * part
            .height()
            .max(part.radii()[0])
            .max(part.radii()[1])
            .max(part.frame().origin().norm());
    for face in &solid.shell.faces {
        assert_eq!(face.wires.len(), 1);
        for coedge in &face.wires[0].coedges {
            uses[coedge.edge].0 += 1;
            uses[coedge.edge].1 += face.orientation as i32 * if coedge.forward { 1 } else { -1 };
            for i in 0..=16 {
                let t = i as f64 / 16.;
                let uv = coedge.pcurve.try_evaluate(t).unwrap();
                let p = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                assert!(
                    (p - solid.edges[coedge.edge].curve.try_evaluate(t).unwrap()).norm() <= guard
                );
            }
        }
    }
    assert!(uses.iter().all(|&u| u == (2, 0)));
}
fn closed_mesh(part: &NurbsFrustumSolid, tol: GeometryTolerance, error: f64) {
    let mesh = part.tessellate(error, tol).unwrap();
    let mut edges = BTreeMap::new();
    let key = |p: Point3| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
    for &[a, b, c] in &mesh.triangles {
        assert!(
            (mesh.positions[b] - mesh.positions[a])
                .cross(mesh.positions[c] - mesh.positions[a])
                .norm()
                > 0.
        );
        for (a, b) in [(a, b), (b, c), (c, a)] {
            let a = key(mesh.positions[a]);
            let b = key(mesh.positions[b]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = edges.entry(k).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(edges.values().all(|&u| u == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
    assert!(mesh.signed_volume() < part.volume(tol).unwrap());
    // STEP must retain actual rational tensor nets, not replace the children
    // with analytic cone surfaces or a triangulated representation.
    let step = part.export_step_mm(tol).unwrap();
    assert_eq!(step.matches("B_SPLINE_SURFACE(2,1,").count(), 4);
    assert_eq!(step.matches("RATIONAL_B_SPLINE_SURFACE(").count(), 4);
    assert_eq!(step.matches("RATIONAL_B_SPLINE_CURVE(").count(), 16);
    assert!(!step.contains("TRIANGULATED_FACE_SET"));
    let weight = FRAC_1_SQRT_2.to_string();
    assert!(step.contains(&format!(
        "RATIONAL_B_SPLINE_SURFACE(((1.,1.),({weight},{weight}),(1.,1.)))"
    )));
}

#[test]
fn actual_children_restrict_source_parameterization_and_share_opposed_section() {
    for s in [1e-4, 1., 10.] {
        for radii in [[12., 6.], [6., 12.], [6., 6.]] {
            for frame in [Frame3::IDENTITY, pose(s)] {
                let r = radii.map(|x| x * s);
                let h = 20. * s;
                let tol = policy(s);
                let source = NurbsFrustumSolid::new(frame, r, h, tol).unwrap();
                let before = format!("{:?}", source.solid());
                let cut = 7. * s;
                let split = source.split_axial(cut, tol).unwrap();
                let lower = &split.lower;
                let upper = &split.upper;
                let radius = r[0] + (r[1] - r[0]) * cut / h;
                close(split.cut_height, cut, h);
                close(split.radius, radius, r[0].max(r[1]));
                assert_eq!(split.section.len(), 4);
                close(lower.height(), cut, h);
                close(upper.height(), h - cut, h);
                close(lower.radii()[1], radius, radius);
                close(upper.radii()[0], radius, radius);
                assert_eq!(lower.radii()[0], r[0]);
                assert_eq!(upper.radii()[1], r[1]);
                actual_closed(lower, tol);
                actual_closed(upper, tol);
                let guard = 2e-11 * h;
                for q in 0..4 {
                    let Surface::Nurbs(original) = &source.solid().shell.faces[2 + q].surface
                    else {
                        panic!()
                    };
                    for (child, offset, fraction) in
                        [(lower, 0., cut / h), (upper, cut / h, (h - cut) / h)]
                    {
                        let Surface::Nurbs(surface) = &child.solid().shell.faces[2 + q].surface
                        else {
                            panic!()
                        };
                        assert_eq!(surface.weights(), original.weights());
                        for i in 0..=16 {
                            for j in 0..=16 {
                                let u = i as f64 / 16.;
                                let v = j as f64 / 16.;
                                assert!(
                                    (surface.evaluate(u, v).unwrap()
                                        - original.evaluate(u, offset + fraction * v).unwrap())
                                    .norm()
                                        < guard
                                );
                            }
                        }
                    }
                    let Curve::Nurbs(section) = &split.section[q] else {
                        panic!()
                    };
                    assert_eq!(section.degree(), 2);
                    assert_eq!(section.weights(), [1., FRAC_1_SQRT_2, 1.]);
                    for i in 0..=32 {
                        let t = i as f64 / 32.;
                        let p = section.evaluate(t).unwrap();
                        for edge in [&lower.solid().edges[4 + q], &upper.solid().edges[q]] {
                            assert!((p - edge.curve.try_evaluate(t).unwrap()).norm() < guard);
                        }
                        let local = frame.local_point(p);
                        close(local.z, cut, h);
                        close(local.x.hypot(local.y), radius, radius);
                    }
                }
                let lower_cap = &lower.solid().shell.faces[1];
                let upper_cap = &upper.solid().shell.faces[0];
                let a = lower_cap.surface.normal(0.) * lower_cap.orientation as f64;
                let b = upper_cap.surface.normal(0.) * upper_cap.orientation as f64;
                assert!((a + b).norm() < 1e-12);
                assert_eq!(format!("{:?}", source.solid()), before);
            }
        }
    }
}

#[test]
fn independent_polynomial_moments_add_in_world_axes_and_repeated_splits_display() {
    for s in [1e-4, 1., 10.] {
        let tol = policy(s);
        let source = NurbsFrustumSolid::new(pose(s), [12. * s, 6. * s], 20. * s, tol).unwrap();
        let split = source.split_axial(7. * s, tol).unwrap();
        let original = independent_metrics(&source);
        let mut volume = 0.;
        let mut first = Vec3::new(0., 0., 0.);
        let mut tensor = [[0.; 3]; 3];
        for child in [&split.lower, &split.upper] {
            let expected = independent_metrics(child);
            let actual = child.inertia_properties(tol).unwrap();
            close(actual.volume, expected.volume, original.volume);
            assert!((actual.centroid - expected.centroid).norm() <= 3e-11 * source.height());
            let displacement = expected.centroid - original.centroid;
            for (i, row) in tensor.iter_mut().enumerate() {
                for (j, entry) in row.iter_mut().enumerate() {
                    close(
                        actual.inertia[i][j],
                        expected.inertia[i][j],
                        original.inertia[0][0].max(original.inertia[2][2]),
                    );
                    *entry += actual.inertia[i][j]
                        + actual.volume
                            * (if i == j {
                                displacement.dot(displacement)
                            } else {
                                0.
                            } - coord(displacement, i) * coord(displacement, j));
                }
            }
            volume += actual.volume;
            first = first + (actual.centroid - Point3::new(0., 0., 0.)) * actual.volume;
            closed_mesh(child, tol, 0.3 * s);
        }
        close(volume, original.volume, original.volume);
        assert!(
            (first * (1. / volume) - (original.centroid - Point3::new(0., 0., 0.))).norm()
                <= 3e-11 * source.height()
        );
        for (i, row) in tensor.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                close(
                    value,
                    original.inertia[i][j],
                    original.inertia[0][0].max(original.inertia[2][2]),
                );
            }
        }
        let repeated = split.upper.split_axial(5. * s, tol).unwrap();
        for child in [&repeated.lower, &repeated.upper] {
            actual_closed(child, tol);
        }
        close(
            split.lower.volume(tol).unwrap()
                + repeated.lower.volume(tol).unwrap()
                + repeated.upper.volume(tol).unwrap(),
            original.volume,
            original.volume,
        );
    }
}

#[test]
fn invalid_and_unresolved_cuts_refuse_without_source_mutation_or_scope_changes() {
    let tol = policy(1.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 6.], 20., tol).unwrap();
    let before = format!("{:?}", source.solid());
    for height in [0., -1., 20., 21., f64::NAN, f64::INFINITY, 1e-8, 20. - 1e-8] {
        assert!(source.split_axial(height, tol).is_err(), "height {height}");
    }
    // A policy resolved on the parent must still refuse a child thinner than
    // the parent-scale relative band. Recomputing only the child diameter
    // would incorrectly admit some cuts.
    let relative = GeometryTolerance::new(1e-5, 1e-10, 0.001).unwrap();
    source.validate(relative).unwrap();
    assert!(source.split_axial(0.28, relative).is_err());
    assert!(source.split_axial(19.8, relative).is_err());
    assert!(matches!(
        source.solid().validate(tol.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(format!("{:?}", source.solid()), before);
}
