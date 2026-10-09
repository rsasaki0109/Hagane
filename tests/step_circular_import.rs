use hagane::*;
use std::f64::consts::{PI, TAU};
const CYLINDER: &str = include_str!("../docs/step-cylinder-metres.step");
#[test]
fn independent_metre_cylinder_validates_two_seam_pcurves_and_rotated_wire_start() {
    let t = Tolerance::default();
    let s = import_step_mm(CYLINDER, t).unwrap();
    let c = certify_circular_prism(&s, t).unwrap();
    assert_eq!(c.outer_radius, 2.);
    assert_eq!(c.inner_radius, None);
    assert_eq!(c.height, 3.);
    assert!((s.volume().unwrap() - 12. * PI).abs() < 1e-12);
    assert_eq!(s.bounds().min, Point3::new(-2., -2., 0.));
    assert_eq!(s.bounds().max, Point3::new(2., 2., 3.));
    assert_eq!(
        (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
        (2, 3, 3)
    );
    assert!(import_step_planar_mm(CYLINDER, t).is_err());
    let wall = &s.shell.faces[0];
    let coedges = &wall.wires[0].coedges;
    assert_eq!(coedges[1].edge, coedges[3].edge);
    assert_ne!(coedges[1].forward, coedges[3].forward);
    assert_eq!(coedges[1].pcurve.evaluate(0.), [TAU, 0.]);
    assert_eq!(coedges[3].pcurve.evaluate(0.), [0., 0.]);
    let restored = import_step_mm(&export_step_mm(&s, t).unwrap(), t).unwrap();
    assert!((restored.volume().unwrap() - s.volume().unwrap()).abs() < 1e-12);
    let reordered = CYLINDER.replace(
        "#63=SEAM_CURVE('',#34,(#62,#61)",
        "#63=SEAM_CURVE('',#34,(#61,#62)",
    );
    assert!(import_step_mm(&reordered, t).is_ok());
    let reversed = CYLINDER
        .replace(
            "#93=EDGE_LOOP('',(#85,#86,#83,#84))",
            "#93=EDGE_LOOP('',(#84,#83,#86,#85))",
        )
        .replace(
            "#83=ORIENTED_EDGE('',*,*,#71,.T.)",
            "#83=ORIENTED_EDGE('',*,*,#71,.F.)",
        )
        .replace(
            "#84=ORIENTED_EDGE('',*,*,#73,.T.)",
            "#84=ORIENTED_EDGE('',*,*,#73,.F.)",
        )
        .replace(
            "#85=ORIENTED_EDGE('',*,*,#72,.F.)",
            "#85=ORIENTED_EDGE('',*,*,#72,.T.)",
        )
        .replace(
            "#86=ORIENTED_EDGE('',*,*,#73,.F.)",
            "#86=ORIENTED_EDGE('',*,*,#73,.T.)",
        )
        .replace(
            "#103=FACE_OUTER_BOUND('',#93,.T.)",
            "#103=FACE_OUTER_BOUND('',#93,.F.)",
        );
    assert!(import_step_mm(&reversed, t).is_ok());
}
#[test]
fn cylinder_and_tube_round_trips_at_three_scales_with_rigid_placement() {
    for scale in [1e-6, 1., 1000.] {
        let t = Tolerance::new(1e-8 * scale).unwrap();
        let policy = GeometryTolerance::try_from(t).unwrap();
        for tube in [false, true] {
            let base = Point3::new(0., 0., -12. * scale);
            let source = if tube {
                make_tube(
                    TubeSpec {
                        base,
                        outer_radius: 8. * scale,
                        inner_radius: 4. * scale,
                        height: 24. * scale,
                    },
                    t,
                )
                .unwrap()
            } else {
                make_cylinder(
                    CylinderSpec {
                        base,
                        radius: 8. * scale,
                        height: 24. * scale,
                    },
                    t,
                )
                .unwrap()
            };
            let placed = Transform::translation(Vec3::new(20. * scale, -7. * scale, 12. * scale))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap())
                .unwrap();
            for transform in [Transform::IDENTITY, placed] {
                let source = source.transformed(transform, t).unwrap();
                let s = import_step_mm(&export_step_mm(&source, t).unwrap(), t).unwrap();
                assert_eq!(
                    (s.vertices.len(), s.edges.len(), s.shell.faces.len()),
                    if tube { (4, 6, 4) } else { (2, 3, 3) }
                );
                assert!(
                    (s.volume().unwrap() - source.volume().unwrap()).abs()
                        < source.volume().unwrap() * 1e-12
                );
                assert!((s.bounds().min - source.bounds().min).norm() < t.linear);
                assert!((s.bounds().max - source.bounds().max).norm() < t.linear);
                let c = certify_circular_prism(&s, t).unwrap();
                assert!((c.height - 24. * scale).abs() < t.linear);
                assert_eq!(c.inner_radius.is_some(), tube);
                for (p, expected) in [
                    (
                        Point3::new(0., 0., 0.),
                        if tube {
                            PointLocation::Outside
                        } else {
                            PointLocation::Inside
                        },
                    ),
                    (Point3::new(6. * scale, 0., 0.), PointLocation::Inside),
                    (Point3::new(9. * scale, 0., 0.), PointLocation::Outside),
                ] {
                    assert_eq!(
                        classify_point_in_solid(&s, transform.point(p), policy).unwrap(),
                        expected
                    );
                }
                let error = 0.01 * scale;
                let mesh = s.tessellate(error, t).unwrap();
                let allowance = TAU * error * (if tube { 12. } else { 8. }) * 24. * scale.powi(2);
                assert!((mesh.signed_volume() - s.volume().unwrap()).abs() < allowance);
                let local_positions: Vec<_> = mesh
                    .positions
                    .iter()
                    .map(|&p| c.axis.local_point(p))
                    .collect();
                for (tri, &face) in mesh.triangles.iter().zip(&mesh.face_ids) {
                    if matches!(s.shell.faces[face].surface, Surface::Plane { .. }) {
                        continue;
                    }
                    let radius = if s.shell.faces[face].orientation == 1 {
                        c.outer_radius
                    } else {
                        c.inner_radius.unwrap()
                    };
                    for i in 0..3 {
                        let midpoint =
                            (local_positions[tri[i]] + local_positions[tri[(i + 1) % 3]]) * 0.5;
                        assert!(
                            (midpoint.x.hypot(midpoint.y) - radius).abs() <= error + 1e-10 * scale
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn malformed_or_out_of_domain_circular_geometry_is_explicitly_rejected() {
    let t = Tolerance::default();
    for invalid in [
        CYLINDER.replace("(#62,#61)", "(#61,#61)"),
        CYLINDER.replace("(#62,#61)", "(#61)"),
        CYLINDER.replace("#62=PCURVE('',#43", "#62=PCURVE('',#42"),
        CYLINDER.replace("6.283185307179586,0.", "6.283185307,0."),
        CYLINDER.replace("(0.,0.));\n#53", "(0.,0.001));\n#53"),
        CYLINDER.replace(
            "#55=VECTOR('',#54,0.003)",
            "#55=VECTOR('',#54,0.003000000001)",
        ),
        CYLINDER.replace("#55=VECTOR('',#54,0.003)", "#55=VECTOR('',#54,-0.003)"),
        CYLINDER.replace("(0.,4.)", "(0.0001,4.)"),
        CYLINDER.replace(
            "GEOMETRIC_REPRESENTATION_CONTEXT(2)",
            "GEOMETRIC_REPRESENTATION_CONTEXT(3)",
        ),
        CYLINDER.replace(
            "#71=EDGE_CURVE('',#11,#11,#31,.T.)",
            "#71=EDGE_CURVE('',#11,#11,#31,.F.)",
        ),
        CYLINDER.replace("#71=EDGE_CURVE('',#11,#11", "#71=EDGE_CURVE('',#11,#12"),
        CYLINDER.replace("CIRCLE('',#21,0.002)", "CIRCLE('',#21,0.002000000001)"),
        CYLINDER.replace(
            "CYLINDRICAL_SURFACE('',#21,0.002)",
            "CYLINDRICAL_SURFACE('',#21,0.001)",
        ),
        CYLINDER.replace(".CURVE_3D.", ".PCURVE_S1."),
        CYLINDER.replace(
            "#73=EDGE_CURVE('',#11,#12,#63",
            "#73=EDGE_CURVE('',#11,#12,#34",
        ),
        CYLINDER.replace(
            "#123=ADVANCED_FACE('',(#103),#43,.T.)",
            "#123=ADVANCED_FACE('',(#103),#43,.F.)",
        ),
        CYLINDER.replace("CIRCLE('',#21,0.002)", "CIRCLE('',#21,-0.002)"),
        CYLINDER.replace(
            "#1=CARTESIAN_POINT('',(0.002,0.,0.))",
            "#1=CARTESIAN_POINT('',(0.,0.002,0.))",
        ),
    ] {
        assert_ne!(invalid, CYLINDER);
        assert!(
            import_step_mm(&invalid, t).is_err(),
            "accepted invalid circular input: {invalid}"
        );
    }
    let source = make_box(
        BoxSpec {
            min: Point3::new(-4., -4., 0.),
            size: Vec3::new(8., 8., 3.),
        },
        t,
    )
    .unwrap();
    assert!(certify_circular_prism(&source, t).is_err());
    let bored = subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-4., -4., 0.),
            size: Vec3::new(8., 8., 3.),
        },
        CylinderSpec {
            base: Point3::new(0., 0., -1.),
            radius: 1.,
            height: 5.,
        },
        t,
    )
    .unwrap();
    assert!(matches!(
        import_step_mm(&export_step_mm(&bored, t).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn unresolved_height_and_nearly_coincident_tube_walls_fail_without_repair() {
    let t = Tolerance::default();
    let construction = Tolerance::new(t.linear / 100.).unwrap();
    let thin = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., 0.),
            radius: 2.,
            height: 5. * t.linear,
        },
        construction,
    )
    .unwrap();
    assert!(import_step_mm(&export_step_mm(&thin, construction).unwrap(), t).is_err());
    let tube = make_tube(
        TubeSpec {
            base: Point3::new(0., 0., 0.),
            outer_radius: 2.,
            inner_radius: 2. - 5. * t.linear,
            height: 3.,
        },
        construction,
    )
    .unwrap();
    assert!(matches!(
        import_step_mm(&export_step_mm(&tube, construction).unwrap(), t),
        Err(Error::Unsupported(_))
    ));
    let touching = CYLINDER.replace("#32=CIRCLE('',#22,0.002)", "#32=CIRCLE('',#21,0.002)");
    assert!(import_step_mm(&touching, t).is_err());
    for i in (0..CYLINDER.len()).step_by(19) {
        assert!(std::panic::catch_unwind(|| import_step_mm(&CYLINDER[..i], t)).is_ok());
        let mut mutated = CYLINDER.to_string();
        mutated.insert(i, '#');
        assert!(std::panic::catch_unwind(|| import_step_mm(&mutated, t)).is_ok());
    }
}

#[test]
fn public_certificate_rejects_tolerance_sized_boundary_deformations() {
    let t = Tolerance::default();
    let s = import_step_mm(CYLINDER, t).unwrap();
    let mut cap = s.clone();
    let face = cap
        .shell
        .faces
        .iter_mut()
        .find(|f| matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    let PCurve::Circle { center, .. } = &mut face.wires[0].coedges[0].pcurve else {
        panic!("cap must be circular")
    };
    center[0] += t.linear / 10.;
    cap.validate(t).unwrap();
    assert!(certify_circular_prism(&cap, t).is_err());
    let mut line = s.clone();
    let edge = line
        .edges
        .iter_mut()
        .find(|e| matches!(e.curve, Curve::Line { .. }))
        .unwrap();
    let Curve::Line { a, .. } = &mut edge.curve else {
        unreachable!()
    };
    a.x += t.linear / 10.;
    line.validate(t).unwrap();
    assert!(certify_circular_prism(&line, t).is_err());
}
