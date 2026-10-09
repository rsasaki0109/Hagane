use hagane::*;
fn surface() -> NurbsSurface {
    let controls = [(1., 0.), (1., 1.), (0., 1.)];
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (i, (x, y)) in controls.into_iter().enumerate() {
        for z in [0., 2.] {
            points.push(Point3::new(x, y, z));
            weights.push(if i == 1 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.
            });
        }
    }
    NurbsSurface::new(
        [2, 1],
        [vec![2., 2., 2., 6., 6., 6.], vec![-3., -3., 5., 5.]],
        [3, 2],
        points,
        weights,
    )
    .unwrap()
}
#[test]
fn exact_open_brep_shares_corners_and_surface_parameters() {
    let patch = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    patch.validate_boundary(Tolerance::default()).unwrap();
    assert!(matches!(patch.face.surface, Surface::Nurbs(_)));
    for (i, coedge) in patch.face.wires[0].coedges.iter().enumerate() {
        assert_eq!(coedge.edge, i);
        let edge = &patch.edges[i];
        let [a, b] = edge.curve.range();
        for j in 0..=32 {
            let t = a + (b - a) * j as f64 / 32.;
            let [u, v] = coedge.pcurve.evaluate(t);
            let p = edge.curve.try_evaluate(t).unwrap();
            assert!((p - patch.face.surface.try_evaluate(u, v).unwrap()).norm() < 1e-13);
            assert!((p.x.hypot(p.y) - 1.).abs() < 1e-13);
        }
        let end = edge.vertices[usize::from(coedge.forward)];
        let next = &patch.face.wires[0].coedges[(i + 1) % 4];
        assert_eq!(
            end,
            patch.edges[next.edge].vertices[usize::from(!next.forward)]
        );
    }
}
#[test]
fn reversed_face_mesh_has_reversed_winding_and_normals() {
    let a = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    let b = NurbsFace::new(surface(), -1, Tolerance::default()).unwrap();
    let ma = a.sample_grid([16, 4], Tolerance::default()).unwrap();
    let mb = b.sample_grid([16, 4], Tolerance::default()).unwrap();
    assert_eq!(ma.positions, mb.positions);
    for (a, b) in ma.normals.iter().zip(&mb.normals) {
        assert!((*a + *b).norm() < 1e-15);
    }
    for (i, t) in mb.triangles.iter().enumerate() {
        assert_eq!(
            *t,
            [ma.triangles[i][0], ma.triangles[i][2], ma.triangles[i][1]]
        );
        let cross = (mb.positions[t[1]] - mb.positions[t[0]])
            .cross(mb.positions[t[2]] - mb.positions[t[0]]);
        assert!(cross.dot(mb.normals[t[0]]) > 0.);
    }
    assert!(a.sample_grid([0, 4], Tolerance::default()).is_err());
}
#[test]
fn mutation_is_rejected_before_display_including_interior_geometry() {
    let original = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    let rejects = |p: NurbsFace| {
        assert!(p.validate_boundary(Tolerance::default()).is_err());
        assert!(p.sample_grid([8, 4], Tolerance::default()).is_err());
    };
    let mut p = original.clone();
    p.edges[0].vertices[1] = 100;
    rejects(p);
    let mut p = original.clone();
    p.face.wires[0].coedges[0].edge = 100;
    rejects(p);
    let mut p = original.clone();
    p.face.wires[0].coedges[2].forward = true;
    rejects(p);
    let mut p = original.clone();
    p.vertices[0].point.x = f64::NAN;
    rejects(p);
    let mut p = original.clone();
    p.face.orientation = 0;
    rejects(p);
    let mut p = original.clone();
    p.face.wires.clear();
    rejects(p);
    let mut p = original.clone();
    p.face.wires[0].coedges[0].pcurve = PCurve::Affine {
        origin: [0.1, -3.],
        direction: [1., 0.],
    };
    rejects(p);
    let mut p = original.clone();
    let Curve::Nurbs(c) = &p.edges[0].curve else {
        panic!()
    };
    let mut points = c.control_points().to_vec();
    points[1].x += 0.01;
    p.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(c.degree(), c.knots().to_vec(), points, c.weights().to_vec()).unwrap(),
    ));
    rejects(p);
    let mut p = original.clone();
    let Curve::Nurbs(c) = &p.edges[0].curve else {
        panic!()
    };
    let mut weights = c.weights().to_vec();
    weights[1] *= 0.5;
    p.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            c.degree(),
            c.knots().to_vec(),
            c.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    rejects(p);
    assert!(NurbsFace::new(surface(), 0, Tolerance::default()).is_err());
    assert!(original
        .validate_boundary(Tolerance { linear: f64::NAN })
        .is_err());
}
#[test]
fn checked_geometry_dispatch_and_rigid_placement() {
    let patch = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.7).unwrap();
    let transform = Transform::translation(Vec3::new(10., -4., 7.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let placed = patch.transformed(transform, Tolerance::default()).unwrap();
    placed.validate_boundary(Tolerance::default()).unwrap();
    for i in 0..=12 {
        let u = 2. + i as f64 / 3.;
        let v = -1.;
        let p = patch.face.surface.try_evaluate(u, v).unwrap();
        let q = placed.face.surface.try_evaluate(u, v).unwrap();
        assert!((q - transform.point(p)).norm() < 1e-12);
    }
    let edge = &patch.edges[0].curve;
    let moved = edge.transformed(transform).unwrap();
    assert!(
        (moved.try_evaluate(4.).unwrap() - transform.point(edge.try_evaluate(4.).unwrap())).norm()
            < 1e-12
    );
    assert!(edge.try_evaluate(1.).is_err());
    assert!(!edge.evaluate(1.).finite());
    assert!(patch.face.surface.try_evaluate(1., 0.).is_err());
    assert!(
        (patch.face.surface.normal_at(4., 0.).unwrap()
            - Vec3::new(
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.
            ))
        .norm()
            < 1e-13
    );
    assert!(matches!(
        patch.face.surface.try_parameters(Point3::new(1., 0., 0.)),
        Err(Error::Unsupported(_))
    ));
    assert!(!patch.face.surface.normal(4.).finite());
}
#[test]
fn open_nurbs_face_cannot_be_promoted_to_a_solid() {
    let p = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    let solid = Solid {
        vertices: p.vertices.to_vec(),
        edges: p.edges.to_vec(),
        shell: Shell {
            faces: vec![p.face],
        },
    };
    assert!(matches!(
        solid.validate(Tolerance::default()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(solid.volume(), Err(Error::Unsupported(_))));
    assert!(solid.tessellate(0.01, Tolerance::default()).is_err());
    assert!(classify_point_in_solid(
        &solid,
        Point3::new(0., 0., 0.),
        GeometryTolerance::default()
    )
    .is_err());
    assert!(export_step_mm(&solid, Tolerance::default()).is_err());
}
#[test]
fn singular_surface_has_no_invented_display_mesh() {
    let s = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![Point3::new(0., 0., 0.); 4],
        vec![1.; 4],
    )
    .unwrap();
    let face = NurbsFace::new(s, 1, Tolerance::default()).unwrap();
    // Boundary structure alone says nothing about regularity or injectivity.
    assert!(face.sample_grid([8, 8], Tolerance::default()).is_err());
}
#[test]
fn bounded_face_mesh_respects_orientation_and_rejects_dirty_boundaries() {
    let a = NurbsFace::new(surface(), 1, Tolerance::default()).unwrap();
    let b = NurbsFace::new(surface(), -1, Tolerance::default()).unwrap();
    let ma = a
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .unwrap();
    let mb = b
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .unwrap();
    assert_eq!(ma.mesh.positions, mb.mesh.positions);
    assert_eq!(ma.uv_ranges, mb.uv_ranges);
    assert_eq!(ma.error_bounds, mb.error_bounds);
    assert!(ma.error_bounds.iter().all(|bound| *bound <= 0.01));
    for (a, b) in ma.mesh.triangles.iter().zip(&mb.mesh.triangles) {
        assert_eq!(*b, [a[0], a[2], a[1]]);
    }
    for (a, b) in ma.mesh.normals.iter().zip(&mb.mesh.normals) {
        assert!((*a + *b).norm() < 1e-15);
    }
    let mut dirty = a;
    dirty.edges[0].vertices[0] = 100;
    assert!(dirty
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .is_err());
}
#[test]
fn bounded_multispan_face_preserves_orientation_and_rejects_c0_lines() {
    let smooth = surface()
        .insert_knot(0, 3., 1)
        .unwrap()
        .insert_knot(0, 5., 1)
        .unwrap();
    let a = NurbsFace::new(smooth.clone(), 1, Tolerance::default()).unwrap();
    let b = NurbsFace::new(smooth, -1, Tolerance::default()).unwrap();
    let ma = a
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .unwrap();
    let mb = b
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .unwrap();
    assert_eq!(ma.mesh.positions, mb.mesh.positions);
    assert_eq!(ma.error_bounds, mb.error_bounds);
    for (a, b) in ma.mesh.triangles.iter().zip(&mb.mesh.triangles) {
        assert_eq!(*b, [a[0], a[2], a[1]]);
    }
    let c0 = NurbsFace::new(
        surface().insert_knot(0, 4., 2).unwrap(),
        1,
        Tolerance::default(),
    )
    .unwrap();
    c0.validate_boundary(Tolerance::default()).unwrap();
    assert!(matches!(
        c0.tessellate_bounded(0.01, 4096, Tolerance::default()),
        Err(Error::Unsupported(_))
    ));
}
