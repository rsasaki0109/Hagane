use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn body(s: f64, posed: bool) -> NurbsTorusSolid {
    let axes = if posed {
        Transform::rotation(Vec3::new(1., 2., 3.), 0.37)
            .unwrap()
            .axes()
    } else {
        [
            Vec3::new(0., -1., 0.),
            Vec3::new(0., 0., -1.),
            Vec3::new(1., 0., 0.),
        ]
    };
    let f = Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, axes, policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn geometry(section: &NurbsTorusHorizontalSection, s: f64) {
    let source = section.source();
    let c = section.cut_height();
    let expected = [
        16. * s + (16. * s * s - c * c).sqrt(),
        16. * s - (16. * s * s - c * c).sqrt(),
    ];
    assert_eq!(section.vertices().len(), 8);
    assert_eq!(section.edges().len(), 8);
    assert_eq!(section.loops().len(), 2);
    let mut used = [0; 8];
    for (index, wire) in section.loops().iter().enumerate() {
        assert_eq!(wire.coedges.len(), 4);
        let mut winding = 0.;
        for (i, coedge) in wire.coedges.iter().enumerate() {
            assert_eq!(coedge.edge, index * 4 + i);
            assert!(coedge.forward);
            let edge = &section.edges()[coedge.edge];
            let next = &section.edges()[wire.coedges[(i + 1) % 4].edge];
            assert_eq!(edge.vertices[1], next.vertices[0]);
            assert_eq!(edge.vertices, [index * 4 + i, index * 4 + (i + 1) % 4]);
            for k in 0..=32 {
                let t = k as f64 / 32.;
                let point = edge.curve.try_evaluate(t).unwrap();
                let local = source.frame().local_point(point);
                assert!((local.z - c).abs() < 1e-8 * s);
                assert!((local.x.hypot(local.y) - expected[index]).abs() < 1e-8 * s);
                let uv = coedge.pcurve.try_evaluate(t).unwrap();
                let cap = section.plane().try_evaluate(uv[0], uv[1]).unwrap();
                assert!((cap - point).norm() < 1e-8 * s);
                if k < 32 {
                    let q = source
                        .frame()
                        .local_point(edge.curve.try_evaluate((k + 1) as f64 / 32.).unwrap());
                    winding += local.x * q.y - local.y * q.x;
                }
            }
            assert!(
                (edge.curve.try_evaluate(0.).unwrap() - section.vertices()[edge.vertices[0]].point)
                    .norm()
                    < 1e-8 * s
            );
            assert!(
                (edge.curve.try_evaluate(1.).unwrap() - section.vertices()[edge.vertices[1]].point)
                    .norm()
                    < 1e-8 * s
            );
        }
        assert!(
            winding > 0.,
            "both section loops follow original positive major parameter"
        );
    }
    for attach in section.source_uses() {
        used[attach.edge] += 1;
        let face = &source.solid().shell.faces[attach.face];
        let edge = &section.edges()[attach.edge];
        for i in 0..=32 {
            let t = i as f64 / 32.;
            let uv = attach.pcurve.try_evaluate(t).unwrap();
            assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
            let actual = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
            assert!((actual - edge.curve.try_evaluate(t).unwrap()).norm() < 1e-8 * s);
        }
    }
    assert!(used.iter().all(|&n| n == if c == 0. { 2 } else { 1 }));
}
#[test]
fn two_actual_closed_rational_loops_match_independent_radial_plane_and_source_uv_equations() {
    for s in [1e-4, 1., 10.] {
        for posed in [false, true] {
            let source = body(s, posed);
            let p = policy(s);
            let before = format!("{:?}", source.solid());
            let step = source.export_step_mm(p).unwrap();
            for c in [0., 1.25, -1.25, 3.2, -3.2] {
                let section = source.section_by_local_z(c * s, p).unwrap();
                section.validate(p).unwrap();
                geometry(&section, s);
            }
            assert_eq!(before, format!("{:?}", source.solid()));
            assert_eq!(step, source.export_step_mm(p).unwrap());
        }
    }
}
#[test]
fn literal_zero_matches_original_rational_edge_support_and_both_adjacent_face_attachments() {
    let source = body(1., true);
    let p = policy(1.);
    let section = source.section_by_local_z(0., p).unwrap();
    let split = source.split_by_local_z(0., p).unwrap();
    let cap = split
        .lower
        .solid()
        .shell
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    for (edge_id, edge) in section.edges().iter().enumerate() {
        let major = edge_id % 4;
        let minor = if edge_id < 4 { 0 } else { 2 };
        let original = &source.solid().edges[major * 4 + minor];
        let (Curve::Nurbs(actual), Curve::Nurbs(expected)) = (&edge.curve, &original.curve) else {
            panic!("actual rational edges required")
        };
        assert_eq!(actual.degree(), expected.degree());
        assert_eq!(actual.knots(), expected.knots());
        assert_eq!(actual.weights(), expected.weights());
        for (a, b) in actual
            .control_points()
            .iter()
            .zip(expected.control_points())
        {
            assert!((*a - *b).norm() < 1e-12);
        }

        let retained = cap
            .wires
            .iter()
            .flat_map(|w| &w.coedges)
            .map(|c| &split.lower.solid().edges[c.edge])
            .find(|other| {
                (other.curve.try_evaluate(0.).unwrap() - edge.curve.try_evaluate(0.).unwrap())
                    .norm()
                    < 1e-12
                    && (other.curve.try_evaluate(0.5).unwrap()
                        - edge.curve.try_evaluate(0.5).unwrap())
                    .norm()
                        < 1e-12
            })
            .expect("actual split rim support");
        assert_eq!(format!("{:?}", edge.curve), format!("{:?}", retained.curve));
        let faces: Vec<_> = section
            .source_uses()
            .iter()
            .filter(|u| u.edge == edge_id)
            .map(|u| u.face)
            .collect();
        assert_eq!(faces.len(), 2);
        assert_ne!(faces[0], faces[1]);
        for id in faces {
            assert_eq!(id / 4, major);
        }
    }
    geometry(&section, 1.);
}
#[test]
fn tangency_near_zero_nonfinite_and_full_source_policy_refuse_without_snapping_then_recover() {
    let source = body(1., false);
    let p = policy(1.);
    let before = format!("{:?}", source.solid());
    let step = source.export_step_mm(p).unwrap();
    for c in [
        4.,
        -4.,
        5.,
        4. - 1e-10,
        -4. + 1e-10,
        1e-12,
        -1e-12,
        f64::NAN,
        f64::INFINITY,
        1e12,
    ] {
        assert!(source.section_by_local_z(c, p).is_err(), "accepted {c}");
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(source.section_by_local_z(0.001, relative).is_err());
    geometry(&source.section_by_local_z(0., p).unwrap(), 1.);
    geometry(&source.section_by_local_z(1.25, p).unwrap(), 1.);
    assert_eq!(before, format!("{:?}", source.solid()));
    assert_eq!(step, source.export_step_mm(p).unwrap());
}
