use hagane::*;
fn band(linear: f64) -> GeometryTolerance {
    GeometryTolerance::new(linear, 1e-10, 0.).unwrap()
}
// Independent distance to the finite flat stock's actual material boundary.
// Queries below stay in the stock XY rectangle, so a cap's nearest material
// point is either its vertical projection or the circular rim.
fn flat_oracle(p: Point3, epsilon: f64) -> PointLocation {
    let rho = (p.x - 2.).hypot(p.y - 1.5);
    let cap_gap = (0.5 - rho).max(0.);
    let vertical_gap = (-p.z).max(p.z - 2.).max(0.);
    let distance = cap_gap
        .hypot(p.z.abs())
        .min(cap_gap.hypot((p.z - 2.).abs()))
        .min((rho - 0.5).abs().hypot(vertical_gap))
        .min((p.x - 0.5).abs().hypot(vertical_gap))
        .min((3.5 - p.x).abs().hypot(vertical_gap))
        .min((p.y - 0.375).abs().hypot(vertical_gap))
        .min((2.625 - p.y).abs().hypot(vertical_gap));
    if distance <= epsilon {
        PointLocation::Boundary
    } else if rho > 0.5 && p.z > 0. && p.z < 2. {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}
#[test]
fn finite_flat_caps_wall_and_rims_match_independent_euclidean_distance() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([4., 3., 2.], 0., t)
        .unwrap()
        .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], t)
        .unwrap();
    let b = NurbsGraphCircularHoledSolid::new(&s, [2., 1.5], 0.5, t).unwrap();
    let epsilon = 1e-5;
    for angle in [0., 0.37, 1.1, 2.7, 4.9] {
        for radial in [-2., -0.8, -0.6, 0., 0.6, 0.8, 2.] {
            for z in [
                -0.75 * epsilon,
                -0.7 * epsilon,
                0.,
                0.7 * epsilon,
                1.,
                2.,
                2. + 0.75 * epsilon,
            ] {
                let radius = 0.5 + radial * epsilon;
                let p = Point3::new(
                    2. + radius * f64::cos(angle),
                    1.5 + radius * f64::sin(angle),
                    z,
                );
                assert_eq!(
                    b.classify_point(p, band(epsilon))
                        .unwrap_or_else(|e| panic!("p={p:?} error={e:?}")),
                    flat_oracle(p, epsilon),
                    "p={p:?}"
                );
            }
        }
    }
    for z in [0., 1., 2.] {
        assert_eq!(
            b.classify_point(Point3::new(2., 1.5, z), band(epsilon))
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn rigid_placement_signed_roof_and_canonical_mutation_preserve_contract() {
    let t = Tolerance::default();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([4., 3., 2.], -1., t)
        .unwrap()
        .transformed(frame, t)
        .unwrap();
    let body = NurbsGraphCircularHoledSolid::new(&source, [2., 1.5], 0.5, t).unwrap();
    for z in [0., 0.5, 1.75] {
        assert_eq!(
            body.classify_point(frame.point(Point3::new(2., 1.5, z)), band(1e-7))
                .unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        body.classify_point(frame.point(Point3::new(1., 1.5, 0.5)), band(1e-7))
            .unwrap(),
        PointLocation::Inside
    );
    for d in [-0.6, 0.6] {
        assert_eq!(
            body.classify_point(
                frame.point(Point3::new(2.5 + d * 1e-5, 1.5, 0.5)),
                band(1e-5)
            )
            .unwrap(),
            PointLocation::Boundary
        );
    }
    let mut dirty = body.clone();
    dirty.solid.vertices[0].point.x += t.linear * 0.1;
    assert!(dirty
        .classify_point(Point3::new(0., 0., 0.), band(1e-5))
        .is_err());
    for value in [f64::NAN, f64::INFINITY] {
        assert!(body
            .classify_point(Point3::new(value, 0., 0.), band(1e-5))
            .is_err());
    }
}
#[test]
fn steep_roof_offsets_use_normal_distance_instead_of_vertical_gap() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([4., 3., 2.], 40., t).unwrap();
    let b = NurbsGraphCircularHoledSolid::new(&s, [3., 1.5], 0.3, t).unwrap();
    // u=.25,v=.5: h=9.5 and physical dh/dx=5, dh/dy=0.
    let roof = Point3::new(1., 1.5, 9.5);
    let n = Vec3::new(-5., 0., 1.).normalized().unwrap();
    for delta in [-0.6, 0.6] {
        let p = roof + n * (delta * 1e-5);
        assert_eq!(
            b.classify_point(p, band(1e-5)).unwrap(),
            PointLocation::Boundary
        );
    }
    assert_eq!(
        b.classify_point(roof + n * 2e-5, band(1e-5)).unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        b.classify_point(roof - n * 2e-5, band(1e-5)).unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn physical_scaling_and_relative_band_do_not_use_world_translation_scale() {
    for scale in [1e-4, 1., 1e4] {
        let t = Tolerance::new(scale * 1e-9).unwrap();
        let source = NurbsGraphSolid::new([4. * scale, 3. * scale, 2. * scale], 0., t).unwrap();
        let body =
            NurbsGraphCircularHoledSolid::new(&source, [2. * scale, 1.5 * scale], 0.5 * scale, t)
                .unwrap();
        let gt = GeometryTolerance::new(scale * 1e-8, 1e-10, 1e-5).unwrap();
        let epsilon = 29_f64.sqrt() * scale * 1e-5;
        for delta in [-0.6, 0.6] {
            let p = Point3::new(2.5 * scale + delta * epsilon, 1.5 * scale, scale);
            assert_eq!(body.classify_point(p, gt).unwrap(), PointLocation::Boundary);
        }
        assert_eq!(
            body.classify_point(Point3::new(2. * scale, 1.5 * scale, scale), gt)
                .unwrap(),
            PointLocation::Outside
        );
    }
}
#[test]
fn rim_query_at_unresolved_exact_tolerance_threshold_returns_error() {
    let t = Tolerance::default();
    let s = NurbsGraphSolid::new([4., 3., 2.], 0., t)
        .unwrap()
        .trimmed_uv([[0.125, 0.875], [0.125, 0.875]], t)
        .unwrap();
    let b = NurbsGraphCircularHoledSolid::new(&s, [2., 1.5], 0.5, t).unwrap();
    let epsilon = 1e-5;
    let p = Point3::new(2.5 - 0.6 * epsilon, 1.5, -0.8 * epsilon);
    assert!(matches!(
        b.classify_point(p, band(epsilon)),
        Err(Error::Unsupported(_))
    ));
}
