use hagane::*;
#[test]
fn actual_rational_circle_closed_brep_and_same_parameter_geometry() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = source.through_xy_circle([40., 30.], 10., tol).unwrap();
    body.validate(tol).unwrap();
    let b = body.brep();
    assert_eq!(
        (b.vertices.len(), b.edges.len(), b.shell.faces.len()),
        (16, 24, 10)
    );
    let mut incidence = [[0usize; 2]; 24];
    for (face_id, f) in b.shell.faces.iter().enumerate() {
        for wire in &f.wires {
            for c in &wire.coedges {
                incidence[c.edge][usize::from(c.forward == (f.orientation == 1))] += 1;
                let Curve::Nurbs(curve) = &b.edges[c.edge].curve else {
                    panic!()
                };
                for k in 0..=32 {
                    let [a, z] = curve.domain();
                    let t = a + (z - a) * k as f64 / 32.;
                    let uv = c.pcurve.try_evaluate(t).unwrap();
                    let actual = curve.evaluate(t).unwrap();
                    assert!(
                        (actual - f.surface.try_evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-10
                    );
                    if c.edge >= 12 && c.edge < 20 {
                        assert!(((actual.x - 40.).hypot(actual.y - 30.) - 10.).abs() < 1e-10);
                        if face_id == 1 {
                            assert!(
                                (actual.z
                                    - (20. + 120. * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1])))
                                    .abs()
                                    < 1e-10
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(incidence.iter().all(|u| *u == [1, 1]));
    for offset in [12, 16] {
        for i in 0..4 {
            let Curve::Nurbs(a) = &b.edges[offset + i].curve else {
                panic!()
            };
            let Curve::Nurbs(next) = &b.edges[offset + (i + 1) % 4].curve else {
                panic!()
            };
            assert_eq!(a.evaluate(1.).unwrap(), next.evaluate(0.).unwrap());
        }
    }

    for edge in &b.edges[12..20] {
        let Curve::Nurbs(c) = &edge.curve else {
            panic!()
        };
        assert_eq!(c.degree(), 8);
        assert!(c.weights().iter().any(|w| *w != 1.));
    }
    for f in &b.shell.faces[6..] {
        let Surface::Nurbs(s) = &f.surface else {
            panic!()
        };
        assert_eq!(s.degrees(), [8, 1]);
        let p = s.evaluate(0.5, 0.5).unwrap();
        let n = s.normal(0.5, 0.5).unwrap() * f.orientation as f64;
        assert!(n.dot(Vec3::new(p.x - 40., p.y - 30., 0.)) < 0.);
    }
    assert!(b.validate(tol).is_err());
}
#[test]
fn trimmed_placed_signed_source_preserves_local_physical_circle() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(Transform::translation(Vec3::new(3., -2., 5.)).unwrap(), tol)
        .unwrap();
    let body = NurbsGraphCircularHoledSolid::new(&source, [40., 30.], 10., tol).unwrap();
    body.validate(tol).unwrap();
    for e in &body.brep().edges[16..20] {
        let Curve::Nurbs(c) = &e.curve else { panic!() };
        let p = c.evaluate(0.37).unwrap();
        assert!(((p.x - 43.).hypot(p.y - 28.) - 10.).abs() < 1e-10);
    }
    assert_eq!(
        body.source().placement().point(Point3::new(40., 30., 0.)),
        Point3::new(43., 28., 5.)
    );
}
#[test]
fn rejects_invalid_clearance_and_every_public_mutation() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., t).unwrap();
    for (c, r) in [
        ([40., 30.], 0.),
        ([40., 30.], -1.),
        ([40., 30.], f64::NAN),
        ([f64::NAN, 30.], 1.),
        ([10., 30.], 10.),
        ([40., 30.], 30.),
        ([40., 30.], 1e-12),
    ] {
        assert!(source.through_xy_circle(c, r, t).is_err());
    }
    assert!(source
        .through_xy_circle([40., 30.], 10., Tolerance::new(1e-20).unwrap())
        .is_err());
    let body = source.through_xy_circle([40., 30.], 10., t).unwrap();
    let mut changed = body.clone();
    changed.solid.vertices[8].point.z = f64::EPSILON;
    assert!(changed.validate(t).is_err());
    assert!(changed.bounds().is_err());
    let mut changed = body.clone();
    changed.solid.shell.faces[1].wires[1].coedges[0].forward = true;
    assert!(changed.validate(t).is_err());
    let mut changed = body;
    let Curve::Nurbs(c) = &changed.solid.edges[16].curve else {
        panic!()
    };
    let mut weights = c.weights().to_vec();
    weights[4] = weights[4].next_up();
    changed.solid.edges[16].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            c.degree(),
            c.knots().to_vec(),
            c.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    assert!(changed.validate(t).is_err());
}
