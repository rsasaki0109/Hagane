use hagane::*;
use std::f64::consts::PI;
#[test]
fn exact_cap_topology_volume_and_mesh_follow_the_ellipse() {
    let t = GeometryTolerance::default();
    for slope in [-0.25, 0., 0.25] {
        let s = ellipse_planar_demo_solid(2., 4., slope, t).unwrap();
        s.validate(t.absolute()).unwrap();
        assert_eq!(s.shell.faces.len(), 4);
        assert!((s.volume().unwrap() - PI * 4. * 2.).abs() < 1e-11);
        let mesh = s.tessellate(0.01, t.absolute()).unwrap();
        assert!((mesh.signed_volume() - s.volume().unwrap()).abs() < 0.2);
        let mut uses = std::collections::BTreeMap::new();
        let key = |p: Point3| {
            [
                (p.x * 1e9).round() as i64,
                (p.y * 1e9).round() as i64,
                (p.z * 1e9).round() as i64,
            ]
        };
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
        for (p, expected) in [
            (Point3::new(0., 0., -1.), PointLocation::Inside),
            (Point3::new(0., 0., 0.), PointLocation::Boundary),
            (Point3::new(0., 0., 1.), PointLocation::Outside),
            (Point3::new(1., 0., -slope), PointLocation::Boundary),
            (Point3::new(0., 2., 0.), PointLocation::Boundary),
            (Point3::new(0., 2.1, 0.), PointLocation::Outside),
        ] {
            assert_eq!(classify_point_in_solid(&s, p, t).unwrap(), expected);
        }
    }
}
#[test]
fn analytic_crossings_preserve_line_and_ellipse_edge_parameters() {
    for (scale, epsilon) in [(1., 1e-8), (1e-6, 1e-14)] {
        let t = GeometryTolerance::new(epsilon, 1e-10, 0.).unwrap();
        let s = ellipse_planar_demo_solid(2. * scale, 4. * scale, 0.25, t).unwrap();
        let cap = &s.shell.faces[3];
        let Surface::Plane { origin, u, v } = cap.surface else {
            panic!("plane")
        };
        for angle in [0., 0.37] {
            let transform = Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap();
            let placed = s.transformed(transform, t.absolute()).unwrap();
            for sign in [-1., 1.] {
                let a = transform.point(origin - u * (3. * scale) + v * scale);
                let d = transform.vector(u * (2. * scale * sign));
                let result = clip_line_to_planar_face(&placed, 3, a, d, t).unwrap();
                assert_eq!(result.events.len(), 2);
                assert_eq!(result.intervals.len(), 1);
                let root = 2. * (1.0625f64).sqrt() * (0.75f64).sqrt();
                let expected = if sign > 0. {
                    [(3. - root) / 2., (3. + root) / 2.]
                } else {
                    [-(3. + root) / 2., -(3. - root) / 2.]
                };
                for (p, e) in result.events.iter().zip(expected) {
                    assert!((p.parameter - e).abs() < 1e-10);
                    assert!(
                        (placed.edges[p.edge].curve.evaluate(p.edge_parameter) - p.point).norm()
                            < epsilon
                    );
                }
            }
        }
        for y in [2. * scale, (2. + 1e-9) * scale] {
            assert!(matches!(
                clip_line_to_planar_face(&s, 3, origin - u * (3. * scale) + v * y, u, t),
                Err(Error::Unsupported(_))
            ));
        }
        assert!(clip_line_to_planar_face(&s, 3, origin - u * (3. * scale), u, t).is_err());
        assert!(clip_line_to_planar_face(&s, 3, origin + Vec3::new(0., 0., scale), u, t).is_err());
        assert!(clip_line_to_planar_face(&s, 3, Point3::new(f64::NAN, 0., 0.), u, t).is_err());
        assert!(clip_line_to_planar_face(
            &s,
            3,
            origin - u * (3. * scale) + v * (3. * scale),
            u,
            t
        )
        .unwrap()
        .events
        .is_empty());
    }
}
#[test]
fn malformed_and_mixed_ellipse_domains_are_rejected_before_shortcuts() {
    let t = GeometryTolerance::default();
    let mut s = ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    let PCurve::EllipseArc { ref mut cosine, .. } = s.shell.faces[3].wires[0].coedges[0].pcurve
    else {
        panic!("ellipse")
    };
    cosine[0] += 1.;
    assert!(s.validate(t.absolute()).is_err());
    assert!(classify_point_in_solid(&s, Point3::new(100., 0., 0.), t).is_err());
    for (r, h, slope) in [
        (0., 4., 0.25),
        (2., 0., 0.25),
        (2., 4., 1.),
        (2., 4., f64::NAN),
    ] {
        assert!(ellipse_planar_demo_solid(r, h, slope, t).is_err());
    }
}
#[test]
fn ellipse_and_polygon_faces_intersect_with_shared_parameter_pcurves() {
    let t = GeometryTolerance::default();
    let ellipse = ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    let box_solid = make_box(
        BoxSpec {
            min: Point3::new(0., -3., -3.),
            size: Vec3::new(1., 6., 6.),
        },
        t.absolute(),
    )
    .unwrap();
    let index = box_solid
        .shell
        .faces
        .iter()
        .position(|f| {
            let Surface::Plane { origin, u, v } = f.surface else {
                return false;
            };
            origin.x == 0. && u.cross(v).x.abs() > 0.9
        })
        .unwrap();
    let PlanarFacesIntersection::Segments(segments) =
        intersect_planar_faces(&ellipse, 3, &box_solid, index, t).unwrap()
    else {
        panic!("expected trimmed segment")
    };
    assert_eq!(segments.len(), 1);
    let s = &segments[0];
    assert!((s.parameter_range[1] - s.parameter_range[0] - 4.).abs() < 1e-12);
    for parameter in [0., 0.37, 1.] {
        let point = s.curve.evaluate(parameter);
        let a = s.first.evaluate(parameter);
        let b = s.second.evaluate(parameter);
        assert!((point - ellipse.shell.faces[3].surface.evaluate(a[0], a[1])).norm() < 1e-12);
        assert!((point - box_solid.shell.faces[index].surface.evaluate(b[0], b[1])).norm() < 1e-12);
    }
}
#[test]
fn malformed_mixed_wires_and_duplicate_holes_are_rejected() {
    let t = GeometryTolerance::default();
    let solid = ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    let mut mixed = solid.clone();
    mixed.shell.faces[3].wires[0].coedges[1].pcurve = PCurve::Affine {
        origin: [0., 0.],
        direction: [1., 1.],
    };
    assert!(matches!(
        mixed.validate(t.absolute()),
        Err(Error::InvalidTopology(_))
    ));
    let mut hole = solid;
    let wire = hole.shell.faces[3].wires[0].clone();
    hole.shell.faces[3].wires.push(wire);
    assert!(matches!(
        hole.validate(t.absolute()),
        Err(Error::InvalidTopology(_))
    ));
}
#[test]
fn almost_coincident_half_conics_do_not_become_a_wrong_success() {
    let t = GeometryTolerance::default();
    let mut s = ellipse_planar_demo_solid(2., 4., 0.25, t).unwrap();
    let Surface::Plane { u, .. } = s.shell.faces[3].surface else {
        panic!("plane")
    };
    let coedge = &mut s.shell.faces[3].wires[0].coedges[1];
    let edge = coedge.edge;
    let PCurve::EllipseArc { ref mut cosine, .. } = coedge.pcurve else {
        panic!("ellipse")
    };
    cosine[0] += 1e-10;
    let Curve::EllipseArc { ref mut cosine, .. } = s.edges[edge].curve else {
        panic!("ellipse")
    };
    *cosine = *cosine + u * 1e-10;
    assert!(matches!(
        s.validate(t.absolute()),
        Err(Error::Unsupported(_))
    ));
    assert!(classify_point_in_solid(&s, Point3::new(100., 0., 0.), t).is_err());
}
