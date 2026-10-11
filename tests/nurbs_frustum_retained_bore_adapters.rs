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
fn retained(original: &Solid, actual: &Solid) {
    assert_eq!(
        format!("{:?}", &actual.vertices[..8]),
        format!("{:?}", &original.vertices[..8])
    );
    assert_eq!(
        format!("{:?}", &actual.edges[..12]),
        format!("{:?}", &original.edges[..12])
    );
    for i in 0..6 {
        assert_eq!(
            format!("{:?}", actual.shell.faces[i].surface),
            format!("{:?}", original.shell.faces[i].surface)
        );
        assert_eq!(
            format!("{:?}", actual.shell.faces[i].wires[0]),
            format!("{:?}", original.shell.faces[i].wires[0])
        );
    }
}
fn verify(
    body: &NurbsBoredFrustumPlaneBoundedSolid,
    f: Frame3,
    s: f64,
    lo: [f64; 3],
    hi: [f64; 3],
    radius: f64,
) {
    let p = policy(s);
    body.validate(p).unwrap();
    geometry(body.solid(), true, s);
    compare(
        body.inertia_properties(p).unwrap(),
        oracle(f, s, radius, lo, hi),
        s,
    );
    let step = body.export_step_mm(p).unwrap();
    assert_eq!(step.matches("FACE_BOUND(").count(), 2);
    let midpoint = (lo[0] + hi[0]) / 2.;
    assert_eq!(
        body.classify_point(f.point(Point3::new(0., 0., midpoint) * s), p)
            .unwrap(),
        PointLocation::Outside
    );
    assert_eq!(
        body.classify_point(f.point(Point3::new(radius, 0., midpoint) * s), p)
            .unwrap(),
        PointLocation::Boundary
    );
    assert!(body.tessellate(0.3 * s, p).unwrap().signed_volume() > 0.);
}
#[test]
fn old_oblique_and_parallel_adapters_preserve_actual_outer_and_removed_inner() {
    for s in [1e-4, 1., 10.] {
        let p = policy(s);
        let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
        let f =
            Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rotation.axes(), p).unwrap();
        let source = NurbsFrustumSolid::new(f, [12. * s, 12. * s], 40. * s, p).unwrap();
        let cut = plane(f, s, 16., 0.2, 0.1);
        let split = source.split_by_plane(&cut, p).unwrap();
        for (old, lo, hi) in [
            (&split.lower, [0., 0., 0.], [16., 0.2, 0.1]),
            (&split.upper, [16., 0.2, 0.1], [40., 0., 0.]),
        ] {
            let before = format!("{:?}", old.solid());
            let new = old.partition_coaxial_bore(4. * s, p).unwrap();
            retained(old.solid(), new.kept.solid());
            verify(&new.kept, f, s, lo, hi, 4.);
            assert!(
                (new.kept.volume(p).unwrap() + new.removed.volume(p).unwrap()
                    - old.volume(p).unwrap())
                .abs()
                    < 2e-9 * old.volume(p).unwrap()
            );
            assert_eq!(before, format!("{:?}", old.solid()));
        }
        let Surface::Plane { origin, u, v } = cut.clone() else {
            panic!("plane")
        };
        let next = Surface::Plane {
            origin: origin + f.axes()[2] * (8. * s),
            u,
            v,
        };
        let slab = split.upper.split_by_parallel_plane(&next, p).unwrap().lower;
        let drilled = slab.bore_coaxial(4. * s, p).unwrap();
        retained(slab.solid(), drilled.solid());
        verify(&drilled, f, s, [16., 0.2, 0.1], [24., 0.2, 0.1], 4.);
        let bored = source.bore_coaxial(4. * s, p).unwrap();
        let cut = bored.split_by_plane(&cut, p).unwrap();
        for (old, lo, hi) in [
            (&cut.lower, [0., 0., 0.], [16., 0.2, 0.1]),
            (&cut.upper, [16., 0.2, 0.1], [40., 0., 0.]),
        ] {
            let original = format!("{:?}", old.solid());
            let enlarged = old.partition_coaxial_bore_enlargement(6. * s, p).unwrap();
            retained(old.solid(), enlarged.kept.solid());
            verify(&enlarged.kept, f, s, lo, hi, 6.);
            assert_eq!(
                format!("{:?}", &old.solid().edges[12..]),
                format!("{:?}", &enlarged.removed.solid().edges[12..])
            );
            assert!(
                (enlarged.kept.volume(p).unwrap() + enlarged.removed.volume(p).unwrap()
                    - old.volume(p).unwrap())
                .abs()
                    < 2e-9 * old.volume(p).unwrap()
            );
            assert_eq!(original, format!("{:?}", old.solid()));
        }
        let slab = cut.upper.split_by_parallel_plane(&next, p).unwrap().lower;
        let enlarged = slab.partition_coaxial_bore_enlargement(6. * s, p).unwrap();
        retained(slab.solid(), enlarged.kept.solid());
        verify(&enlarged.kept, f, s, [16., 0.2, 0.1], [24., 0.2, 0.1], 6.);
        assert_eq!(
            format!("{:?}", &slab.solid().edges[12..]),
            format!("{:?}", &enlarged.removed.solid().edges[12..])
        );
    }
}
#[test]
fn canonical_enlargement_and_invalid_adapter_requests_remain_immutable() {
    let p = policy(1.);
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 40., p).unwrap();
    let bored = source.bore_coaxial(4., p).unwrap();
    let before = format!("{:?}", bored.solid());
    let part = bored.partition_coaxial_bore_enlargement(6., p).unwrap();
    retained(bored.solid(), part.kept.solid());
    compare(
        part.kept.inertia_properties(p).unwrap(),
        oracle(Frame3::IDENTITY, 1., 6., [0., 0., 0.], [40., 0., 0.]),
        1.,
    );
    assert!(
        (part.kept.volume(p).unwrap() + part.removed.volume(p).unwrap() - bored.volume(p).unwrap())
            .abs()
            < 2e-9 * bored.volume(p).unwrap()
    );
    for radius in [0., 3., 4., 4. + 1e-7, 12., f64::NAN, f64::INFINITY] {
        assert!(bored.enlarge_coaxial_bore(radius, p).is_err());
    }
    let plane = plane(Frame3::IDENTITY, 1., 16., 0.2, 0.1);
    let old = bored.split_by_plane(&plane, p).unwrap().lower;
    let previous = format!("{:?}", old.solid());
    let step = old.export_step_mm(p).unwrap();
    for radius in [0., 3., 4., 4. + 1e-7, 12., f64::NAN, f64::INFINITY] {
        assert!(old.enlarge_coaxial_bore(radius, p).is_err());
    }
    assert!(old
        .enlarge_coaxial_bore(4.1, GeometryTolerance::new(1e-6, 1e-10, 0.01).unwrap())
        .is_err());
    assert_eq!(previous, format!("{:?}", old.solid()));
    assert_eq!(step, old.export_step_mm(p).unwrap());
    assert_eq!(before, format!("{:?}", bored.solid()));
    assert!(old.enlarge_coaxial_bore(6., p).is_ok());
}
