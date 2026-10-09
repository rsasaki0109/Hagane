use hagane::*;
#[test]
fn graph_trim_retains_source_roof_curved_edges_and_exact_integral() {
    let tol = Tolerance::default();
    let original = NurbsGraphSolid::new([80., 60., 24.], 36., tol).unwrap();
    let ranges = [[0.2, 0.8], [0.1, 0.7]];
    let trimmed = original.trimmed_uv(ranges, tol).unwrap();
    trimmed.validate(tol).unwrap();
    assert_eq!(trimmed.source_domain(), ranges);
    let Surface::Nurbs(roof) = &trimmed.solid.shell.faces[1].surface else {
        panic!()
    };
    for i in 0..31 {
        for j in 0..31 {
            let u = 0.2 + 0.6 * i as f64 / 30.;
            let v = 0.1 + 0.6 * j as f64 / 30.;
            let p = roof.evaluate(u, v).unwrap();
            assert!(
                (p - Point3::new(80. * u, 60. * v, 24. + 144. * u * (1. - u) * v * (1. - v)))
                    .norm()
                    < 1e-12
            );
        }
    }
    let antiderivative = |x: f64| x * x / 2. - x * x * x / 3.;
    let expected = 80.
        * 60.
        * (24. * 0.6 * 0.6
            + 144.
                * (antiderivative(0.8) - antiderivative(0.2))
                * (antiderivative(0.7) - antiderivative(0.1)));
    assert!((trimmed.volume().unwrap() - expected).abs() < 1e-10);
    let mesh = trimmed.tessellate_bounded(0.05, 65536, tol).unwrap();
    let mut edges = std::collections::BTreeMap::<[usize; 2], (usize, i32)>::new();
    for t in &mesh.mesh.triangles {
        for i in 0..3 {
            let a = mesh.vertex_nodes[t[i]];
            let b = mesh.vertex_nodes[t[(i + 1) % 3]];
            let e = edges.entry([a.min(b), a.max(b)]).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|e| *e == (2, 0)));
    assert!(mesh.error_bounds.iter().all(|e| *e <= 0.05));
}
#[test]
fn graph_trim_nested_and_placed_are_canonical_and_invalid_cuts_fail() {
    let tol = Tolerance::default();
    let original = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let t = Transform::translation(Vec3::new(12., -7., 4.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
        .unwrap();
    let ranges = [[0.3, 0.7], [0.2, 0.6]];
    let nested = original
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)
        .unwrap()
        .transformed(t, tol)
        .unwrap()
        .trimmed_uv(ranges, tol)
        .unwrap();
    let direct = original
        .trimmed_uv(ranges, tol)
        .unwrap()
        .transformed(t, tol)
        .unwrap();
    assert_eq!(nested.placement(), direct.placement());
    assert_eq!(nested.volume().unwrap(), direct.volume().unwrap());
    for (a, b) in nested.solid.vertices.iter().zip(&direct.solid.vertices) {
        assert_eq!(a.point, b.point);
    }
    assert!(nested.trimmed_uv([[0.2, 0.7], [0.2, 0.6]], tol).is_err());
    for r in [
        [[0.5, 0.5], [0., 1.]],
        [[f64::NAN, 0.5], [0., 1.]],
        [[0., 1.1], [0., 1.]],
    ] {
        assert!(original.trimmed_uv(r, tol).is_err());
    }
    direct.tessellate_bounded(0.01, 65536, tol).unwrap();
}

#[test]
fn curved_wall_and_roof_bounds_cover_independent_parameter_samples() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([8., 6., 2.], 12., tol)
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], tol)
        .unwrap();
    let display = graph.tessellate_bounded(0.01, 65536, tol).unwrap();
    let mut wall_deviation: f64 = 0.;
    for (index, tri) in display.mesh.triangles.iter().enumerate() {
        let face = display.mesh.face_ids[index];
        let Surface::Nurbs(surface) = &graph.solid.shell.faces[face].surface else {
            panic!()
        };
        for bary in [[0.2, 0.3, 0.5], [0.5, 0.25, 0.25], [0.1, 0.8, 0.1]] {
            let mut uv = [0.; 2];
            let mut point = Point3::new(0., 0., 0.);
            for k in 0..3 {
                for (axis, value) in uv.iter_mut().enumerate() {
                    *value += bary[k] * display.vertex_uv[tri[k]][axis];
                }
                point = point + display.mesh.positions[tri[k]] * bary[k];
            }
            let distance = (surface.evaluate(uv[0], uv[1]).unwrap() - point).norm();
            assert!(
                distance <= display.error_bounds[index] + 1e-13,
                "face {face} distance {distance} bound {}",
                display.error_bounds[index]
            );
            if face >= 2 {
                wall_deviation = wall_deviation.max(distance);
            }
        }
    }
    assert!(wall_deviation > 1e-5);
}
