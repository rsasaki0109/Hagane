use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn close(a: f64, b: f64, operand: f64) {
    assert!(
        (a - b).abs() <= 2e-10 * operand,
        "{a} != {b}, operand {operand}"
    );
}
fn topology(body: &NurbsBoredFrustumSolid, p: GeometryTolerance) {
    body.validate(p).unwrap();
    let s = body.solid();
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (16, 24, 10)
    );
    assert_eq!(s.shell.faces[0].wires.len(), 2);
    assert_eq!(s.shell.faces[1].wires.len(), 2);
    let mut uses = [(0, 0); 24];
    for face in &s.shell.faces {
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += if c.forward {
                    face.orientation as i32
                } else {
                    -(face.orientation as i32)
                };
            }
        }
    }
    assert!(uses.iter().all(|v| *v == (2, 0)));
    let step = body.export_step_mm(p).unwrap();
    assert!(step.contains("MANIFOLD_SOLID_BREP"));
    assert!(step.matches("FACE_BOUND(").count() >= 2);
}
#[test]
fn actual_annular_children_seams_and_all_mass_moments_conserve_under_pose_and_scale() {
    for scale in [1e-4, 1., 10.] {
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let frame = Frame3::new_with_tolerance(
            Point3::new(13., -7., 5.) * scale,
            rotation.axes(),
            policy(scale),
        )
        .unwrap();
        let body =
            NurbsFrustumSolid::new(frame, [16. * scale, 8. * scale], 24. * scale, policy(scale))
                .unwrap()
                .bore_coaxial(4. * scale, policy(scale))
                .unwrap();
        let before = format!("{:?}", body.solid());
        let parent = body.inertia_properties(policy(scale)).unwrap();
        for cuts in [
            vec![12. * scale],
            vec![6. * scale, 12. * scale, 18. * scale],
        ] {
            let result = body.split_axial_many(&cuts, policy(scale)).unwrap();
            assert_eq!(result.parts.len(), cuts.len() + 1);
            let mut volume = 0.;
            let mut first = Vec3::new(0., 0., 0.);
            let mut tensor = [[0.; 3]; 3];
            let mut sum_abs = [[0.; 3]; 3];
            let mut z0 = 0.;
            for (k, part) in result.parts.iter().enumerate() {
                topology(part, policy(scale));
                let z1 = cuts.get(k).copied().unwrap_or(24. * scale);
                let props = part.inertia_properties(policy(scale)).unwrap();
                let r0 = 16. * scale - z0 / 3.;
                let r1 = 16. * scale - z1 / 3.;
                let analytic = std::f64::consts::PI
                    * (z1 - z0)
                    * ((r0 * r0 + r0 * r1 + r1 * r1) / 3. - 16. * scale * scale);
                close(props.volume, analytic, analytic);
                volume += props.volume;
                first = first + props.centroid * props.volume;
                let d = props.centroid - parent.centroid;
                let xyz = [d.x, d.y, d.z];
                for i in 0..3 {
                    for j in 0..3 {
                        let term = props.inertia[i][j]
                            + props.volume
                                * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
                        tensor[i][j] += term;
                        sum_abs[i][j] += term.abs();
                    }
                }
                let mid = (z0 + z1) / 2.;
                let point = |r| frame.point(Point3::new(r * scale, 0., mid));
                assert_eq!(
                    part.classify_point(point(0.), policy(scale)).unwrap(),
                    PointLocation::Outside
                );
                assert_eq!(
                    part.classify_point(point(4.), policy(scale)).unwrap(),
                    PointLocation::Boundary
                );
                assert_eq!(
                    part.classify_point(point(5.), policy(scale)).unwrap(),
                    PointLocation::Inside
                );
                if k > 0 {
                    let previous = &result.parts[k - 1];
                    for (top, bottom) in [(4, 0), (16, 12)] {
                        for q in 0..4 {
                            for t in [0., 0.23, 0.71, 1.] {
                                let a = previous.solid().edges[top + q]
                                    .curve
                                    .try_evaluate(t)
                                    .unwrap();
                                let b = part.solid().edges[bottom + q]
                                    .curve
                                    .try_evaluate(t)
                                    .unwrap();
                                assert!((a - b).norm() < 1e-10 * scale);
                            }
                        }
                    }
                    let a = &previous.solid().shell.faces[1];
                    let b = &part.solid().shell.faces[0];
                    let na = a.surface.normal_at(0., 0.).unwrap() * a.orientation as f64;
                    let nb = b.surface.normal_at(0., 0.).unwrap() * b.orientation as f64;
                    assert!(na.dot(nb) < -0.999999);
                }
                z0 = z1;
            }
            close(volume, parent.volume, parent.volume);
            let expected = parent.centroid * parent.volume;
            assert!(
                (first - expected).norm()
                    < 2e-10 * parent.volume * frame.origin().norm().max(24. * scale)
            );
            for i in 0..3 {
                for j in 0..3 {
                    close(
                        tensor[i][j],
                        parent.inertia[i][j],
                        sum_abs[i][j]
                            .max(parent.inertia[i][j].abs())
                            .max(parent.volume * scale * scale),
                    );
                }
            }
            let repeated = result.parts[0]
                .split_axial_many(&[cuts[0] / 2.], policy(scale))
                .unwrap();
            assert_eq!(repeated.parts.len(), 2);
            close(
                repeated
                    .parts
                    .iter()
                    .map(|v| v.volume(policy(scale)).unwrap())
                    .sum(),
                result.parts[0].volume(policy(scale)).unwrap(),
                parent.volume,
            );
        }
        assert_eq!(before, format!("{:?}", body.solid()));
    }
}
#[test]
fn admitted_thin_annular_cylinder_never_becomes_filled_on_partition() {
    let p = policy(1.);
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 16.], 24., p)
        .unwrap()
        .bore_coaxial(16. - 2e-5, p)
        .unwrap();
    let result = body.split_axial_many(&[6., 12., 18.], p).unwrap();
    let expected = std::f64::consts::PI * 6. * (16. - body.radius()) * (16. + body.radius());
    for (i, part) in result.parts.iter().enumerate() {
        topology(part, p);
        close(part.volume(p).unwrap(), expected, expected);
        assert_eq!(
            part.classify_point(Point3::new(0., 0., 3. + i as f64 * 6.), p)
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn invalid_partition_requests_reject_without_parent_mutation_and_recover() {
    let p = policy(1.);
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let before = format!("{:?}", body.solid());
    for cuts in [
        vec![],
        vec![0.],
        vec![24.],
        vec![-1.],
        vec![25.],
        vec![8., 8.],
        vec![12., 6.],
        vec![6., 6. + 1e-7],
        vec![f64::NAN],
        vec![f64::INFINITY],
        (1..=17).map(|i| i as f64).collect(),
    ] {
        assert!(body.split_axial_many(&cuts, p).is_err(), "{cuts:?}");
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap();
    assert!(body.split_axial_many(&[0.2], relative).is_err());
    assert_eq!(before, format!("{:?}", body.solid()));
    assert_eq!(body.split_axial_many(&[12.], p).unwrap().parts.len(), 2);
}
