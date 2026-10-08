use hagane::*;
#[test]
fn rounded_material_matches_analytic_grid_before_and_after_placement() {
    let t = GeometryTolerance::default();
    let s = extrude_arc_line(
        &rounded_rectangle_profile(Point3::new(0., 0., -2.), 8., 6., 1., t.absolute()).unwrap(),
        4.,
        t.absolute(),
    )
    .unwrap();
    let tr = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let placed = s.transformed(tr, t.absolute()).unwrap();
    for x in -18..=18 {
        for y in -14..=14 {
            for z in [-2.5, -2., 0., 2., 2.5] {
                let p = Point3::new(x as f64 * 0.25, y as f64 * 0.25, z);
                let dx = (p.x.abs() - 3.).max(0.);
                let dy = (p.y.abs() - 2.).max(0.);
                let d = dx.hypot(dy);
                let within = d <= 1. && p.x.abs() <= 4. && p.y.abs() <= 3. && z.abs() <= 2.;
                let expected = if !within {
                    PointLocation::Outside
                } else if d == 1. || z.abs() == 2. {
                    PointLocation::Boundary
                } else {
                    PointLocation::Inside
                };
                assert_eq!(
                    classify_point_in_solid(&s, p, t).unwrap(),
                    expected,
                    "{p:?}"
                );
                assert_eq!(
                    classify_point_in_solid(&placed, tr.point(p), t).unwrap(),
                    expected,
                    "placed {p:?}"
                );
            }
        }
    }
}
#[test]
fn periodic_subdivision_preserves_membership_and_seams() {
    let t = GeometryTolerance::default();
    let s = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 4.,
            height: 2.,
        },
        t.absolute(),
    )
    .unwrap();
    let split = subdivide_planar_face(&s, 0, Point3::new(0., 0., 0.), Vec3::new(1., 0., 0.), t)
        .unwrap()
        .solid;
    for x in -9..=9 {
        for y in -9..=9 {
            for z in [0., 1., 2.] {
                let p = Point3::new(x as f64 * 0.5, y as f64 * 0.5, z);
                assert_eq!(
                    classify_point_in_solid(&split, p, t).unwrap(),
                    classify_point_in_solid(&s, p, t).unwrap(),
                    "{p:?}"
                );
            }
        }
    }
}
#[test]
fn arc_rim_band_is_euclidean_and_tiny_geometry_is_supported() {
    for (scale, epsilon) in [(1., 1e-5), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        let s = extrude_arc_line(
            &rounded_rectangle_profile(
                Point3::new(0., 0., 0.),
                8. * scale,
                6. * scale,
                scale,
                t.absolute(),
            )
            .unwrap(),
            4. * scale,
            t.absolute(),
        )
        .unwrap();
        assert_eq!(
            classify_point_in_solid(&s, Point3::new(0., 0., scale), t).unwrap(),
            PointLocation::Inside
        );
        for (offset, expected) in [
            (0.6, PointLocation::Boundary),
            (0.8, PointLocation::Outside),
        ] {
            let r = scale + offset * epsilon;
            let p = Point3::new(
                3. * scale + r / 2_f64.sqrt(),
                2. * scale + r / 2_f64.sqrt(),
                4. * scale + offset * epsilon,
            );
            assert_eq!(classify_point_in_solid(&s, p, t).unwrap(), expected);
        }
    }
}
