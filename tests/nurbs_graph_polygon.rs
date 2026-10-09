use hagane::*;
fn polygon() -> Vec<[f64; 2]> {
    vec![[0.1, 0.2], [0.8, 0.1], [0.9, 0.7], [0.3, 0.9]]
}
#[test]
fn closed_curved_caps_and_walls() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = NurbsGraphPolygonSolid::new(&source, polygon(), tol).unwrap();
    assert_eq!(
        (
            body.solid.vertices.len(),
            body.solid.edges.len(),
            body.solid.shell.faces.len()
        ),
        (8, 12, 6)
    );
    let mut incidence = [[0i32; 2]; 12];
    for (fi, face) in body.solid.shell.faces.iter().enumerate() {
        for c in &face.wires[0].coedges {
            incidence[c.edge][0] += 1;
            incidence[c.edge][1] += face.orientation as i32 * if c.forward { 1 } else { -1 };
            let e = &body.solid.edges[c.edge];
            for k in 0..101 {
                let t = k as f64 / 100.;
                let uv = c.pcurve.evaluate(t);
                let p = e.curve.try_evaluate(t).unwrap();
                assert!((p - face.surface.try_evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-10);
                if fi == 1 {
                    let want = 20. + 120. * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1]);
                    assert!((p.z - want).abs() < 1e-10);
                }
            }
        }
    }
    assert!(incidence.iter().all(|i| *i == [2, 0]));
    assert!(body.brep().volume().is_err());
}
#[test]
fn placement_trim_signed_and_corruption() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], -2., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap(),
            tol,
        )
        .unwrap();
    let mut body =
        NurbsGraphPolygonSolid::new(&source, vec![[0.2, 0.2], [0.8, 0.2], [0.6, 0.8]], tol)
            .unwrap();
    body.validate(tol).unwrap();
    body.solid.vertices[0].point.x += tol.linear / 10.;
    assert!(body.validate(tol).is_err());
    assert!(body.bounds().is_err());
}
#[test]
fn invalid_polygons() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let mut cw = polygon();
    cw.reverse();
    for p in [
        cw,
        vec![[0.2, 0.2], [0.8, 0.2], [0.5, 0.3], [0.5, 0.8]],
        vec![[0.2, 0.2], [0.5, 0.2], [0.8, 0.2]],
        vec![[0.2, 0.2], [0.2, 0.2], [0.8, 0.8]],
        vec![[f64::NAN, 0.2], [0.8, 0.2], [0.4, 0.8]],
        vec![[0.2, 0.2], [1.1, 0.2], [0.5, 0.8]],
    ] {
        assert!(NurbsGraphPolygonSolid::new(&source, p, tol).is_err());
    }
}
