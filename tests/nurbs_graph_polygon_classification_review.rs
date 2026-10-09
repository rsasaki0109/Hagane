use hagane::*;
fn tol() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn polygon() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [0., 1.]]
}
fn opening() -> Vec<[f64; 2]> {
    vec![[0.2, 0.2], [0.4, 0.2], [0.2, 0.4]]
}
fn inside(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    poly.iter().enumerate().all(|(i, a)| {
        let b = poly[(i + 1) % poly.len()];
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) >= 0.
    })
}
fn distance(p: [f64; 2], poly: &[[f64; 2]]) -> f64 {
    poly.iter()
        .enumerate()
        .map(|(i, a)| {
            let b = poly[(i + 1) % poly.len()];
            let d = [b[0] - a[0], b[1] - a[1]];
            let t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
            let t = t.clamp(0., 1.);
            (p[0] - a[0] - d[0] * t).hypot(p[1] - a[1] - d[1] * t)
        })
        .fold(f64::INFINITY, f64::min)
}
fn oracle(p: Point3, holed: bool) -> PointLocation {
    let xy = [p.x, p.y];
    let outer = polygon();
    let hole = opening();
    let within = inside(xy, &outer) && (!holed || !inside(xy, &hole));
    let outerd = distance(xy, &outer);
    let wall = if holed {
        outerd.min(distance(xy, &hole))
    } else {
        outerd
    };
    let gapz = if p.z < 0. {
        -p.z
    } else if p.z > 1. {
        p.z - 1.
    } else {
        0.
    };
    let capxy = if within {
        0.
    } else if !inside(xy, &outer) {
        outerd
    } else {
        distance(xy, &hole)
    };
    let dist = wall
        .hypot(gapz)
        .min(capxy.hypot(p.z.abs().min((p.z - 1.).abs())));
    if dist <= 1e-6 {
        PointLocation::Boundary
    } else if within && p.z > 0. && p.z < 1. {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}
#[test]
fn flat_triangular_prism_matches_independent_segment_and_cap_euclidean_oracle() {
    let source = NurbsGraphSolid::new([1.; 3], 0., Tolerance::default()).unwrap();
    let p = NurbsGraphPolygonSolid::new(&source, polygon(), Tolerance::default()).unwrap();
    let h = p
        .through_uv_polygon(opening(), Tolerance::default())
        .unwrap();
    for holed in [false, true] {
        for point in [
            Point3::new(0.1, 0.1, 0.5),
            Point3::new(0.25, 0.25, 0.5),
            Point3::new(0.8, 0.8, 1. + 0.2e-6),
            Point3::new(0.8, 0.8, -0.2e-6),
            Point3::new(0.25, 0.25, 1. + 0.2e-6),
            Point3::new(0.25, 0.25, -0.2e-6),
            Point3::new(0.5, 0.5, 0.5),
            Point3::new(0.1, 0.1, 0.),
            Point3::new(0.1, 0.1, 1.),
            Point3::new(-0.8e-6, -0.8e-6, -0.8e-6),
            Point3::new(-0.4e-6, -0.4e-6, -0.4e-6),
            Point3::new(-0.8e-6, -0.8e-6, 0.5),
            Point3::new(-0.5e-6, -0.5e-6, 0.5),
            Point3::new(0.5 + 0.8e-6, 0.5 + 0.8e-6, 1. + 0.5e-6),
            Point3::new(0.2 - 0.8e-6, 0.2 - 0.8e-6, 0.5),
            Point3::new(0.2 - 0.5e-6, 0.2 - 0.5e-6, 0.5),
        ] {
            let actual = if holed {
                h.classify_point(point, tol())
            } else {
                p.classify_point(point, tol())
            };
            assert_eq!(
                actual.unwrap(),
                oracle(point, holed),
                "{point:?} holed={holed}"
            );
        }
    }
}
#[test]
fn signed_trimmed_rigid_source_excluded_cap_witnesses_and_actual_inner_walls() {
    let tr = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let s = NurbsGraphSolid::new([20., 12., 3.], -2., Tolerance::default())
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], Tolerance::default())
        .unwrap()
        .transformed(tr, Tolerance::default())
        .unwrap();
    let p = NurbsGraphPolygonSolid::new(
        &s,
        vec![[0.15, 0.15], [0.85, 0.15], [0.15, 0.85]],
        Tolerance::default(),
    )
    .unwrap();
    let h = p
        .through_uv_polygon(
            vec![[0.25, 0.25], [0.45, 0.25], [0.25, 0.45]],
            Tolerance::default(),
        )
        .unwrap();
    let roof = |u: f64, v: f64| 3. - 8. * u * (1. - u) * v * (1. - v);
    let point = |u: f64, v: f64, z: f64| tr.point(Point3::new(20. * u, 12. * v, z));
    for z in [-0.2e-6, 0., roof(0.3, 0.3), roof(0.3, 0.3) + 0.2e-6] {
        assert_eq!(
            h.classify_point(point(0.3, 0.3, z), tol()).unwrap(),
            PointLocation::Outside
        );
    }
    for z in [0., roof(0.7, 0.7)] {
        assert_eq!(
            p.classify_point(point(0.7, 0.7, z), tol()).unwrap(),
            PointLocation::Outside
        );
    }
    assert_eq!(
        h.classify_point(point(0.2, 0.2, 1.), tol()).unwrap(),
        PointLocation::Inside
    );
    for uv in [[0.25, 0.35], [0.35, 0.25], [0.35, 0.35]] {
        assert_eq!(
            h.classify_point(point(uv[0], uv[1], 1.), tol()).unwrap(),
            PointLocation::Boundary
        );
    }
}
#[test]
fn steep_roof_uses_euclidean_normal_offsets_and_invalid_public_geometry_rejects() {
    let s = NurbsGraphSolid::new([1.; 3], 100., Tolerance::default()).unwrap();
    let p = NurbsGraphPolygonSolid::new(&s, polygon(), Tolerance::default()).unwrap();
    let q = Point3::new(0.1, 0.5, 10.);
    let n = Vec3::new(-80., 0., 1.).normalized().unwrap();
    for sign in [-1., 1.] {
        match p.classify_point(q + n * (sign * 0.5e-6), tol()) {
            Ok(location) => assert_eq!(location, PointLocation::Boundary),
            Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
    assert_eq!(p.classify_point(q, tol()).unwrap(), PointLocation::Boundary);
    assert!(p
        .classify_point(Point3::new(f64::NAN, 0., 0.), tol())
        .is_err());
    let mut broken = p.clone();
    broken.solid.vertices[0].point.x += 1e-12;
    assert!(broken.classify_point(q, tol()).is_err());
    let mut broken = p;
    broken.solid.shell.faces[0].wires[0].coedges[0].edge = 999;
    assert!(broken.classify_point(q, tol()).is_err());
}
