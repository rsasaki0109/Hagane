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
fn actual_skew_bore_and_enlargement_preserve_outer_and_partition_full_metrics() {
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let f =
            Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), p).unwrap();
        let source = NurbsFrustumSolid::new(f, [12. * s, 12. * s], 40. * s, p).unwrap();
        let lo = plane(f, s, 12., 0.2, 0.1);
        let hi = plane(f, s, 24., -0.1, 0.15);
        let bounded = source
            .split_by_plane(&lo, p)
            .unwrap()
            .upper
            .split_by_interior_plane(&hi, p)
            .unwrap()
            .lower;
        let before = format!("{:?}", bounded.solid());
        let partition = bounded.partition_coaxial_bore(4. * s, p).unwrap();
        let hole = &partition.kept;
        geometry(hole.solid(), true, s);
        geometry(partition.removed.solid(), false, s);
        compare(
            hole.inertia_properties(p).unwrap(),
            oracle(f, s, 4., [12., 0.2, 0.1], [24., -0.1, 0.15]),
            s,
        );
        assert_eq!(
            format!("{:?}", &hole.solid().edges[..12]),
            format!("{:?}", bounded.solid().edges)
        );
        assert_eq!(
            format!("{:?}", &hole.solid().vertices[..8]),
            format!("{:?}", bounded.solid().vertices)
        );
        let new = hole.partition_coaxial_bore_enlargement(6. * s, p).unwrap();
        geometry(new.kept.solid(), true, s);
        geometry(new.removed.solid(), true, s);
        compare(
            new.kept.inertia_properties(p).unwrap(),
            oracle(f, s, 6., [12., 0.2, 0.1], [24., -0.1, 0.15]),
            s,
        );
        assert!(
            (new.kept.volume(p).unwrap() + new.removed.volume(p).unwrap()
                - hole.volume(p).unwrap())
            .abs()
                < 2e-9 * hole.volume(p).unwrap()
        );
        assert!(
            (hole.volume(p).unwrap() + partition.removed.volume(p).unwrap()
                - bounded.volume(p).unwrap())
            .abs()
                < 2e-9 * bounded.volume(p).unwrap()
        );
        assert_eq!(
            format!("{:?}", &new.kept.solid().edges[..12]),
            format!("{:?}", bounded.solid().edges)
        );
        let direct = bounded.bore_coaxial(6. * s, p).unwrap();
        assert_eq!(
            format!("{:?}", direct.solid()),
            format!("{:?}", new.kept.solid())
        );
        let step = new.kept.export_step_mm(p).unwrap();
        assert_eq!(step.matches("FACE_BOUND(").count(), 2);
        let world = |v: Point3| f.point(v * s);
        assert_eq!(
            new.kept
                .classify_point(world(Point3::new(0., 0., 18.)), p)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            new.kept
                .classify_point(world(Point3::new(6., 0., 18.)), p)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            new.kept
                .classify_point(world(Point3::new(8., 0., 18.)), p)
                .unwrap(),
            PointLocation::Inside
        );
        let query = new
            .kept
            .intersect_segment(
                world(Point3::new(-20., 0., 18.)),
                world(Point3::new(20., 0., 18.)),
                p,
            )
            .unwrap();
        assert_eq!(query.hits.len(), 4);
        assert_eq!(query.material_intervals.len(), 2);
        for h in &query.hits {
            for w in &h.faces {
                let actual = new.kept.solid().shell.faces[w.face_id]
                    .surface
                    .try_evaluate(w.uv[0], w.uv[1])
                    .unwrap();
                assert!((actual - h.point).norm() < 1e-7 * s);
            }
        }
        let mesh = new.kept.tessellate(0.2 * s, p).unwrap();
        assert!(mesh.signed_volume() > 0.);
        let repeated = new
            .kept
            .enlarge_coaxial_bore(8. * s, p)
            .unwrap()
            .split_by_interior_plane(&plane(f, s, 16., 0.05, -0.05), p)
            .unwrap();
        compare(
            repeated.lower.inertia_properties(p).unwrap(),
            oracle(f, s, 8., [12., 0.2, 0.1], [16., 0.05, -0.05]),
            s,
        );
        assert_eq!(before, format!("{:?}", bounded.solid()));
    }
}
#[test]
fn radius_and_world_policy_failures_are_immutable_and_recover() {
    let p = policy(1.);
    let f = Frame3::IDENTITY;
    let source = NurbsFrustumSolid::new(f, [12., 12.], 40., p).unwrap();
    let lo = plane(f, 1., 12., 0.2, 0.1);
    let hi = plane(f, 1., 24., -0.1, 0.15);
    let body = source
        .split_by_plane(&lo, p)
        .unwrap()
        .upper
        .split_by_interior_plane(&hi, p)
        .unwrap()
        .lower;
    let before = format!("{:?}", body.solid());
    for r in [0., -1., 12., 12. - 1e-7, f64::NAN, f64::INFINITY] {
        assert!(body.bore_coaxial(r, p).is_err());
    }
    let hole = body.bore_coaxial(4., p).unwrap();
    let previous = format!("{:?}", hole.solid());
    let step = hole.export_step_mm(p).unwrap();
    for r in [0., 3., 4., 4. + 1e-7, 12., f64::NAN, f64::INFINITY] {
        assert!(hole.enlarge_coaxial_bore(r, p).is_err());
    }
    assert!(hole
        .enlarge_coaxial_bore(4.1, GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap())
        .is_err());
    assert_eq!(before, format!("{:?}", body.solid()));
    assert_eq!(previous, format!("{:?}", hole.solid()));
    assert_eq!(step, hole.export_step_mm(p).unwrap());
    assert!(hole.enlarge_coaxial_bore(6., p).is_ok());
}
