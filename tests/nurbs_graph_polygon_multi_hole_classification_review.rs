use hagane::*;
fn rect(a: [f64; 2], b: [f64; 2]) -> Vec<[f64; 2]> {
    vec![a, [b[0], a[1]], b, [a[0], b[1]]]
}
fn holes() -> Vec<Vec<[f64; 2]>> {
    vec![
        rect([0.125, 0.125], [0.375, 0.375]),
        rect([0.625, 0.125], [0.875, 0.3125]),
        rect([0.5625, 0.625], [0.8125, 0.875]),
        rect([0.125, 0.6875], [0.3125, 0.875]),
    ]
}
fn band(t: f64) -> GeometryTolerance {
    GeometryTolerance::new(t, 1e-10, 0.).unwrap()
}
fn body(source: &NurbsGraphSolid, count: usize, t: Tolerance) -> NurbsGraphPolygonMultiHoledSolid {
    let [u, v] = source.source_domain();
    let outer = NurbsGraphPolygonSolid::new(source, rect([u[0], v[0]], [u[1], v[1]]), t).unwrap();
    NurbsGraphPolygonMultiHoledSolid::new(&outer, holes()[..count].to_vec(), t).unwrap()
}
#[test]
fn every_opening_excludes_base_roof_and_core_with_material_between_them() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], -0.5, t).unwrap();
    for count in 2..=4 {
        let body = body(&source, count, t);
        for h in &holes()[..count] {
            let u = (h[0][0] + h[2][0]) / 2.;
            let v = (h[0][1] + h[2][1]) / 2.;
            let roof = 1. - 2. * u * (1. - u) * v * (1. - v);
            for z in [0., roof / 2., roof] {
                assert_eq!(
                    body.classify_point(Point3::new(u, v, z), band(1e-8))
                        .unwrap(),
                    PointLocation::Outside
                );
            }
        }
        assert_eq!(
            body.classify_point(Point3::new(0.5, 0.5, 0.4), band(1e-6))
                .unwrap(),
            PointLocation::Inside
        );
    }
}
#[test]
fn each_inner_wall_and_corner_obeys_actual_euclidean_band() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let body = body(&source, 4, t);
    let epsilon = 1e-5;
    for h in holes() {
        for delta in [-0.6, 0., 0.6] {
            let p = Point3::new(h[0][0] + delta * epsilon, (h[0][1] + h[2][1]) / 2., 0.5);
            assert_eq!(
                body.classify_point(p, band(epsilon)).unwrap(),
                PointLocation::Boundary
            );
        }
        for (delta, expected) in [(0.6, PointLocation::Boundary), (0.8, PointLocation::Inside)] {
            let p = Point3::new(h[0][0] - delta * epsilon, h[0][1] - delta * epsilon, 0.5);
            assert_eq!(body.classify_point(p, band(epsilon)).unwrap(), expected);
        }
    }
}
#[test]
fn signed_trimmed_rigid_source_queries_preserve_original_uv_and_small_band() {
    let t = Tolerance::default();
    let frame = Transform::translation(Vec3::new(12., -8., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let source = NurbsGraphSolid::new([20., 12., 3.], -2., t)
        .unwrap()
        .trimmed_uv([[0.0625, 0.9375], [0.0625, 0.9375]], t)
        .unwrap()
        .transformed(frame, t)
        .unwrap();
    let body = body(&source, 4, t);
    for h in holes() {
        let u = (h[0][0] + h[2][0]) / 2.;
        let v = (h[0][1] + h[2][1]) / 2.;
        let roof = 3. - 8. * u * (1. - u) * v * (1. - v);
        assert_eq!(
            body.classify_point(frame.point(Point3::new(20. * u, 12. * v, roof)), band(1e-8))
                .unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        body.classify_point(frame.point(Point3::new(10., 6., 1.)), band(1e-6))
            .unwrap(),
        PointLocation::Inside
    );
}
#[test]
fn relative_scaling_canonical_mutation_and_world_arithmetic_are_checked() {
    for scale in [1e-6, 1., 100.] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([scale; 3], 0., t).unwrap();
        let mut body = body(&source, 2, t);
        let relative = GeometryTolerance::new(scale * 1e-8, 1e-10, 1e-5).unwrap();
        assert_eq!(
            body.classify_point(
                Point3::new(scale * (0.125 + 5e-6), scale * 0.25, scale * 0.5),
                relative
            )
            .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            body.classify_point(
                Point3::new(scale * (0.125 + 1e-4), scale * 0.25, scale * 0.5),
                relative
            )
            .unwrap(),
            PointLocation::Outside
        );
        assert!(body
            .classify_point(Point3::new(1e15, 1e15, 1e15), band(scale * 1e-8))
            .is_err());
        body.solid.shell.faces[0].wires[2].coedges[0].edge = usize::MAX;
        assert!(body
            .classify_point(
                Point3::new(scale * 0.75, scale * 0.25, scale * 0.5),
                relative
            )
            .is_err());
    }
}

#[test]
fn actual_curve_mutation_and_nonfinite_queries_cannot_be_hidden_by_early_void_proof() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let mut body = body(&source, 4, t);
    for point in [
        Point3::new(f64::NAN, 0.25, 0.5),
        Point3::new(0.25, f64::INFINITY, 0.5),
    ] {
        assert!(body.classify_point(point, band(1e-6)).is_err());
    }
    let Curve::Nurbs(curve) = &body.solid.edges[4].curve else {
        panic!()
    };
    let mut controls = curve.control_points().to_vec();
    controls[1].z += t.linear / 100.;
    body.solid.edges[4].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            controls,
            curve.weights().to_vec(),
        )
        .unwrap(),
    ));
    assert!(body
        .classify_point(Point3::new(0.25, 0.25, 0.5), band(1e-6))
        .is_err());
}
