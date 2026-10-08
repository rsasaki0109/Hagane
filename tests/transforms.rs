use hagane::*;
use std::f64::consts::{FRAC_PI_2, PI};
fn tol() -> Tolerance {
    Tolerance::default()
}
fn close(a: Vec3, b: Vec3) {
    assert!((a - b).norm() < 1e-8, "{a:?} != {b:?}");
}
fn placement(angle: f64) -> Transform {
    Transform::translation(Vec3::new(20.0, -7.0, 12.0))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle).unwrap())
        .unwrap()
}
fn fixtures() -> Vec<Solid> {
    let block = BoxSpec {
        min: Point3::new(-4.0, -3.0, -1.0),
        size: Vec3::new(8.0, 6.0, 2.0),
    };
    vec![
        make_box(block, tol()).unwrap(),
        make_cylinder(
            CylinderSpec {
                base: Vec3::new(0.0, 0.0, -1.0),
                radius: 2.0,
                height: 2.0,
            },
            tol(),
        )
        .unwrap(),
        make_tube(
            TubeSpec {
                base: Vec3::new(0.0, 0.0, -1.0),
                outer_radius: 2.0,
                inner_radius: 1.0,
                height: 2.0,
            },
            tol(),
        )
        .unwrap(),
        subtract_through_cylinder(
            block,
            CylinderSpec {
                base: Vec3::new(0.0, 0.0, -2.0),
                radius: 1.0,
                height: 4.0,
            },
            tol(),
        )
        .unwrap(),
    ]
}
#[test]
fn frames_compose_and_invert_points_and_directions() {
    for angle in [0.0, 1e-12, 0.8, FRAC_PI_2, PI, -2.0] {
        let t = placement(angle);
        let inverse = t.inverse().unwrap();
        let p = Vec3::new(-12.0, 5.0, 0.3);
        close(inverse.point(t.point(p)), p);
        close(t.local_point(t.point(p)), p);
        close(t.local_vector(t.vector(p)), p);
        close(inverse.vector(t.vector(p)), p);
        close(t.compose(inverse).unwrap().point(p), p);
        assert!((t.vector(p).norm() - p.norm()).abs() < 1e-12);
    }
}
#[test]
fn invalid_frames_are_rejected_even_with_loose_linear_tolerance() {
    let basis = Transform::IDENTITY.axes();
    for epsilon in [1e-8, 1.0, 100.0] {
        for bad in [
            [basis[0] * 2.0, basis[1], basis[2]],
            [basis[0], basis[1], basis[2] * -1.0],
            [basis[0], basis[1], Vec3::new(0.1, 0.0, 0.99)],
            [basis[0], basis[0], basis[2]],
            [Vec3::new(f64::NAN, 0.0, 0.0), basis[1], basis[2]],
        ] {
            assert!(Transform::new(
                Vec3::new(0.0, 0.0, 0.0),
                bad,
                Tolerance::new(epsilon).unwrap()
            )
            .is_err());
        }
    }
    assert!(Transform::new(
        Vec3::new(0.0, 0.0, 0.0),
        basis,
        Tolerance { linear: f64::NAN }
    )
    .is_err());
    assert!(Transform::translation(Vec3::new(f64::INFINITY, 0.0, 0.0)).is_err());
    assert!(Transform::rotation(Vec3::new(0.0, 0.0, 0.0), 1.0).is_err());
    assert!(Transform::rotation(basis[0], f64::NAN).is_err());
}
#[test]
fn solid_placement_preserves_geometry_topology_pcurves_and_metrics() {
    for source in fixtures() {
        for angle in [0.0, 1e-12, 0.8, FRAC_PI_2, PI, -2.0] {
            let t = placement(angle);
            let moved = source.transformed(t, tol()).unwrap();
            moved.validate(tol()).unwrap();
            assert!((moved.volume().unwrap() - source.volume().unwrap()).abs() < 1e-9);
            assert_eq!(source.edges.len(), moved.edges.len());
            for (a, b) in source.edges.iter().zip(&moved.edges) {
                assert_eq!(a.vertices, b.vertices);
                for k in 0..=32 {
                    let [lo, hi] = a.curve.range();
                    let u = lo + (hi - lo) * k as f64 / 32.0;
                    close(t.point(a.curve.evaluate(u)), b.curve.evaluate(u));
                }
            }
            for (a, b) in source.shell.faces.iter().zip(&moved.shell.faces) {
                assert_eq!(a.orientation, b.orientation);
                for (wa, wb) in a.wires.iter().zip(&b.wires) {
                    for (ca, cb) in wa.coedges.iter().zip(&wb.coedges) {
                        assert_eq!((ca.edge, ca.forward), (cb.edge, cb.forward));
                        for u in [0.0, 0.5, 1.0] {
                            assert_eq!(ca.pcurve.evaluate(u), cb.pcurve.evaluate(u));
                        }
                    }
                }
            }
            let mesh = source.tessellate(0.01, tol()).unwrap();
            let moved_mesh = moved.tessellate(0.01, tol()).unwrap();
            assert_eq!(mesh.triangles, moved_mesh.triangles);
            assert_eq!(mesh.face_ids, moved_mesh.face_ids);
            for (a, b) in mesh.positions.iter().zip(&moved_mesh.positions) {
                close(t.point(*a), *b);
            }
            for (a, b) in mesh.normals.iter().zip(&moved_mesh.normals) {
                close(t.vector(*a), *b);
            }
            assert!((mesh.signed_volume() - moved_mesh.signed_volume()).abs() < 1e-8);
            let back = moved.transformed(t.inverse().unwrap(), tol()).unwrap();
            for (a, b) in source.vertices.iter().zip(back.vertices) {
                close(a.point, b.point);
            }
        }
    }
}
#[test]
fn tilted_cylinder_bounds_include_exact_circle_extrema() {
    let source = fixtures().remove(1);
    let t = placement(0.8);
    let moved = source.transformed(t, tol()).unwrap();
    let bounds = moved.bounds();
    let [u, v, _] = t.axes();
    let extent = Vec3::new(u.x.hypot(v.x), u.y.hypot(v.y), u.z.hypot(v.z)) * 2.0;
    let centers = [
        t.point(Vec3::new(0.0, 0.0, -1.0)),
        t.point(Vec3::new(0.0, 0.0, 1.0)),
    ];
    close(
        bounds.min,
        Vec3::new(
            centers[0].x.min(centers[1].x),
            centers[0].y.min(centers[1].y),
            centers[0].z.min(centers[1].z),
        ) - extent,
    );
    close(
        bounds.max,
        Vec3::new(
            centers[0].x.max(centers[1].x),
            centers[0].y.max(centers[1].y),
            centers[0].z.max(centers[1].z),
        ) + extent,
    );
    for edge in &moved.edges {
        for k in 0..=256 {
            let [a, b] = edge.curve.range();
            let p = edge.curve.evaluate(a + (b - a) * k as f64 / 256.0);
            assert!(
                p.x >= bounds.min.x - 1e-12
                    && p.x <= bounds.max.x + 1e-12
                    && p.y >= bounds.min.y - 1e-12
                    && p.y <= bounds.max.y + 1e-12
                    && p.z >= bounds.min.z - 1e-12
                    && p.z <= bounds.max.z + 1e-12
            );
        }
    }
}
#[test]
fn framed_surface_parameters_normals_and_chord_error() {
    let frame = placement(0.8);
    let surface = Surface::FramedCylinder {
        frame,
        radius: 2.0,
        height: 3.0,
    };
    for u in [0.1, 1.0, 3.0, 6.0] {
        let uv = surface.parameters(surface.evaluate(u, 1.2));
        assert!((uv[0] - u).abs() < 1e-13 && (uv[1] - 1.2).abs() < 1e-13);
        close(
            surface.normal(u),
            frame.vector(Vec3::new(u.cos(), u.sin(), 0.0)),
        );
    }
    let moved = fixtures().remove(1).transformed(frame, tol()).unwrap();
    let mesh = moved.tessellate(0.01, tol()).unwrap();
    for (tri, &face) in mesh.triangles.iter().zip(&mesh.face_ids) {
        if matches!(
            moved.shell.faces[face].surface,
            Surface::FramedCylinder { .. }
        ) {
            for j in 0..3 {
                let a = frame.local_point(mesh.positions[tri[j]]);
                let b = frame.local_point(mesh.positions[tri[(j + 1) % 3]]);
                let mid = (a + b) * 0.5;
                assert!(2.0 - mid.x.hypot(mid.y) <= 0.01 + 1e-12);
            }
        }
    }
}
#[test]
fn arbitrary_plane_extrusion_with_hole_and_reverse_normal_span() {
    let frame = placement(1.2);
    let profile = PolygonProfile {
        origin: Vec3::new(0.0, 0.0, 0.0),
        outer: vec![[0.0, 0.0], [5.0, 0.0], [5.0, 4.0], [0.0, 4.0]],
        holes: vec![vec![[1.0, 1.0], [2.0, 1.0], [2.0, 2.0], [1.0, 2.0]]],
    };
    for z in [-3.0, 3.0] {
        let direction = frame.vector(Vec3::new(0.4, 0.2, z));
        let s = extrude_polygon_in_frame(&profile, direction, frame, tol()).unwrap();
        s.validate(tol()).unwrap();
        assert!((s.volume().unwrap() - 57.0).abs() < 1e-10);
        assert!(s.tessellate(0.01, tol()).unwrap().signed_volume() > 0.0);
    }
    assert!(extrude_polygon_in_frame(
        &profile,
        frame.vector(Vec3::new(1.0, 2.0, 0.0)),
        frame,
        tol()
    )
    .is_err());
}
#[test]
fn corrupted_or_unrepresentable_placements_do_not_succeed() {
    let mut s = fixtures().remove(0);
    s.edges[0].vertices[0] = usize::MAX;
    assert!(s.transformed(Transform::IDENTITY, tol()).is_err());
    for s in fixtures() {
        assert!(s
            .transformed(
                Transform::translation(Vec3::new(1e30, 1e30, 1e30)).unwrap(),
                tol()
            )
            .is_err());
    }
    let t = Transform::translation(Vec3::new(f64::MAX, 0.0, 0.0)).unwrap();
    assert!(t.compose(t).is_err());
}

#[test]
fn rotation_has_right_handed_quarter_turn_and_supports_small_parts() {
    for magnitude in [1e-320, 1e-200, 1.0, 1e200] {
        let rotation = Transform::rotation(Vec3::new(magnitude, 0.0, 0.0), FRAC_PI_2).unwrap();
        close(
            rotation.vector(Vec3::new(0.0, 1.0, 0.0)),
            Vec3::new(0.0, 0.0, 1.0),
        );
    }
    let t = Transform::rotation(Vec3::new(1.0, 0.0, 0.0), FRAC_PI_2).unwrap();
    close(t.point(Vec3::new(0.0, 1.0, 0.0)), Vec3::new(0.0, 0.0, 1.0));
    let tolerance = Tolerance::new(1e-14).unwrap();
    let part = make_tube(
        TubeSpec {
            base: Vec3::new(0.0, 0.0, 0.0),
            outer_radius: 2e-6,
            inner_radius: 1e-6,
            height: 3e-6,
        },
        tolerance,
    )
    .unwrap();
    let moved = part.transformed(t, tolerance).unwrap();
    assert!((moved.volume().unwrap() - PI * 9e-18).abs() < 1e-30);
    moved.tessellate(1e-9, tolerance).unwrap();
}
