use hagane::*;
use std::f64::consts::PI;
#[test]
fn half_ellipse_cap_keeps_exact_volume_and_material() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        for slope in [-0.25, 0., 0.25] {
            let s = half_ellipse_planar_demo_solid(2. * scale, 4. * scale, slope, t).unwrap();
            s.validate(t.absolute()).unwrap();
            assert_eq!(s.shell.faces.len(), 4);
            assert!((s.volume().unwrap() - PI * 4. * scale.powi(3)).abs() < scale.powi(3) * 1e-11);
            for (p, expected) in [
                (Point3::new(0., 1., -1.), PointLocation::Inside),
                (Point3::new(0., 1., 0.), PointLocation::Boundary),
                (Point3::new(0., 0., -1.), PointLocation::Boundary),
                (Point3::new(0., 2., 0.), PointLocation::Boundary),
                (Point3::new(0., -0.1, -1.), PointLocation::Outside),
                (Point3::new(0., 1., 1.), PointLocation::Outside),
            ] {
                assert_eq!(classify_point_in_solid(&s, p * scale, t).unwrap(), expected);
            }
            let mesh = s.tessellate(scale * 0.01, t.absolute()).unwrap();
            assert!((mesh.signed_volume() - s.volume().unwrap()).abs() < 0.1 * scale.powi(3));
            let key = |p: Point3| {
                [
                    (p.x / scale * 1e9).round() as i64,
                    (p.y / scale * 1e9).round() as i64,
                    (p.z / scale * 1e9).round() as i64,
                ]
            };
            let mut uses = std::collections::BTreeMap::new();
            for tri in &mesh.triangles {
                for i in 0..3 {
                    let a = key(mesh.positions[tri[i]]);
                    let b = key(mesh.positions[tri[(i + 1) % 3]]);
                    let (e, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                    let entry = uses.entry(e).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += sign;
                }
            }
            assert!(uses.values().all(|v| *v == (2, 0)));
        }
    }
}
#[test]
fn ellipse_and_diameter_crossings_keep_their_own_parameters() {
    let t = GeometryTolerance::default();
    let s = half_ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    let Surface::Plane { origin, u, v } = s.shell.faces[3].surface else {
        panic!("cap")
    };
    for angle in [0., 0.37] {
        let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
        let placed = s.transformed(transform, t.absolute()).unwrap();
        for sign in [-1., 1.] {
            let clip = clip_line_to_planar_face(
                &placed,
                3,
                transform.point(origin - v * 3.),
                transform.vector(v * (2. * sign)),
                t,
            )
            .unwrap();
            assert_eq!(clip.events.len(), 2);
            assert_eq!(clip.intervals.len(), 1);
            let expected = if sign > 0. { [1.5, 2.5] } else { [-2.5, -1.5] };
            for (p, expected) in clip.events.iter().zip(expected) {
                assert!((p.parameter - expected).abs() < 1e-12);
                assert!(
                    (placed.edges[p.edge].curve.evaluate(p.edge_parameter) - p.point).norm()
                        < 1e-10
                );
            }
            let arc = clip.events.iter().find(|p| p.coedge == 0).unwrap();
            let line = clip.events.iter().find(|p| p.coedge == 1).unwrap();
            assert!((arc.edge_parameter - PI / 2.).abs() < 1e-12);
            assert!((line.edge_parameter - 0.5).abs() < 1e-12);
        }
    }
    for y in [-1., 3.] {
        assert!(
            clip_line_to_planar_face(&s, 3, origin - u * 3. + v * y, u, t)
                .unwrap()
                .events
                .is_empty()
        );
    }
    for y in [0., 1e-9, 2., 2. + 1e-9] {
        assert!(clip_line_to_planar_face(&s, 3, origin - u * 3. + v * y, u, t).is_err());
    }
}

#[test]
fn malformed_and_out_of_domain_half_ellipse_trims_fail() {
    let t = GeometryTolerance::default();
    let source = half_ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    for variant in 0..4 {
        let mut solid = source.clone();
        let face = &mut solid.shell.faces[3];
        match variant {
            0 => {
                let PCurve::Affine { origin, .. } = &mut face.wires[0].coedges[1].pcurve else {
                    panic!("diameter")
                };
                origin[1] += 0.01;
            }
            1 => {
                let PCurve::EllipseArc { sweep, .. } = &mut face.wires[0].coedges[0].pcurve else {
                    panic!("ellipse")
                };
                *sweep = PI * 0.75;
            }
            2 => face.wires.push(face.wires[0].clone()),
            _ => face.wires[0].coedges.swap(0, 1),
        }
        assert!(solid.validate(t.absolute()).is_err());
        assert!(clip_line_to_planar_face(
            &solid,
            3,
            Point3::new(100., 100., 100.),
            Vec3::new(1., 0., 0.),
            t
        )
        .is_err());
    }
    for (radius, height, slope) in [
        (0., 4., 0.),
        (2., -4., 0.),
        (2., 4., 1.),
        (2., 4., f64::NAN),
    ] {
        assert!(half_ellipse_planar_demo_solid(radius, height, slope, t).is_err());
    }
}
