use hagane::*;

fn flat(scale: f64, tol: Tolerance) -> NurbsGraphSolid {
    NurbsGraphSolid::new([scale, scale, scale], 0., tol).unwrap()
}

#[test]
fn strict_line_contact_and_resolution_rules_do_not_snap_input() {
    let t = Tolerance::default();
    let source = flat(1., t);
    source
        .split_uv_line([0.5, 0.], [0.5, 1.], t)
        .unwrap()
        .validate(t)
        .unwrap();
    for (start, end) in [
        ([0., 0.], [1., 1.]),
        ([0., 0.], [1., 0.]),
        ([2., 0.], [2., 1.]),
        ([3. * t.linear, 0.], [3. * t.linear, 1.]),
        ([0.5, 0.], [0.5, 0.]),
        ([0.5, 0.], [0.5 + 1e-10, 1e-10]),
        ([0.5, 0.], [f64::from_bits(0.5f64.to_bits() + 1), 1.]),
        ([f64::NAN, 0.], [0.5, 1.]),
        ([0.5, 0.], [0.5, f64::INFINITY]),
    ] {
        assert!(
            source.split_uv_line(start, end, t).is_err(),
            "accepted {start:?} {end:?}"
        );
    }
}

#[test]
fn output_corner_limit_is_checked_even_for_valid_sixteen_corner_source() {
    let t = Tolerance::default();
    let source = flat(1., t);
    let polygon = (0..16)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 16.;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect();
    let source = NurbsGraphPolygonSolid::new(&source, polygon, t).unwrap();
    // Only the rightmost vertex lies to the right: the other closed output
    // would require fifteen retained vertices plus two intersections.
    assert!(source.split_uv_line([0.885, 0.], [0.885, 1.], t).is_err());
}

#[test]
fn tiny_scaled_and_placed_sources_enforce_physical_tolerance() {
    for scale in [1e-6, 1., 100.] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = flat(scale, t);
        let split = source.split_uv_line([0.375, 0.], [0.375, 1.], t).unwrap();
        split.validate(t).unwrap();
        assert!(
            (split.negative.volume().unwrap() + split.positive.volume().unwrap()
                - source.volume().unwrap())
            .abs()
                < source.volume().unwrap() * 1e-12
        );
        assert!(source.split_uv_line([3e-8, 0.], [3e-8, 1.], t).is_err());
    }
    let t = Tolerance::default();
    let source = flat(1., t);
    assert!(source
        .transformed(
            Transform::translation(Point3::new(1e12, 1e12, 1e12)).unwrap(),
            t
        )
        .is_err());
}

#[test]
fn far_line_origin_must_not_hide_physical_plane_offset_roundoff() {
    let t = Tolerance::default();
    let source = flat(1., t);
    // This mathematically cuts v-u=1/4. A remote plane origin gives signed
    // distances far worse precision than the local solid tolerance.
    assert!(source
        .split_uv_line([1e15, 1e15 + 0.25], [1e15 + 1., 1e15 + 1.25], t)
        .is_err());
}

#[test]
fn public_split_parts_plane_and_section_reject_subtolerance_changes() {
    let t = Tolerance::default();
    let original = flat(1., t).split_uv_line([0.5, 0.], [0.5, 1.], t).unwrap();
    let mut mutated = original.clone();
    mutated.negative.solid.vertices[0].point.x += t.linear / 100.;
    assert!(mutated.validate(t).is_err());
    let mut mutated = original.clone();
    mutated.positive.solid.vertices[0].point.x += t.linear / 100.;
    assert!(mutated.validate(t).is_err());
    let mut mutated = original.clone();
    mutated.section.vertices[0].point.x += t.linear / 100.;
    assert!(mutated.validate(t).is_err());
    let mut mutated = original.clone();
    let Surface::Plane { origin, .. } = &mut mutated.plane else {
        panic!()
    };
    origin.x += t.linear / 100.;
    assert!(mutated.validate(t).is_err());
}

#[test]
fn trimmed_domain_keeps_original_uv_and_rejects_affine_boundary_overshoot() {
    let tolerance = Tolerance::default();
    let resolved = flat(1., tolerance)
        .trimmed_uv([[0.25, 0.75], [0.25, 0.75]], tolerance)
        .unwrap();
    let split = resolved
        .split_uv_line([0.5, 0.], [0.5, 1.], tolerance)
        .unwrap();
    split.validate(tolerance).unwrap();
    assert_eq!(
        split.negative.source().source_domain(),
        [[0.25, 0.75], [0.25, 0.75]]
    );
    let rounded = flat(1., tolerance)
        .trimmed_uv([[0.2, 0.8], [0.3, 0.9]], tolerance)
        .unwrap();
    let boundary = PCurve::Affine {
        origin: [0.2, 0.3],
        direction: [0., 0.9 - 0.3],
    };
    assert!(boundary.evaluate(1.)[1] > 0.9);
    assert!(rounded
        .split_uv_line([0.5, 0.], [0.5, 1.], tolerance)
        .is_err());
}
