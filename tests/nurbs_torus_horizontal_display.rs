use hagane::*;
use std::collections::BTreeMap;
fn angle(t: f64) -> f64 {
    2. * (std::f64::consts::FRAC_1_SQRT_2 * t / (1. - (1. - std::f64::consts::FRAC_1_SQRT_2) * t))
        .atan()
}
fn parameter(a: f64) -> f64 {
    let q = (a / 2.).tan();
    q / (std::f64::consts::FRAC_1_SQRT_2 + (1. - std::f64::consts::FRAC_1_SQRT_2) * q)
}
fn delta(a: f64, b: f64) -> f64 {
    (a - b).sin().atan2((a - b).cos())
}
#[test]
fn actual_restricted_walls_and_annular_cap_are_bounded_and_bit_closed() {
    let p = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.37).unwrap();
    let frame = Frame3::new_with_tolerance(
        Point3::new(12., -5., 8.),
        [
            rotation.vector(Vec3::new(1., 0., 0.)),
            rotation.vector(Vec3::new(0., 1., 0.)),
            rotation.vector(Vec3::new(0., 0., 1.)),
        ],
        p,
    )
    .unwrap();
    let source = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
    for cut in [0., 1.25, -1.25] {
        let split = source.split_by_local_z(cut, p).unwrap();
        for body in [&split.lower, &split.upper] {
            let mesh = body.tessellate(0.1, p).unwrap();
            let cap = body.solid().shell.faces.len() - 1;
            let mut nodes = BTreeMap::new();
            let ids: Vec<_> = mesh
                .positions
                .iter()
                .map(|v| {
                    let next = nodes.len();
                    *nodes
                        .entry([v.x.to_bits(), v.y.to_bits(), v.z.to_bits()])
                        .or_insert(next)
                })
                .collect();
            let mut edges: BTreeMap<(usize, usize), (usize, isize)> = BTreeMap::new();
            let inner = 16. - (16. - cut * cut).sqrt();
            for (ti, tri) in mesh.triangles.iter().enumerate() {
                for k in 0..3 {
                    let a = ids[tri[k]];
                    let b = ids[tri[(k + 1) % 3]];
                    assert_ne!(a, b);
                    let entry = edges.entry((a.min(b), a.max(b))).or_default();
                    entry.0 += 1;
                    entry.1 += if a < b { 1 } else { -1 };
                }
                let points = tri.map(|i| mesh.positions[i]);
                let faceid = mesh.face_ids[ti];
                if faceid == cap {
                    for weights in [[1. / 3.; 3], [0.5, 0.5, 0.], [0., 0.5, 0.5], [0.5, 0., 0.5]] {
                        let x = points[0] * weights[0]
                            + points[1] * weights[1]
                            + points[2] * weights[2];
                        let local = frame.local_point(x);
                        assert!((local.z - cut).abs() < 1e-8);
                        assert!(local.x.hypot(local.y) >= inner - 0.1);
                    }
                    for id in tri {
                        assert!(
                            mesh.normals[*id].dot(frame.axes()[2])
                                * (if body.is_lower() { 1. } else { -1. })
                                > 0.999999
                        );
                    }
                } else {
                    let face = &body.solid().shell.faces[faceid];
                    let Surface::Nurbs(surface) = &face.surface else {
                        panic!()
                    };
                    let domain = surface.domain();
                    let base = frame.local_point(surface.evaluate(0., domain[1][0]).unwrap());
                    let major_base = base.y.atan2(base.x);
                    let minor_base = base.z.atan2(base.x.hypot(base.y) - 16.) - angle(domain[1][0]);
                    let angles = points.map(|x| {
                        let q = frame.local_point(x);
                        [
                            delta(q.y.atan2(q.x), major_base),
                            delta(q.z.atan2(q.x.hypot(q.y) - 16.), minor_base),
                        ]
                    });
                    for weights in [
                        [1. / 3.; 3],
                        [0.5, 0.25, 0.25],
                        [0.25, 0.5, 0.25],
                        [0.25, 0.25, 0.5],
                    ] {
                        let a = (0..3).map(|i| weights[i] * angles[i][0]).sum::<f64>();
                        let b = (0..3).map(|i| weights[i] * angles[i][1]).sum::<f64>();
                        let u = parameter(a).clamp(0., 1.);
                        let v = parameter(b).clamp(domain[1][0], domain[1][1]);
                        let actual = surface.evaluate(u, v).unwrap();
                        let linear = points[0] * weights[0]
                            + points[1] * weights[1]
                            + points[2] * weights[2];
                        assert!(
                            (actual - linear).norm() <= 0.1,
                            "whole triangle actual angular chart"
                        );
                    }
                }
            }
            assert!(edges.values().all(|&(n, d)| n == 2 && d == 0));
            assert_eq!(
                nodes.len() as isize - edges.len() as isize + mesh.triangles.len() as isize,
                0
            );
            let volume = body.volume(p).unwrap();
            assert!(mesh.signed_volume() > 0.);
            assert!((mesh.signed_volume() - volume).abs() < volume * 0.03);
            assert!(body.tessellate(1e-15, p).is_err());
        }
    }
}
