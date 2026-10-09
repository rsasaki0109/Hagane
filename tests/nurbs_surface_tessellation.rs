use hagane::{NurbsSurface, NurbsSurfaceMesh, Point3};
fn knots(p: usize, a: f64, b: f64) -> Vec<f64> {
    let mut k = vec![a; p + 1];
    k.extend(vec![b; p + 1]);
    k
}
fn warped(scale: f64, weights: f64) -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [knots(1, 2., 6.), knots(1, -3., 5.)],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 1.),
        ]
        .into_iter()
        .map(|p| p * scale)
        .collect(),
        vec![weights; 4],
    )
    .unwrap()
}
fn verify(surface: &NurbsSurface, tess: &NurbsSurfaceMesh, error: f64) {
    assert_eq!(tess.mesh.triangles.len(), tess.uv_ranges.len() * 2);
    assert_eq!(tess.error_bounds.len(), tess.uv_ranges.len());
    for (cell, range) in tess.uv_ranges.iter().enumerate() {
        assert!(tess.error_bounds[cell] <= error);
        let first = tess.mesh.triangles[2 * cell];
        let second = tess.mesh.triangles[2 * cell + 1];
        assert_eq!(first[0], second[0]);
        assert_eq!(first[2], second[1]);
        let corners = [
            tess.mesh.positions[first[0]],
            tess.mesh.positions[first[1]],
            tess.mesh.positions[first[2]],
            tess.mesh.positions[second[2]],
        ];
        let [[a, b], [c, d]] = *range;
        for i in 0..=12 {
            for j in 0..=12 {
                let s = i as f64 / 12.;
                let t = j as f64 / 12.;
                let u = a + (b - a) * s;
                let v = c + (d - c) * t;
                let actual_s = (u - a) / (b - a);
                let actual_t = (v - c) / (d - c);
                let triangle = if actual_t <= actual_s {
                    corners[0] * (1. - actual_s)
                        + corners[1] * (actual_s - actual_t)
                        + corners[2] * actual_t
                } else {
                    corners[0] * (1. - actual_t)
                        + corners[2] * actual_s
                        + corners[3] * (actual_t - actual_s)
                };
                let p = surface.evaluate(u, v).unwrap();
                let nearest = triangle_distance(p, [corners[0], corners[1], corners[2]])
                    .min(triangle_distance(p, [corners[0], corners[2], corners[3]]));
                assert!(nearest <= tess.error_bounds[cell] + error * 1e-10);
                assert!(
                    (p - triangle).norm() <= tess.error_bounds[cell] + error * 1e-10,
                    "distance {} bound {}",
                    (p - triangle).norm(),
                    tess.error_bounds[cell]
                );
            }
        }
    }
    let mut edges = std::collections::BTreeMap::new();
    for tri in &tess.mesh.triangles {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edges.entry(key).or_insert(0) += 1;
        }
    }
    assert!(edges.values().all(|n| *n == 1 || *n == 2));
    let level = (tess.uv_ranges.len() as f64).sqrt() as usize;
    assert_eq!(tess.mesh.positions.len(), (level + 1) * (level + 1));
    assert_eq!(edges.values().filter(|n| **n == 1).count(), 4 * level);
}
#[test]
fn bilinear_warp_has_two_triangle_bound_and_conforming_mesh() {
    let surface = warped(1., 1.);
    let tess = surface.tessellate_bounded(0.01, 4096).unwrap();
    assert!(tess.uv_ranges.len() > 1);
    verify(&surface, &tess, 0.01);
    let coarse = surface.tessellate_bounded(0.3, 1).unwrap();
    assert_eq!(coarse.uv_ranges.len(), 1);
    assert!((coarse.error_bounds[0] - 0.25).abs() < 1e-10);
    verify(&surface, &coarse, 0.3);
}
#[test]
fn rational_quarter_cylinder_and_nonuniform_weights() {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (p, w) in [
        (Point3::new(1., 0., 0.), 1.),
        (Point3::new(1., 1., 0.), std::f64::consts::FRAC_1_SQRT_2),
        (Point3::new(0., 1., 0.), 1.),
    ] {
        for z in [0., 2.] {
            points.push(Point3::new(p.x, p.y, z));
            weights.push(w);
        }
    }
    let cylinder = NurbsSurface::new(
        [2, 1],
        [knots(2, 2., 6.), knots(1, -3., 5.)],
        [3, 2],
        points,
        weights,
    )
    .unwrap();
    let tess = cylinder.tessellate_bounded(0.01, 4096).unwrap();
    verify(&cylinder, &tess, 0.01);
    for p in &tess.mesh.positions {
        assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-14);
    }
    let rational = NurbsSurface::new(
        [1, 1],
        [knots(1, 2., 6.), knots(1, -3., 5.)],
        [2, 2],
        warped(1., 1.).control_points().to_vec(),
        vec![1., 1.3, 0.8, 2.],
    )
    .unwrap();
    verify(
        &rational,
        &rational.tessellate_bounded(0.02, 4096).unwrap(),
        0.02,
    );
}
#[test]
fn small_dimensions_common_weight_scale_and_degree_sixteen() {
    let tiny = warped(1e-100, 1e200);
    verify(
        &tiny,
        &tiny.tessellate_bounded(1e-102, 4096).unwrap(),
        1e-102,
    );
    let degree = 16;
    let count = degree + 1;
    let points = (0..count)
        .flat_map(|i| (0..2).map(move |j| Point3::new(i as f64 / degree as f64, j as f64, 0.)))
        .collect();
    let surface = NurbsSurface::new(
        [degree, 1],
        [knots(degree, 0., 1.), knots(1, 0., 1.)],
        [count, 2],
        points,
        vec![1.; count * 2],
    )
    .unwrap();
    let mesh = surface.tessellate_bounded(0.001, 1).unwrap();
    verify(&surface, &mesh, 0.001);
}
#[test]
fn rounded_original_parameter_midpoints_match_mesh() {
    let a = 1e16;
    let b = a + 6.;
    let surface = NurbsSurface::new(
        [1, 1],
        [knots(1, a, b), knots(1, a, b)],
        [2, 2],
        warped(1., 1.).control_points().to_vec(),
        vec![1.; 4],
    )
    .unwrap();
    let mesh = surface.tessellate_bounded(0.12, 64).unwrap();
    verify(&surface, &mesh, 0.12);
    assert!(surface.tessellate_bounded(1e-4, 65536).is_err());
}
#[test]
fn rejects_invalid_multispan_singular_precision_and_resources() {
    let s = warped(1., 1.);
    for error in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(s.tessellate_bounded(error, 10).is_err());
    }
    for count in [0, 65537, usize::MAX] {
        assert!(s.tessellate_bounded(0.1, count).is_err());
    }
    assert!(s.tessellate_bounded(0.01, 1).is_err());
    assert!(s.tessellate_bounded(1e-16, 65536).is_err());
    assert!(s
        .insert_knot(0, 4., 1)
        .unwrap()
        .tessellate_bounded(0.1, 10)
        .is_err());
    let shifted = NurbsSurface::new(
        s.degrees(),
        [s.knots(0).unwrap().to_vec(), s.knots(1).unwrap().to_vec()],
        s.control_counts(),
        s.control_points()
            .iter()
            .map(|p| *p + Point3::new(1e15, 0., 0.))
            .collect(),
        s.weights().to_vec(),
    )
    .unwrap();
    assert!(shifted.tessellate_bounded(0.01, 65536).is_err());
    let singular = NurbsSurface::new(
        [1, 1],
        [knots(1, 0., 1.), knots(1, 0., 1.)],
        [2, 2],
        vec![Point3::new(0., 0., 0.); 4],
        vec![1.; 4],
    )
    .unwrap();
    assert!(singular.tessellate_bounded(1., 1).is_err());
}

// Independent geometric distance: plane projection with barycentric membership,
// otherwise the nearest of three segments. Rescale to avoid tiny squared norms.
fn triangle_distance(point: Point3, tri: [Point3; 3]) -> f64 {
    let origin = tri[0];
    let scale = (tri[1] - origin).norm().max((tri[2] - origin).norm());
    let local = |p: Point3| {
        let d = p - origin;
        Point3::new(d.x / scale, d.y / scale, d.z / scale)
    };
    let p = local(point);
    let a = Point3::new(0., 0., 0.);
    let b = local(tri[1]);
    let c = local(tri[2]);
    let normal = b.cross(c).normalized().unwrap();
    let distance = p.dot(normal);
    let projected = p - normal * distance;
    let bb = b.dot(b);
    let cc = c.dot(c);
    let bc = b.dot(c);
    let pb = projected.dot(b);
    let pc = projected.dot(c);
    let denominator = bb * cc - bc * bc;
    let u = (pb * cc - pc * bc) / denominator;
    let v = (pc * bb - pb * bc) / denominator;
    if u >= 0. && v >= 0. && u + v <= 1. {
        return distance.abs() * scale;
    }
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .map(|(a, b)| {
            let d = b - a;
            let t = ((p - a).dot(d) / d.dot(d)).clamp(0., 1.);
            (p - a - d * t).norm() * scale
        })
        .fold(f64::INFINITY, f64::min)
}
#[test]
fn sampled_fold_orientation_is_rejected() {
    let folded = NurbsSurface::new(
        [1, 1],
        [knots(1, 0., 1.), knots(1, 0., 1.)],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(1., 1., 0.),
            Point3::new(1., 0., 0.),
            Point3::new(0., 1., 0.),
        ],
        vec![1.; 4],
    )
    .unwrap();
    assert!(folded.tessellate_bounded(1., 1).is_err());
}
