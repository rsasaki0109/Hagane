use hagane::*;
use std::f64::consts::PI;

fn cylinder(radius: f64, offset: f64) -> Solid {
    let t = Tolerance::default();
    make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -12.),
            radius,
            height: 24.,
        },
        t,
    )
    .unwrap()
    .transformed(
        Transform::translation(Vec3::new(offset, -offset, offset)).unwrap(),
        t,
    )
    .unwrap()
}

#[test]
fn closed_mesh_volume_is_stable_under_large_translation() {
    let t = Tolerance::default();
    let source = make_box(
        BoxSpec {
            min: Point3::new(-4., -4., -4.),
            size: Vec3::new(8., 8., 8.),
        },
        t,
    )
    .unwrap();
    let moved = source
        .transformed(
            Transform::translation(Vec3::new(1e12, -1e12, 1e12)).unwrap(),
            t,
        )
        .unwrap();
    assert!((moved.tessellate(0.05, t).unwrap().signed_volume() - 512.).abs() < 1e-9);
    let mut reversed = moved.tessellate(0.05, t).unwrap();
    for triangle in &mut reversed.triangles {
        triangle.swap(1, 2);
    }
    assert!((reversed.signed_volume() + 512.).abs() < 1e-9);
    let mut unused = source.tessellate(0.05, t).unwrap();
    unused.positions.insert(0, Point3::new(1e12, 1e12, 1e12));
    unused.normals.insert(0, Vec3::new(0., 0., 0.));
    for triangle in &mut unused.triangles {
        for v in triangle {
            *v += 1;
        }
    }
    assert!((unused.signed_volume() - 512.).abs() < 1e-9);
    assert_eq!(Mesh::default().signed_volume(), 0.);
}

#[test]
fn rounded_away_circle_seam_is_rejected_without_snapping() {
    let t = Tolerance::default();
    let local = cylinder(0.01, 0.);
    let transform = Transform::translation(Vec3::new(1e15, -1e15, 1e15)).unwrap();
    assert!(local.transformed(transform, t).is_err());
    // Deliberately construct the old rounded invalid boundary without using a checked transform.
    let mut source = local;
    for v in &mut source.vertices {
        v.point = transform.point(v.point);
    }
    for e in &mut source.edges {
        e.curve = e.curve.transformed(transform).unwrap();
    }
    for f in &mut source.shell.faces {
        f.surface = f.surface.transformed(transform).unwrap();
    }
    assert!(source.validate(t).is_err());
    assert!(certify_circular_prism(&source, t).is_err());
    assert!(export_step_mm(&source, t).is_err());
    let valid = export_step_mm(&cylinder(0.125, 1e15), t).unwrap();
    let invalid = valid
        .replace("1000000000000000.1", "1000000000000000.")
        .replace(",0.125)", ",0.01)");
    assert_ne!(valid, invalid);
    assert!(import_step_mm(&invalid, t).is_err());
}

#[test]
fn unresolved_display_precision_is_an_explicit_error() {
    let t = Tolerance::default();
    let source = cylinder(0.125, 1e15);
    certify_circular_prism(&source, t).unwrap(); // The cardinal seam is representable.
    let input = export_step_mm(&source, t).unwrap();
    import_step_mm(&input, t).unwrap(); // Exact geometry can be retained independently of display.
    assert!(matches!(
        source.tessellate(0.05, t),
        Err(Error::Tessellation(_))
    ));
    assert!(matches!(
        import_step_json(&input),
        Err(Error::Tessellation(_))
    ));
    // An explicit caller-selected local placement preserves the analytic shape.
    let local = source
        .transformed(
            Transform::translation(Vec3::new(-1e15, 1e15, -1e15)).unwrap(),
            t,
        )
        .unwrap();
    certify_circular_prism(&local, t).unwrap();
    assert!(local.tessellate(0.001, t).unwrap().signed_volume() > 0.);
}

#[test]
fn resolvable_far_cylinder_preserves_chord_bound_and_mesh_volume() {
    let t = Tolerance::default();
    let source = cylinder(8., 1e12);
    let imported = import_step_mm(&export_step_mm(&source, t).unwrap(), t).unwrap();
    let proof = certify_circular_prism(&imported, t).unwrap();
    let error = 0.05;
    let allowance = imported.tessellation_roundoff_budget(t).unwrap();
    assert!(allowance > 0.007 && allowance < 0.008);
    let mesh = imported.tessellate(error, t).unwrap();
    assert!(
        (mesh.signed_volume() - PI * 64. * 24.).abs() < std::f64::consts::TAU * 8. * 24. * error
    );
    for (triangle, &face) in mesh.triangles.iter().zip(&mesh.face_ids) {
        if matches!(imported.shell.faces[face].surface, Surface::Plane { .. }) {
            continue;
        }
        for k in 0..3 {
            let a = mesh.positions[triangle[k]];
            let b = mesh.positions[triangle[(k + 1) % 3]];
            let local = proof.axis.local_point(a + (b - a) * 0.5);
            assert!((local.x.hypot(local.y) - 8.).abs() <= error);
        }
    }
}

#[test]
fn cancelling_parameter_origin_is_guarded_even_with_small_world_bounds() {
    let t = Tolerance::default();
    let mut source = make_box(
        BoxSpec {
            min: Point3::new(-4., -4., -4.),
            size: Vec3::new(8., 8., 8.),
        },
        t,
    )
    .unwrap();
    let face = &mut source.shell.faces[0];
    let Surface::Plane { origin, u, .. } = &mut face.surface else {
        unreachable!()
    };
    *origin = *origin + *u * 1e15;
    for c in &mut face.wires[0].coedges {
        let PCurve::Affine { origin, .. } = &mut c.pcurve else {
            unreachable!()
        };
        origin[0] -= 1e15;
    }
    source.validate(t).unwrap();
    assert_eq!(source.bounds().min, Point3::new(-4., -4., -4.));
    assert!(source.tessellation_roundoff_budget(t).unwrap() > 1.);
    assert!(matches!(
        source.tessellate(0.05, t),
        Err(Error::Tessellation(_))
    ));
}
