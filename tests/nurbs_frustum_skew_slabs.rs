use hagane::*;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn plane(f: Frame3, s: f64, d: f64, a: f64, b: f64) -> Surface {
    let n = Vec3::new(a, b, 1.).normalized().unwrap();
    let u = (Vec3::new(0., 1., 0.) - n * n.y).normalized().unwrap();
    let vector = |p: Vec3| f.axes()[0] * p.x + f.axes()[1] * p.y + f.axes()[2] * p.z;
    Surface::Plane {
        origin: f.point(Point3::new(0., 0., d * s)),
        u: vector(u),
        v: vector(n.cross(u)),
    }
}
fn oracle(
    f: Frame3,
    s: f64,
    hole: f64,
    lower: [f64; 3],
    upper: [f64; 3],
) -> NurbsGraphInertiaProperties {
    let [dl, al, bl] = lower;
    let [du, au, bu] = upper;
    let dl = dl * s;
    let du = du * s;
    let delta = du - dl;
    let r = 12. * s;
    let rh = hole * s;
    let c = (r * r + rh * rh) / 4.;
    let volume = std::f64::consts::PI * (r - rh) * (r + rh) * delta;
    let nl = al * al + bl * bl;
    let nu = au * au + bu * bu;
    let mean = [
        -c * (au - al) / delta,
        -c * (bu - bl) / delta,
        (du * du - dl * dl + c * (nu - nl)) / (2. * delta),
    ];
    let xz = -c * (du * au - dl * al) / delta;
    let yz = -c * (du * bu - dl * bl) / delta;
    let raw = [
        [c, 0., xz],
        [0., c, yz],
        [
            xz,
            yz,
            (du.powi(3) - dl.powi(3)) / (3. * delta) + c * (du * nu - dl * nl) / delta,
        ],
    ];
    let cov: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| raw[i][j] - mean[i] * mean[j]));
    let trace = cov[0][0] + cov[1][1] + cov[2][2];
    let local: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| volume * ((if i == j { trace } else { 0. }) - cov[i][j]))
    });
    let axes = f.axes().map(|p| [p.x, p.y, p.z]);
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|a| {
                    (0..3)
                        .map(|b| axes[a][i] * local[a][b] * axes[b][j])
                        .sum::<f64>()
                })
                .sum()
        })
    });
    NurbsGraphInertiaProperties {
        volume,
        centroid: f.point(Point3::new(mean[0], mean[1], mean[2])),
        inertia,
    }
}
fn compare(actual: NurbsGraphInertiaProperties, expected: NurbsGraphInertiaProperties, s: f64) {
    assert!((actual.volume - expected.volume).abs() < 2e-9 * expected.volume);
    assert!((actual.centroid - expected.centroid).norm() < 2e-9 * s);
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (actual.inertia[i][j] - expected.inertia[i][j]).abs()
                    < 2e-9 * expected.volume * 144. * s * s
            );
        }
    }
}
fn geometry(body: &Solid, hole: bool, s: f64) {
    assert_eq!(
        (
            body.vertices.len(),
            body.edges.len(),
            body.shell.faces.len()
        ),
        if hole { (16, 24, 10) } else { (8, 12, 6) }
    );
    let mut uses = vec![(0, 0); body.edges.len()];
    for face in &body.shell.faces {
        for c in face.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].0 += 1;
            uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
            for i in 0..=16 {
                let t = i as f64 / 16.;
                let uv = c.pcurve.try_evaluate(t).unwrap();
                let actual = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let expected = body.edges[c.edge].curve.try_evaluate(t).unwrap();
                assert!((actual - expected).norm() < 1e-8 * s);
            }
        }
    }
    assert!(uses.iter().all(|v| *v == (2, 0)));
    assert_eq!(body.shell.faces[0].wires.len(), if hole { 2 } else { 1 });
    assert_eq!(body.shell.faces[1].wires.len(), if hole { 2 } else { 1 });
}
#[test]
fn skew_cylinder_and_annulus_bounds_match_independent_full_tensor_and_actual_geometry() {
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let rot = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let f = Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), p).unwrap();
        let source = NurbsFrustumSolid::new(f, [12. * s, 12. * s], 40. * s, p).unwrap();
        let lower = plane(f, s, 12., 0.2, 0.1);
        let upper = plane(f, s, 24., -0.1, 0.15);
        let middle = plane(f, s, 16., 0.05, -0.05);
        let initial = source.split_by_plane(&lower, p).unwrap();
        assert!(initial.upper.split_by_parallel_plane(&upper, p).is_err());
        let between = initial
            .upper
            .split_by_interior_plane(&upper, p)
            .unwrap()
            .lower;
        geometry(between.solid(), false, s);
        compare(
            between.inertia_properties(p).unwrap(),
            oracle(f, s, 0., [12., 0.2, 0.1], [24., -0.1, 0.15]),
            s,
        );
        let repeated = between.split_by_interior_plane(&middle, p).unwrap();
        for child in [&repeated.lower, &repeated.upper] {
            geometry(child.solid(), false, s);
            child.validate(p).unwrap();
            assert!(child
                .export_step_mm(p)
                .unwrap()
                .contains("MANIFOLD_SOLID_BREP"));
        }
        compare(
            repeated.lower.inertia_properties(p).unwrap(),
            oracle(f, s, 0., [12., 0.2, 0.1], [16., 0.05, -0.05]),
            s,
        );
        compare(
            repeated.upper.inertia_properties(p).unwrap(),
            oracle(f, s, 0., [16., 0.05, -0.05], [24., -0.1, 0.15]),
            s,
        );
        let bored = source.bore_coaxial(4. * s, p).unwrap();
        let initial = bored.split_by_plane(&lower, p).unwrap();
        let between = initial
            .upper
            .split_by_interior_plane(&upper, p)
            .unwrap()
            .lower;
        let before = format!("{:?}", between.solid());
        let step = between.export_step_mm(p).unwrap();
        geometry(between.solid(), true, s);
        compare(
            between.inertia_properties(p).unwrap(),
            oracle(f, s, 4., [12., 0.2, 0.1], [24., -0.1, 0.15]),
            s,
        );
        assert_eq!(step.matches("FACE_BOUND(").count(), 2);
        let repeated = between.split_by_interior_plane(&middle, p).unwrap();
        compare(
            repeated.lower.inertia_properties(p).unwrap(),
            oracle(f, s, 4., [12., 0.2, 0.1], [16., 0.05, -0.05]),
            s,
        );
        compare(
            repeated.upper.inertia_properties(p).unwrap(),
            oracle(f, s, 4., [16., 0.05, -0.05], [24., -0.1, 0.15]),
            s,
        );
        let reverse = initial
            .upper
            .split_by_interior_plane(&middle, p)
            .unwrap()
            .upper
            .split_by_interior_plane(&upper, p)
            .unwrap()
            .lower;
        compare(
            reverse.inertia_properties(p).unwrap(),
            repeated.upper.inertia_properties(p).unwrap(),
            s,
        );
        for (point, expected) in [
            (Point3::new(6., 0., 18.), PointLocation::Inside),
            (Point3::new(0., 0., 18.), PointLocation::Outside),
            (Point3::new(6., 0., 10.8), PointLocation::Boundary),
            (Point3::new(6., 0., 24.6), PointLocation::Boundary),
            (Point3::new(6., 0., 8.), PointLocation::Outside),
            (Point3::new(6., 0., 28.), PointLocation::Outside),
        ] {
            assert_eq!(
                between.classify_point(f.point(point * s), p).unwrap(),
                expected
            );
        }
        assert_eq!(before, format!("{:?}", between.solid()));
        assert_eq!(step, between.export_step_mm(p).unwrap());
    }
}
#[test]
fn crossing_near_and_invalid_new_bounds_preserve_source_and_recover() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 40., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let lower = plane(f, 1., 12., 0.2, 0.1);
    let initial = source.split_by_plane(&lower, p).unwrap();
    let before = format!("{:?}", initial.upper.solid());
    let step = initial.upper.export_step_mm(p).unwrap();
    for candidate in [
        lower.clone(),
        plane(f, 1., 12. + 1e-7, 0.2, 0.1),
        plane(f, 1., 13., 2., 0.),
        plane(f, 1., 39., -0.1, 0.15),
        plane(f, 1., f64::NAN, 0.1, 0.),
        plane(f, 1., 1e12, 0.1, 0.),
    ] {
        assert!(initial
            .upper
            .split_by_interior_plane(&candidate, p)
            .is_err());
    }
    let valid = plane(f, 1., 24., -0.1, 0.15);
    assert!(initial.upper.split_by_interior_plane(&valid, p).is_ok());
    assert_eq!(before, format!("{:?}", initial.upper.solid()));
    assert_eq!(step, initial.upper.export_step_mm(p).unwrap());
}
