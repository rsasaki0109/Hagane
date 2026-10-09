use hagane::*;
#[test]
fn exact_split_preserves_volume_and_closed_curved_parts() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol)
        .unwrap()
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)
        .unwrap()
        .transformed(
            Transform::translation(Vec3::new(12., -7., 4.))
                .unwrap()
                .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4).unwrap())
                .unwrap(),
            tol,
        )
        .unwrap();
    for axis in 0..2 {
        let split = source.split_uv(axis, 0.4, tol).unwrap();
        split.validate(tol).unwrap();
        assert!(
            (split.negative.volume().unwrap() + split.positive.volume().unwrap()
                - source.volume().unwrap())
            .abs()
                < 1e-9
        );
        let Surface::Plane { origin, u, v } = split.plane else {
            panic!()
        };
        let normal = u.cross(v).normalized().unwrap();
        let Surface::Nurbs(section) = &split.section.face.surface else {
            panic!()
        };
        let domain = section.domain();
        for i in 0..21 {
            for j in 0..21 {
                let a = domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 20.;
                let b = j as f64 / 20.;
                let point = section.evaluate(a, b).unwrap();
                assert!((point - origin).dot(normal).abs() < 1e-12);
                assert!(
                    section.normal(a, b).unwrap().dot(normal)
                        * split.section.face.orientation as f64
                        > 0.999999999
                );
            }
        }
        for body in [&split.negative, &split.positive] {
            let display = body.tessellate_bounded(0.1, 65536, tol).unwrap();
            let mut incidence = std::collections::BTreeMap::<[usize; 2], (usize, i32)>::new();
            for tri in &display.mesh.triangles {
                for k in 0..3 {
                    let a = display.vertex_nodes[tri[k]];
                    let b = display.vertex_nodes[tri[(k + 1) % 3]];
                    let e = incidence.entry([a.min(b), a.max(b)]).or_default();
                    e.0 += 1;
                    e.1 += if a < b { 1 } else { -1 };
                }
            }
            assert!(incidence.values().all(|e| *e == (2, 0)));
        }
    }
}
#[test]
fn split_certificate_rejects_changed_section_plane_and_body() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tol).unwrap();
    let original = source.split_uv(0, 0.4, tol).unwrap();
    let mut changed = original.clone();
    changed.section.vertices[0].point.x += tol.linear / 4.;
    assert!(changed.validate(tol).is_err());
    let mut changed = original.clone();
    let Surface::Plane { origin, u, v } = changed.plane else {
        panic!()
    };
    changed.plane = Surface::Plane {
        origin: origin + Vec3::new(1e-12, 0., 0.),
        u,
        v,
    };
    assert!(changed.validate(tol).is_err());
    let mut changed = original.clone();
    changed.positive = source.trimmed_uv([[0.5, 1.], [0., 1.]], tol).unwrap();
    assert!(changed.validate(tol).is_err());
    for (axis, p) in [
        (2, 0.5),
        (0, f64::NAN),
        (0, 0.),
        (0, 1.),
        (1, 1.1),
        (1, 1e-16),
    ] {
        assert!(source.split_uv(axis, p, tol).is_err());
    }
}

#[test]
fn graph_split_plane_identity_rejects_unresolved_physical_tolerance() {
    let tiny_tol = Tolerance::new(1e-20).unwrap();
    let source = NurbsGraphSolid::new([8., 6., 2.], 1., tiny_tol).unwrap();
    assert!(source.split_uv(0, 0.37, tiny_tol).is_err());
    let tiny = NurbsGraphSolid::new([8e-12, 6e-12, 2e-12], 1e-12, tiny_tol).unwrap();
    let split = tiny.split_uv(0, 0.37, tiny_tol).unwrap();
    let Surface::Plane { origin, u, v } = split.plane else {
        panic!()
    };
    let Surface::Nurbs(surface) = &split.section.face.surface else {
        panic!()
    };
    let normal = u.cross(v).normalized().unwrap();
    for point in surface.control_points() {
        assert!((*point - origin).dot(normal).abs() <= tiny_tol.linear / 4.);
    }
}
