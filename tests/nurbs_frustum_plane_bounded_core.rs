use hagane::*;
fn plane(z: f64, n: Vec3) -> Surface {
    let normal = n.normalized().unwrap();
    let u = Vec3::new(-normal.y, normal.x, 0.).normalized().unwrap();
    Surface::Plane {
        origin: Point3::new(0., 0., z),
        u,
        v: normal.cross(u),
    }
}
#[test]
fn genuine_skew_sections_retain_raw_rims_and_reject_crossing_caps() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let original = format!("{:?}", source.solid());
    let old_plane = plane(12., Vec3::new(0.2, 0., 1.));
    let old = source.split_by_plane(&old_plane, p).unwrap().upper;
    let new_plane = plane(18., Vec3::new(-0.1, 0.05, 1.));
    assert!(matches!(
        old.split_by_parallel_plane(&new_plane, p),
        Err(Error::Unsupported(_))
    ));
    let children = old.split_by_interior_plane(&new_plane, p).unwrap();
    assert!(
        (children.lower.volume(p).unwrap() + children.upper.volume(p).unwrap()
            - old.volume(p).unwrap())
        .abs()
            < 1e-9
    );
    assert_eq!(
        format!("{:?}", &children.lower.solid().edges[..4]),
        format!("{:?}", old.section().edges())
    );
    assert_eq!(
        format!("{:?}", children.lower.solid().shell.faces[0]),
        format!("{:?}", old.solid().shell.faces[0])
    );
    assert_eq!(
        (
            children.lower.solid().vertices.len(),
            children.lower.solid().edges.len(),
            children.lower.solid().shell.faces.len()
        ),
        (8, 12, 6)
    );
    for face in &children.lower.solid().shell.faces {
        for coedge in &face.wires[0].coedges {
            for k in 0..33 {
                let t = k as f64 / 32.;
                let uv = coedge.pcurve.try_evaluate(t).unwrap();
                let a = face.surface.try_evaluate(uv[0], uv[1]).unwrap();
                let b = children.lower.solid().edges[coedge.edge]
                    .curve
                    .try_evaluate(t)
                    .unwrap();
                assert!((a - b).norm() < 1e-10);
            }
        }
    }
    let repeated = children
        .lower
        .split_by_interior_plane(&plane(15., Vec3::new(0.02, 0.03, 1.)), p)
        .unwrap();
    assert!(
        (repeated.lower.volume(p).unwrap() + repeated.upper.volume(p).unwrap()
            - children.lower.volume(p).unwrap())
        .abs()
            < 1e-9
    );
    assert!(matches!(
        old.split_by_interior_plane(&plane(12., Vec3::new(-0.5, 0., 1.)), p),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        children.lower.split_by_interior_plane(&new_plane, p),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(original, format!("{:?}", source.solid()));
}
#[test]
fn annular_skew_children_keep_all_inner_wall_and_cap_incidence() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [12., 12.], 40., p)
        .unwrap()
        .bore_coaxial(4., p)
        .unwrap();
    let old = source
        .split_by_plane(&plane(12., Vec3::new(0.2, 0.1, 1.)), p)
        .unwrap()
        .upper;
    let split = old
        .split_by_interior_plane(&plane(24., Vec3::new(-0.1, 0.15, 1.)), p)
        .unwrap();
    assert!(
        (split.lower.volume(p).unwrap() - std::f64::consts::PI * (144. - 16.) * 12.).abs() < 1e-8
    );
    let body = split.lower.solid();
    assert_eq!(
        (
            body.vertices.len(),
            body.edges.len(),
            body.shell.faces.len()
        ),
        (16, 24, 10)
    );
    let mut uses = [(0, 0); 24];
    for face in &body.shell.faces {
        for wire in &face.wires {
            for coedge in &wire.coedges {
                uses[coedge.edge].0 += 1;
                uses[coedge.edge].1 +=
                    i32::from(face.orientation) * if coedge.forward { 1 } else { -1 };
            }
        }
    }
    assert!(uses.iter().all(|&u| u == (2, 0)));
    assert_eq!(body.shell.faces[0].wires.len(), 2);
    assert_eq!(body.shell.faces[1].wires.len(), 2);
    assert!(split
        .lower
        .export_step_mm(p)
        .unwrap()
        .contains("RATIONAL_B_SPLINE_SURFACE"));
}
