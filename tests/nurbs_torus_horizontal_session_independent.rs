use hagane::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap()
}
fn json(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}
fn point(v: &Value) -> Point3 {
    Point3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}
fn integrated_volume(major: f64, minor: f64, height: f64) -> f64 {
    // Integrate annular cross-section area in z = minor*sin(angle), avoiding
    // square-root endpoint singularities without using the volume primitive.
    let a = -std::f64::consts::FRAC_PI_2;
    let b = (height / minor).asin();
    let n = 4096;
    let dx = (b - a) / n as f64;
    let f = |t: f64| 4. * std::f64::consts::PI * major * minor * minor * t.cos().powi(2);
    let mut sum = f(a) + f(b);
    for i in 1..n {
        sum += if i % 2 == 0 { 2. } else { 4. } * f(a + dx * i as f64);
    }
    sum * dx / 3.
}
fn check_exact_child(child: &NurbsTorusHorizontalSolid, report: &Value) {
    child.validate(policy()).unwrap();
    assert_eq!(child.export_step_mm(policy()).unwrap(), report["step"]);
    let frame = child.source().frame();
    let mut patches = 0;
    for face in &child.solid().shell.faces {
        if let Surface::Nurbs(surface) = &face.surface {
            patches += 1;
            assert_eq!(surface.degrees(), [2, 2]);
            assert!(surface.weights().iter().all(|w| w.is_finite() && *w > 0.));
            let domain = surface.domain();
            for i in 0..=8 {
                for j in 0..=8 {
                    let p = frame.local_point(
                        surface
                            .evaluate(
                                domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 8.,
                                domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / 8.,
                            )
                            .unwrap(),
                    );
                    assert!(((p.x.hypot(p.y) - 16.).hypot(p.z) - 4.).abs() < 1e-7);
                    assert!(if child.is_lower() {
                        p.z <= child.cut_height() + 1e-7
                    } else {
                        p.z >= child.cut_height() - 1e-7
                    });
                }
            }
        }
        for wire in &face.wires {
            for coedge in &wire.coedges {
                let edge = &child.solid().edges[coedge.edge];
                let range = match &edge.curve {
                    Curve::Nurbs(c) => c.domain(),
                    _ => [0., 1.],
                };
                for i in 0..=16 {
                    let t = range[0] + (range[1] - range[0]) * i as f64 / 16.;
                    let uv = coedge.pcurve.try_evaluate(t).unwrap();
                    let p = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                    assert!((p - edge.curve.try_evaluate(t).unwrap()).norm() < 1e-7);
                }
            }
        }
    }
    assert!(patches >= 8);
    let cap = child
        .solid()
        .shell
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::Plane { .. }))
        .unwrap();
    assert_eq!(cap.wires.len(), 2);
}
fn check_mesh(report: &Value, frame: Frame3, height: f64, lower: bool) {
    let mesh = &report["mesh"];
    let positions = mesh["positions"].as_array().unwrap();
    let normals = mesh["normals"].as_array().unwrap();
    let triangles = mesh["triangles"].as_array().unwrap();
    assert!(!triangles.is_empty());
    let mut nodes = BTreeMap::new();
    let mut ids = Vec::new();
    for p in positions {
        let p = point(p);
        let local = frame.local_point(p);
        assert!(if lower {
            local.z <= height + 1e-6
        } else {
            local.z >= height - 1e-6
        });
        let key = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
        let next = nodes.len();
        ids.push(*nodes.entry(key).or_insert(next));
    }
    let mut edges: BTreeMap<(usize, usize), (usize, i32)> = BTreeMap::new();
    let mut cut_vertices = 0;
    for (i, p) in positions.iter().enumerate() {
        let p = frame.local_point(point(p));
        let n = frame.local_vector(point(&normals[i]) - Point3::new(0., 0., 0.));
        if (p.z - height).abs() < 1e-7 && n.z.abs() > 0.999 {
            assert!(if lower { n.z > 0. } else { n.z < 0. });
            cut_vertices += 1;
        } else {
            let rho = p.x.hypot(p.y);
            let expected = Vec3::new(
                (rho - 16.) * p.x / (4. * rho),
                (rho - 16.) * p.y / (4. * rho),
                p.z / 4.,
            );
            assert!((n - expected).norm() < 1e-6);
        }
    }
    assert!(cut_vertices > 0);
    for triangle in triangles {
        let t = std::array::from_fn::<_, 3, _>(|k| triangle[k].as_u64().unwrap() as usize);
        let [a, b, c] = t.map(|i| point(&positions[i]));
        assert!((b - a).cross(c - a).norm() > 0.);
        for k in 0..3 {
            let a = ids[t[k]];
            let b = ids[t[(k + 1) % 3]];
            let edge = edges.entry((a.min(b), a.max(b))).or_default();
            edge.0 += 1;
            edge.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|v| *v == (2, 0)));
    assert_eq!(
        nodes.len() as isize - edges.len() as isize + triangles.len() as isize,
        0
    );
    let step = report["step"].as_str().unwrap();
    assert!(step.contains("RATIONAL_B_SPLINE_SURFACE") && step.contains("FACE_BOUND"));
    assert_eq!(report["step_exact"], true);
}

#[test]
fn preview_children_match_annular_section_integration_and_retained_full_source() {
    let mut session = NurbsTorusDemoSession::new();
    assert!(session.split_horizontal(0.7, 0.1).is_err());
    session
        .apply(&[16., 4., 0., 0., 0., 0., 1e-6, 0.1])
        .unwrap();
    let body = session.accepted_body().unwrap();
    let frame = body.frame();
    let snapshot = format!("{:?}", body.solid());
    let step = body.export_step_mm(policy()).unwrap();
    let point_result = session.query_point(Point3::new(16., 0., 0.)).unwrap();
    let segment_result = session
        .intersect_segment(Point3::new(-24., 0., 0.), Point3::new(24., 0., 0.))
        .unwrap();
    let line_result = session
        .intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
        .unwrap();
    let ray_result = session
        .intersect_ray(Point3::new(16., 0., 0.), Vec3::new(2., 0., 0.))
        .unwrap();
    let report = json(session.split_horizontal(0.7, 0.1).unwrap());
    let lower = integrated_volume(16., 4., 0.7);
    let total = integrated_volume(16., 4., 4.);
    assert!((report["lower"]["volume"].as_f64().unwrap() - lower).abs() < 1e-7);
    assert!((report["upper"]["volume"].as_f64().unwrap() - (total - lower)).abs() < 1e-7);
    assert!((report["source_volume"].as_f64().unwrap() - total).abs() < 1e-7);
    check_mesh(&report["lower"], frame, 0.7, true);
    check_mesh(&report["upper"], frame, 0.7, false);
    let exact = session
        .accepted_body()
        .unwrap()
        .split_by_local_z(0.7, policy())
        .unwrap();
    check_exact_child(&exact.lower, &report["lower"]);
    check_exact_child(&exact.upper, &report["upper"]);
    assert_eq!(
        format!("{:?}", session.accepted_body().unwrap().solid()),
        snapshot
    );
    assert_eq!(
        session
            .accepted_body()
            .unwrap()
            .export_step_mm(policy())
            .unwrap(),
        step
    );
    assert_eq!(
        session.query_point(Point3::new(16., 0., 0.)).unwrap(),
        point_result
    );
    assert_eq!(
        session
            .intersect_segment(Point3::new(-24., 0., 0.), Point3::new(24., 0., 0.))
            .unwrap(),
        segment_result
    );
    assert_eq!(
        session
            .intersect_line(Point3::new(0., 0., 0.), Vec3::new(2., 0., 0.))
            .unwrap(),
        line_result
    );
    assert_eq!(
        session
            .intersect_ray(Point3::new(16., 0., 0.), Vec3::new(2., 0., 0.))
            .unwrap(),
        ray_result
    );
}

#[test]
fn posed_restricted_children_and_failed_preview_leave_accepted_queries_unchanged() {
    let mut session = NurbsTorusDemoSession::new();
    session
        .apply(&[16., 4., 0.37, 12., -5., 8., 1e-6, 0.1])
        .unwrap();
    let body = session.accepted_body().unwrap();
    let frame = body.frame();
    let snapshot = format!("{:?}", body.solid());
    let step = body.export_step_mm(policy()).unwrap();
    let query = frame.point(Point3::new(16., 0., 0.));
    let a = frame.point(Point3::new(-24., 2., 0.));
    let b = frame.point(Point3::new(24., 2., 0.));
    let direction = frame.vector(Vec3::new(2., 0., 0.));
    let prior = [
        session.query_point(query).unwrap(),
        session.intersect_segment(a, b).unwrap(),
        session.intersect_line(a, direction).unwrap(),
        session.intersect_ray(a, direction).unwrap(),
    ];
    let report = json(session.split_horizontal(-0.7, 0.1).unwrap());
    check_mesh(&report["lower"], frame, -0.7, true);
    check_mesh(&report["upper"], frame, -0.7, false);
    let exact = session
        .accepted_body()
        .unwrap()
        .split_by_local_z(-0.7, policy())
        .unwrap();
    check_exact_child(&exact.lower, &report["lower"]);
    check_exact_child(&exact.upper, &report["upper"]);
    for (height, chord) in [
        (4., 0.1),
        (-4., 0.1),
        (f64::NAN, 0.1),
        (0.7, 1e-15),
        (0.7, -1.),
    ] {
        assert!(session.split_horizontal(height, chord).is_err());
    }
    assert_eq!(
        format!("{:?}", session.accepted_body().unwrap().solid()),
        snapshot
    );
    assert_eq!(
        session
            .accepted_body()
            .unwrap()
            .export_step_mm(policy())
            .unwrap(),
        step
    );
    assert_eq!(
        [
            session.query_point(query).unwrap(),
            session.intersect_segment(a, b).unwrap(),
            session.intersect_line(a, direction).unwrap(),
            session.intersect_ray(a, direction).unwrap()
        ],
        prior
    );
}
