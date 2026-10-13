use hagane::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;
fn policy(s: f64) -> GeometryTolerance {
    GeometryTolerance::new(1e-6 * s, 1e-10, 0.).unwrap()
}
fn body(s: f64) -> NurbsTorusSolid {
    let rot = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let f =
        Frame3::new_with_tolerance(Point3::new(13., -7., 5.) * s, rot.axes(), policy(s)).unwrap();
    NurbsTorusSolid::new(f, [16. * s, 4. * s], policy(s)).unwrap()
}
fn integral(bounds: [f64; 2], s: f64) -> f64 {
    let lo = (bounds[0] / (4. * s)).asin();
    let hi = (bounds[1] / (4. * s)).asin();
    let n = 8192;
    let h = (hi - lo) / n as f64;
    let mut sum = 0.;
    for i in 0..=n {
        let w = if i == 0 || i == n {
            1.
        } else if i % 2 == 0 {
            2.
        } else {
            4.
        };
        sum += w * (lo + h * i as f64).cos().powi(2);
    }
    4. * PI * 16. * s * (4. * s).powi(2) * sum * h / 3.
}
fn topology(child: &NurbsTorusHorizontalSlab, s: f64) {
    let solid = child.solid();
    let heights = child.heights();
    let frame = child.source().frame();
    let mut uses = vec![(0, 0); solid.edges.len()];
    let mut caps = 0;
    let euler = solid.vertices.len() as isize - solid.edges.len() as isize
        + solid
            .shell
            .faces
            .iter()
            .map(|f| 2 - f.wires.len() as isize)
            .sum::<isize>();
    assert_eq!(
        euler, 0,
        "annular faces each have Euler characteristic zero"
    );
    let bounds = child.bounds(policy(s)).unwrap();
    let contains = |point: Point3| {
        assert!(
            point.x >= bounds.min.x
                && point.x <= bounds.max.x
                && point.y >= bounds.min.y
                && point.y <= bounds.max.y
                && point.z >= bounds.min.z
                && point.z <= bounds.max.z,
            "actual retained geometry {point:?} exceeds {bounds:?}"
        );
    };
    for edge in &solid.edges {
        if let Curve::Nurbs(curve) = &edge.curve {
            for &point in curve.control_points() {
                contains(point);
            }
        }
    }
    for vertex in &solid.vertices {
        let q = frame.local_point(vertex.point);
        assert!(q.z >= heights[0] - 1e-8 * s && q.z <= heights[1] + 1e-8 * s);
        assert!(
            vertex.point.x >= bounds.min.x
                && vertex.point.x <= bounds.max.x
                && vertex.point.y >= bounds.min.y
                && vertex.point.y <= bounds.max.y
                && vertex.point.z >= bounds.min.z
                && vertex.point.z <= bounds.max.z
        );
    }
    for face in &solid.shell.faces {
        match &face.surface {
            Surface::Nurbs(net) => {
                let d = net.domain();
                assert_eq!(d[0], [0., 1.]);
                assert!(d[1][0] >= 0. && d[1][1] <= 1.);
                let mid = net.evaluate(0.5, (d[1][0] + d[1][1]) / 2.).unwrap();
                let original = child
                    .source()
                    .solid()
                    .shell
                    .faces
                    .iter()
                    .find_map(|f| match &f.surface {
                        Surface::Nurbs(n)
                            if (n.evaluate(0.5, (d[1][0] + d[1][1]) / 2.).unwrap() - mid)
                                .norm()
                                < 1e-8 * s =>
                        {
                            Some(n)
                        }
                        _ => None,
                    })
                    .unwrap();
                for i in 0..=8 {
                    for j in 0..=8 {
                        let u = i as f64 / 8.;
                        let v = if j == 8 {
                            d[1][1]
                        } else {
                            d[1][0] + (d[1][1] - d[1][0]) * j as f64 / 8.
                        };
                        let p = net.evaluate(u, v).unwrap();
                        let q = frame.local_point(p);
                        assert!(((q.x.hypot(q.y) - 16. * s).hypot(q.z) - 4. * s).abs() < 1e-8 * s);
                        assert!(q.z >= heights[0] - 1e-8 * s && q.z <= heights[1] + 1e-8 * s);
                        assert!((p - original.evaluate(u, v).unwrap()).norm() < 1e-8 * s);
                    }
                }
            }
            Surface::Plane { .. } => {
                caps += 1;
                assert_eq!(face.wires.len(), 2);
                assert_eq!(face.wires.iter().map(|w| w.coedges.len()).sum::<usize>(), 8);
                let edge = &solid.edges[face.wires[0].coedges[0].edge];
                let q = frame.local_point(edge.curve.try_evaluate(0.).unwrap());
                let lower = (q.z - heights[0]).abs() < (q.z - heights[1]).abs();
                let normal = face.surface.normal_at(0., 0.).unwrap() * face.orientation as f64;
                assert!(normal.dot(frame.axes()[2]) * (if lower { -1. } else { 1. }) > 1. - 1e-10);
            }
            _ => panic!("only actual tensor walls and annular planes"),
        }
        for wire in &face.wires {
            for c in &wire.coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += face.orientation as i32 * if c.forward { 1 } else { -1 };
                let edge = &solid.edges[c.edge];
                if matches!(face.surface, Surface::Plane { .. }) {
                    if let PCurve::Nurbs(curve) = &c.pcurve {
                        for uv in curve.control_points() {
                            contains(face.surface.try_evaluate(uv.x, uv.y).unwrap());
                        }
                    }
                }
                let r = edge.curve.range();
                for k in 0..=16 {
                    let t = if k == 16 {
                        r[1]
                    } else {
                        r[0] + (r[1] - r[0]) * k as f64 / 16.
                    };
                    let uv = c.pcurve.try_evaluate(t).unwrap();
                    contains(face.surface.try_evaluate(uv[0], uv[1]).unwrap());
                    assert!(
                        (face.surface.try_evaluate(uv[0], uv[1]).unwrap()
                            - edge.curve.try_evaluate(t).unwrap())
                        .norm()
                            < 1e-8 * s
                    );
                }
            }
        }
    }
    assert_eq!(caps, 2);
    assert!(uses.iter().all(|&u| u == (2, 0)));
}
#[test]
fn positive_slice_quadrature_and_actual_two_cap_brep_hold_under_pose_and_scale() {
    for s in [1e-4, 1., 10.] {
        let source = body(s);
        let p = policy(s);
        let before = format!("{:?}", source.solid());
        let step = source.export_step_mm(p).unwrap();
        for range in [
            [-1.25, 1.25],
            [0., 1.25],
            [-1.25, 0.],
            [1., 2.5],
            [0.1, 0.3],
            [-0.3, -0.1],
            [-2.5, -1.],
            [3.998, 3.999],
        ] {
            let h = range.map(|v| v * s);
            let child = source.slab_between_local_z(h, p).unwrap();
            child.validate(p).unwrap();
            let expected = integral(h, s);
            assert!((child.volume(p).unwrap() - expected).abs() < expected * 2e-9);
            topology(&child, s);
            let exported = child.export_step_mm(p).unwrap();
            assert!(exported.contains("MANIFOLD_SOLID_BREP"));
            assert_eq!(
                exported.matches("RATIONAL_B_SPLINE_SURFACE(").count(),
                child.solid().shell.faces.len() - 2
            );
        }
        assert_eq!(before, format!("{:?}", source.solid()));
        assert_eq!(step, source.export_step_mm(p).unwrap());
    }
}
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
fn actual_slab_mesh_is_bit_welded_closed_and_dense_surface_chords_respect_requested_error() {
    let source = body(1.);
    let p = policy(1.);
    let frame = source.frame();
    let error = 0.1;
    for range in [[-1.25, 1.25], [1., 2.5], [0.1, 0.3]] {
        let child = source.slab_between_local_z(range, p).unwrap();
        let mesh = child.tessellate(error, p).unwrap();
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
        for (ti, tri) in mesh.triangles.iter().enumerate() {
            for k in 0..3 {
                let a = ids[tri[k]];
                let b = ids[tri[(k + 1) % 3]];
                assert_ne!(a, b);
                let e = edges.entry((a.min(b), a.max(b))).or_default();
                e.0 += 1;
                e.1 += if a < b { 1 } else { -1 };
            }
            let points = tri.map(|i| mesh.positions[i]);
            let face = &child.solid().shell.faces[mesh.face_ids[ti]];
            if let Surface::Nurbs(net) = &face.surface {
                let d = net.domain();
                let base = frame.local_point(net.evaluate(0., d[1][0]).unwrap());
                let major = base.y.atan2(base.x);
                let minor = base.z.atan2(base.x.hypot(base.y) - 16.) - angle(d[1][0]);
                let angles = points.map(|x| {
                    let q = frame.local_point(x);
                    [
                        delta(q.y.atan2(q.x), major),
                        delta(q.z.atan2(q.x.hypot(q.y) - 16.), minor),
                    ]
                });
                for i in 0..=4 {
                    for j in 0..=4 - i {
                        let w = [i as f64 / 4., j as f64 / 4., 1. - (i + j) as f64 / 4.];
                        let a = (0..3).map(|k| w[k] * angles[k][0]).sum();
                        let b = (0..3).map(|k| w[k] * angles[k][1]).sum();
                        let actual = net
                            .evaluate(
                                parameter(a).clamp(d[0][0], d[0][1]),
                                parameter(b).clamp(d[1][0], d[1][1]),
                            )
                            .unwrap();
                        let chord = points[0] * w[0] + points[1] * w[1] + points[2] * w[2];
                        assert!((actual - chord).norm() <= error);
                    }
                }
            } else {
                for w in [
                    [1., 0., 0.],
                    [0., 1., 0.],
                    [0., 0., 1.],
                    [1. / 3.; 3],
                    [0.5, 0.5, 0.],
                    [0.5, 0., 0.5],
                    [0., 0.5, 0.5],
                ] {
                    let point = points[0] * w[0] + points[1] * w[1] + points[2] * w[2];
                    let q = frame.local_point(point);
                    assert!((q.z - range[0]).abs() < 1e-8 || (q.z - range[1]).abs() < 1e-8);
                    let inner = 16. - (16. - q.z * q.z).sqrt();
                    assert!(q.x.hypot(q.y) >= inner - error);
                    let outer = 16. + (16. - q.z * q.z).sqrt();
                    assert!(q.x.hypot(q.y) <= outer + error);
                }
            }
        }
        assert!(edges.values().all(|&e| e == (2, 0)));
        assert_eq!(
            nodes.len() as isize - edges.len() as isize + mesh.triangles.len() as isize,
            0
        );
        assert!(mesh.signed_volume() > 0.);
        assert!(child.tessellate(1e-14, p).is_err());
    }
}
#[test]
fn invalid_ranges_contacts_nonfinite_and_original_relative_policy_fail_then_recover() {
    let source = body(1.);
    let p = policy(1.);
    let before = format!("{:?}", source.solid());
    let step = source.export_step_mm(p).unwrap();
    for range in [
        [1., 1.],
        [2., 1.],
        [-4., 1.],
        [1., 4.],
        [-5., 1.],
        [1., 1. + 1e-12],
        [-1e-12, 1.],
        [1., f64::NAN],
        [f64::NEG_INFINITY, 1.],
        [-1., 1e12],
    ] {
        assert!(
            source.slab_between_local_z(range, p).is_err(),
            "accepted {range:?}"
        );
    }
    let relative = GeometryTolerance::new(1e-6, 1e-10, 0.001).unwrap();
    assert!(source.slab_between_local_z([1., 1.001], relative).is_err());
    topology(&source.slab_between_local_z([-1., 1.], p).unwrap(), 1.);
    assert_eq!(before, format!("{:?}", source.solid()));
    assert_eq!(step, source.export_step_mm(p).unwrap());
}

#[test]
fn ordinary_partial_cap_control_hulls_include_actual_evaluations_near_world_precision_budget() {
    let p = policy(1.);
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(2000., -1000., 500.), rotation.axes(), p).unwrap();
    let source = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
    let before = format!("{:?}", source.solid());
    for heights in [[0.1, 0.3], [-0.3, -0.1]] {
        let slab = source.slab_between_local_z(heights, p).unwrap();
        topology(&slab, 1.);
    }
    assert_eq!(before, format!("{:?}", source.solid()));
}
