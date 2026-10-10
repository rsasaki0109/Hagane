use hagane::*;

#[test]
fn multiple_parts_match_original_geometry_and_conserve_mass() {
    for scale in [1e-60, 1., 1e60] {
        let policy = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
        let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.7).unwrap();
        let frame = Frame3::new(
            Point3::new(7. * scale, -3. * scale, 9. * scale),
            rotation.axes(),
            policy.absolute(),
        )
        .unwrap();
        for radii in [[4., 2.], [2., 4.], [3., 3.]] {
            let body = NurbsFrustumSolid::new(frame, radii.map(|r| r * scale), 20. * scale, policy)
                .unwrap();
            let before = format!("{:?}", body.solid());
            let cuts = [3. * scale, 7. * scale, 12. * scale, 17. * scale];
            let result = body.split_axial_many(&cuts, policy).unwrap();
            assert_eq!(result.parts.len(), 5);
            assert_eq!(result.sections.len(), 4);
            let parent = body.mass_properties(policy).unwrap();
            let mut volume = 0.;
            let mut first = Vec3::new(0., 0., 0.);
            let mut start = 0.;
            for (index, part) in result.parts.iter().enumerate() {
                part.validate(policy).unwrap();
                let end = cuts.get(index).copied().unwrap_or(20. * scale);
                let mass = part.mass_properties(policy).unwrap();
                let weight = mass.volume / parent.volume;
                volume += weight;
                first = first + mass.centroid * (weight / scale);
                for face in 2..6 {
                    for i in 0..=4 {
                        for j in 0..=4 {
                            let u = i as f64 / 4.;
                            let v = j as f64 / 4.;
                            let actual = part.solid().shell.faces[face]
                                .surface
                                .try_evaluate(u, v)
                                .unwrap();
                            let expected = body.solid().shell.faces[face]
                                .surface
                                .try_evaluate(u, (start + (end - start) * v) / (20. * scale))
                                .unwrap();
                            let d = actual - expected;
                            assert!(d.x.hypot(d.y).hypot(d.z) < 1e-12 * scale);
                        }
                    }
                }
                assert!(part
                    .export_step_mm(policy)
                    .unwrap()
                    .contains("MANIFOLD_SOLID_BREP"));
                start = end;
            }
            assert!((volume - 1.).abs() < 1e-13);
            let delta = first - parent.centroid * (1. / scale);
            assert!(delta.x.hypot(delta.y).hypot(delta.z) < 1e-12);
            for (i, section) in result.sections.iter().enumerate() {
                assert_eq!(section.len(), 4);
                for (q, curve) in section.iter().enumerate() {
                    for t in [0., 0.25, 0.5, 0.75, 1.] {
                        let a = curve.try_evaluate(t).unwrap();
                        let b = result.parts[i + 1].solid().edges[q]
                            .curve
                            .try_evaluate(t)
                            .unwrap();
                        let d = a - b;
                        assert!(d.x.hypot(d.y).hypot(d.z) < 1e-12 * scale);
                    }
                }
            }
            assert_eq!(before, format!("{:?}", body.solid()));
        }
    }
}

#[test]
fn invalid_lists_are_atomic_and_use_full_source_band() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [4., 2.], 20., policy).unwrap();
    let before = format!("{:?}", body.solid());
    for cuts in [
        vec![],
        vec![0.],
        vec![20.],
        vec![-1.],
        vec![21.],
        vec![2., 2.],
        vec![3., 2.],
        vec![2., f64::NAN],
        vec![f64::INFINITY],
        vec![2., 2.000001],
        vec![19.999999],
        (1..=17).map(f64::from).collect(),
    ] {
        assert!(body.split_axial_many(&cuts, policy).is_err());
        assert_eq!(before, format!("{:?}", body.solid()));
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 1e-3).unwrap();
    assert!(body.split_axial_many(&[18., 18.1], relative).is_err());
    let cuts: Vec<_> = (1..=16).map(f64::from).collect();
    assert_eq!(
        body.split_axial_many(&cuts, policy).unwrap().parts.len(),
        17
    );
}
