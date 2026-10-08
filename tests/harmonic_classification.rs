use hagane::*;
fn shape(scale: f64, sign: f64, t: Tolerance) -> Solid {
    extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: rounded_rectangle_profile(
                Point3::new(0., 0., 0.),
                12. * scale,
                10. * scale,
                scale,
                t,
            )
            .unwrap()
            .segments,
            holes: vec![
                rounded_rectangle_profile(
                    Point3::new(0., 0., 0.),
                    4. * scale,
                    3. * scale,
                    0.5 * scale,
                    t,
                )
                .unwrap()
                .segments,
            ],
        },
        Vec3::new(3. * scale, -2. * scale, sign * 4. * scale),
        t,
    )
    .unwrap()
}
#[test]
fn subdivided_material_holes_caps_and_section_edges_preserve_membership() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for sign in [-1., 1.] {
            let s = shape(scale, sign, t.absolute());
            let r = subdivide_extrusion_boundary_by_plane(
                &s,
                Point3::new(1.5 * scale, -scale, sign * 2. * scale),
                Vec3::new(0.1, -0.08, sign),
                t,
            )
            .unwrap();
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
            let placed = r.solid.transformed(transform, t.absolute()).unwrap();
            for z in [0.25, 0.5, 0.75] {
                for (x, y, expected) in [
                    (4., 0., PointLocation::Inside),
                    (0., 0., PointLocation::Outside),
                    (5.9, 4.9, PointLocation::Outside),
                    (6., 3., PointLocation::Boundary),
                ] {
                    let p = Vec3::new(
                        (x + 3. * z) * scale,
                        (y - 2. * z) * scale,
                        sign * 4. * z * scale,
                    );
                    assert_eq!(classify_point_in_solid(&s, p, t).unwrap(), expected);
                    assert_eq!(classify_point_in_solid(&r.solid, p, t).unwrap(), expected);
                    assert_eq!(
                        classify_point_in_solid(&placed, transform.point(p), t).unwrap(),
                        expected
                    );
                }
            }
            for index in &r.section_edges {
                let edge = &r.solid.edges[*index];
                let p = edge.curve.evaluate(edge.curve.range()[1] * 0.37);
                assert_eq!(
                    classify_point_in_solid(&r.solid, p, t).unwrap(),
                    PointLocation::Boundary
                );
            }
            for z in [0., 1.] {
                let p = Vec3::new(
                    (4. + 3. * z) * scale,
                    -2. * z * scale,
                    sign * 4. * z * scale,
                );
                assert_eq!(
                    classify_point_in_solid(&r.solid, p, t).unwrap(),
                    PointLocation::Boundary
                );
            }
        }
    }
}
#[test]
fn curved_normal_bands_and_invalid_topology_are_checked() {
    let t = GeometryTolerance::new(1e-5, 1e-10, 0.).unwrap();
    let s = shape(1., 1., t.absolute());
    let r = subdivide_extrusion_boundary_by_plane(
        &s,
        Point3::new(1.5, -1., 2.),
        Vec3::new(0.1, -0.08, 1.),
        t,
    )
    .unwrap();
    for (f, index) in r.solid.shell.faces.iter().enumerate() {
        if !matches!(index.surface, Surface::ExtrudedCircle { .. }) {
            continue;
        }
        let rim = &index.wires[0].coedges[2];
        if !matches!(rim.pcurve, PCurve::HeightGraph { .. }) {
            continue;
        }
        let uv = rim
            .pcurve
            .evaluate(r.solid.edges[rim.edge].curve.range()[1] * 0.37);
        let p = index.surface.evaluate(uv[0], uv[1]);
        let normal = index.surface.normal(uv[0]) * index.orientation as f64;
        for (distance, expected) in [
            (0.25e-5, PointLocation::Boundary),
            (2e-5, PointLocation::Outside),
            (-2e-5, PointLocation::Inside),
        ] {
            assert_eq!(
                classify_point_in_solid(&r.solid, p + normal * distance, t).unwrap(),
                expected,
                "face {f}"
            );
        }
    }
    let mut broken = r.solid;
    let wall = r
        .split_faces
        .iter()
        .find(|pair| {
            matches!(
                broken.shell.faces[pair[0]].surface,
                Surface::ExtrudedCircle { .. }
            )
        })
        .unwrap()[0];
    let PCurve::HeightGraph { ref mut offset, .. } =
        broken.shell.faces[wall].wires[0].coedges[2].pcurve
    else {
        panic!("harmonic rim required")
    };
    *offset += 1.;
    assert!(classify_point_in_solid(&broken, Point3::new(100., 100., 100.), t).is_err());
}
#[test]
fn normal_cylinder_bands_do_not_turn_section_plane_into_a_boundary() {
    let t = GeometryTolerance::default();
    let source = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            holes: vec![],
            outer: [0., std::f64::consts::PI]
                .map(|start_angle| PlanarSegment::Arc {
                    center: [0., 0.],
                    radius: 2.,
                    start_angle,
                    sweep: std::f64::consts::PI,
                })
                .to_vec(),
        },
        Vec3::new(0., 0., 4.),
        t.absolute(),
    )
    .unwrap();
    let cut = subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(0., 0., 2.),
        Vec3::new(0.25, 0., 1.),
        t,
    )
    .unwrap();
    for (p, expected) in [
        (Point3::new(0., 0., 2.), PointLocation::Inside),
        (Point3::new(1., 0., 1.75), PointLocation::Inside),
        (Point3::new(2., 0., 1.5), PointLocation::Boundary),
        (Point3::new(0., 2., 2.), PointLocation::Boundary),
        (Point3::new(2.1, 0., 1.5), PointLocation::Outside),
        (Point3::new(1., 0., 0.), PointLocation::Boundary),
    ] {
        assert_eq!(classify_point_in_solid(&cut.solid, p, t).unwrap(), expected);
    }
}
