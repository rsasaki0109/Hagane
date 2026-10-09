use hagane::{KnotSide, NurbsSurface, Point3};
fn patch(scale: f64, weight_scale: f64) -> NurbsSurface {
    let u = vec![2., 2., 2., 6., 6., 6.];
    let v = vec![-3., -3., -3., 5., 5., 5.];
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(i as f64, j as f64, (i * j + i + j) as f64) * scale);
            weights.push((1. + (3 * i + j) as f64 * 0.3) * weight_scale);
        }
    }
    NurbsSurface::new([2, 2], [u, v], [3, 3], points, weights).unwrap()
}
#[test]
fn refinement_preserves_surface_and_partials_with_row_weight_ratios() {
    for (scale, weight_scale) in [(1., 1.), (1., 1e200), (1e-100, 1e200)] {
        let surface = patch(scale, weight_scale);
        let refined = surface
            .insert_knot(0, 3.5, 2)
            .unwrap()
            .insert_knot(1, 1., 2)
            .unwrap();
        assert_eq!(refined.control_counts(), [5, 5]);
        assert_eq!(surface.domain(), refined.domain());
        for i in 0..=20 {
            for j in 0..=20 {
                let u = 2. + i as f64 / 5.;
                let v = -3. + j as f64 * 0.4;
                for side in [KnotSide::Left, KnotSide::Right] {
                    let a = surface.evaluate_with_partials(u, v, [side; 2]).unwrap();
                    let b = refined.evaluate_with_partials(u, v, [side; 2]).unwrap();
                    assert!((a.point - b.point).norm() < scale * 1e-12);
                    assert!((a.du - b.du).norm() < scale * 1e-12);
                    assert!((a.dv - b.dv).norm() < scale * 1e-12);
                }
            }
        }
    }
}
#[test]
fn isocurves_retain_parameter_domains_and_exact_partial() {
    let surface = patch(1., 1.)
        .insert_knot(0, 4., 2)
        .unwrap()
        .insert_knot(1, 0., 2)
        .unwrap();
    for axis in 0..2 {
        for parameter in [
            surface.domain()[axis][0],
            surface.domain()[axis][1],
            if axis == 0 { 4. } else { 0. },
        ] {
            let curve = surface.isocurve(axis, parameter).unwrap();
            assert_eq!(curve.domain(), surface.domain()[1 - axis]);
            assert_eq!(curve.knots(), surface.knots(1 - axis).unwrap());
            for i in 0..=32 {
                let [a, b] = curve.domain();
                let varying = a + (b - a) * i as f64 / 32.;
                let uv = if axis == 0 {
                    [parameter, varying]
                } else {
                    [varying, parameter]
                };
                for side in [KnotSide::Left, KnotSide::Right] {
                    let (p, d) = curve.evaluate_with_derivative(varying, side).unwrap();
                    let e = surface
                        .evaluate_with_partials(uv[0], uv[1], [side; 2])
                        .unwrap();
                    assert!((p - e.point).norm() < 1e-12);
                    assert!((d - if axis == 0 { e.dv } else { e.du }).norm() < 1e-12);
                }
            }
        }
    }
}
#[test]
fn independent_rational_bilinear_isocurve() {
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![2., 2., 4., 4.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 2., 0.),
            Point3::new(3., 0., 1.),
            Point3::new(3., 2., 4.),
        ],
        vec![1., 2., 5., 11.],
    )
    .unwrap();
    let u = 0.3;
    let curve = s.isocurve(0, u).unwrap();
    for i in 0..=20 {
        let t = i as f64 / 20.;
        let b = [(1. - u) * (1. - t), (1. - u) * t, u * (1. - t), u * t];
        let mut numerator = Point3::new(0., 0., 0.);
        let mut denominator = 0.;
        for (j, basis) in b.into_iter().enumerate() {
            let w = basis * s.weights()[j];
            numerator = numerator + s.control_points()[j] * w;
            denominator += w;
        }
        assert!(
            (curve.evaluate(2. + 2. * t).unwrap() - numerator * (1. / denominator)).norm() < 1e-13
        );
        let derivatives = [-(1. - u), 1. - u, -u, u];
        let mut dn = Point3::new(0., 0., 0.);
        let mut dw = 0.;
        for (j, basis) in derivatives.into_iter().enumerate() {
            let w = basis * s.weights()[j];
            dn = dn + s.control_points()[j] * w;
            dw += w;
        }
        let point = numerator * (1. / denominator);
        let derivative = (dn - point * dw) * (0.5 / denominator);
        assert!((curve.derivative(2. + 2. * t).unwrap() - derivative).norm() < 1e-13);
    }
}
#[test]
fn invalid_inputs_and_resource_limits() {
    let s = patch(1., 1.);
    for axis in [2, usize::MAX] {
        assert!(s.isocurve(axis, 3.).is_err());
        assert!(s.insert_knot(axis, 3., 0).is_err());
    }
    for u in [f64::NAN, f64::INFINITY, 1., 7.] {
        assert!(s.isocurve(0, u).is_err());
        assert!(s.insert_knot(0, u, 0).is_err());
    }
    assert!(s.insert_knot(0, 2., 1).is_err());
    assert!(s.insert_knot(0, 6., 1).is_err());
    assert!(s.insert_knot(0, 4., 3).is_err());
    assert!(s
        .insert_knot(0, 4., 2)
        .unwrap()
        .insert_knot(0, 4., 1)
        .is_err());
    assert_eq!(
        s.insert_knot(0, 2., 0).unwrap().control_points(),
        s.control_points()
    );
    let count = 256;
    let mut k = vec![0., 0.];
    k.extend((1..count - 1).map(|i| i as f64));
    k.extend([count as f64, count as f64]);
    let large = NurbsSurface::new(
        [1, 1],
        [k.clone(), k],
        [count, count],
        vec![Point3::new(0., 0., 0.); count * count],
        vec![1.; count * count],
    )
    .unwrap();
    assert!(large.insert_knot(0, 0.5, 1).is_err());
}
#[test]
fn degree_sixteen_invariance() {
    let p = 16;
    let mut u = vec![0.; p + 1];
    u.extend(vec![1.; p + 1]);
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..=p {
        for j in 0..2 {
            points.push(Point3::new(i as f64, j as f64, (i * i + j) as f64));
            weights.push(1. + (i + j) as f64 / 10.);
        }
    }
    let s = NurbsSurface::new(
        [p, 1],
        [u, vec![0., 0., 1., 1.]],
        [p + 1, 2],
        points,
        weights,
    )
    .unwrap();
    let r = s.insert_knot(0, 0.375, p).unwrap();
    for i in 0..=40 {
        let u = i as f64 / 40.;
        let a = s
            .evaluate_with_partials(u, 0.3, [KnotSide::Right; 2])
            .unwrap();
        let b = r
            .evaluate_with_partials(u, 0.3, [KnotSide::Right; 2])
            .unwrap();
        assert!((a.point - b.point).norm() < 1e-10);
        assert!((a.du - b.du).norm() < 1e-9);
        assert!((a.dv - b.dv).norm() < 1e-10);
    }
}

#[test]
fn refinement_preflights_work_and_huge_times() {
    let s = patch(1., 1.);
    assert!(s.insert_knot(0, 3., usize::MAX).is_err());
    let p = 16;
    let v_count = 1800;
    let mut u = vec![0.; p + 1];
    u.extend(vec![1.; p + 1]);
    let mut v = vec![0., 0.];
    v.extend((1..v_count - 1).map(|i| i as f64));
    v.extend([v_count as f64, v_count as f64]);
    let s = NurbsSurface::new(
        [p, 1],
        [u, v],
        [p + 1, v_count],
        vec![Point3::new(0., 0., 0.); (p + 1) * v_count],
        vec![1.; (p + 1) * v_count],
    )
    .unwrap();
    assert!(matches!(
        s.insert_knot(0, 0.5, p),
        Err(hagane::Error::Unsupported(
            "surface refinement exceeds work limit"
        ))
    ));
}
#[test]
fn oriented_boundary_closes_and_uv_maps_share_curve_parameters() {
    let s = patch(1., 1.)
        .insert_knot(0, 3.5, 1)
        .unwrap()
        .insert_knot(1, 1., 1)
        .unwrap();
    let edges = s.boundary_edges().unwrap();
    assert_eq!(
        edges.each_ref().map(|e| e.forward),
        [true, true, false, false]
    );
    let mut ends = Vec::new();
    for edge in &edges {
        let [a, b] = edge.curve.domain();
        for i in 0..=32 {
            let t = a + (b - a) * i as f64 / 32.;
            let uv = edge.pcurve.evaluate(t);
            assert!(
                (s.evaluate(uv[0], uv[1]).unwrap() - edge.curve.evaluate(t).unwrap()).norm()
                    < 1e-12
            );
        }
        ends.push(if edge.forward {
            [
                edge.curve.evaluate(a).unwrap(),
                edge.curve.evaluate(b).unwrap(),
            ]
        } else {
            [
                edge.curve.evaluate(b).unwrap(),
                edge.curve.evaluate(a).unwrap(),
            ]
        });
    }
    for i in 0..4 {
        assert!((ends[i][1] - ends[(i + 1) % 4][0]).norm() < 1e-12);
    }
    let domains = edges.each_ref().map(|e| e.curve.domain());
    assert_eq!(domains, [[2., 6.], [-3., 5.], [2., 6.], [-3., 5.]]);
}
