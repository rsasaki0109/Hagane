use hagane::*;
use std::collections::BTreeMap;

fn checked_display(solid: &Solid) {
    let mesh = solid
        .tessellate(0.1, Tolerance::new(1e-6).unwrap())
        .unwrap();
    let mut incidence: BTreeMap<([i64; 3], [i64; 3]), (usize, i32)> = BTreeMap::new();
    let key = |p: Point3| [p.x, p.y, p.z].map(|x| (x * 1e8).round() as i64);
    for triangle in &mesh.triangles {
        let p = triangle.map(|i| mesh.positions[i]);
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        assert!(cross.finite() && cross.dot(mesh.normals[triangle[0]]) > 0.);
        for i in 0..3 {
            let (a, b) = (key(p[i]), key(p[(i + 1) % 3]));
            assert_ne!(a, b);
            let entry = incidence.entry((a.min(b), a.max(b))).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    // Removing a collapsed ear must not open a cap or omit a shared wall seam.
    assert!(incidence
        .values()
        .all(|&(count, signed)| count == 2 && signed == 0));
    assert!((mesh.signed_volume() - solid.volume().unwrap()).abs() < 250.);
}

#[test]
fn actual_wasm_step_with_uv_ear_collapsing_in_world_keeps_closed_positive_triangles() {
    // Actual WASM-generated rounded stock with sixteen alternating-entry
    // pockets, STEP entity IDs permuted and records reversed. Before repair,
    // native cap triangle 40 was (-40,-22), (24,-22), (-8,-22), although its
    // original UV ear had an approximately 2e-15 nonzero altitude.
    let source = import_step_bounded_analytic_mm(
        include_str!("fixtures/blind16_wasm_shuffled.step"),
        Tolerance::new(1e-6).unwrap(),
    )
    .unwrap();
    assert_eq!(source.shell.faces.len(), 90);
    checked_display(&source);
}

#[test]
fn native_rounded_sixteen_pocket_roundtrip_preserves_all_cap_wall_seams() {
    let tolerance = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let stock = make_box(
        BoxSpec {
            min: Point3::new(-40., -30., 0.),
            size: Vec3::new(80., 60., 20.),
        },
        tolerance.absolute(),
    )
    .unwrap();
    let rounded =
        fillet_parallel_box_edges(&stock, &[(8, 8.), (9, 8.), (10, 8.), (11, 8.)], tolerance)
            .unwrap();
    let mut specs = Vec::new();
    for (i, x) in [-24., -8., 8., 24.].into_iter().enumerate() {
        for (j, y) in [-24., -8., 8., 24.].into_iter().enumerate() {
            let positive = (i + j) % 2 == 0;
            specs.push(NormalPrismBlindBoreSpec {
                center: Point3::new(x, y, if positive { 20. } else { 0. }),
                radius: 2.,
                depth: 8.,
                entry: if positive {
                    NormalPrismBoreEntry::Positive
                } else {
                    NormalPrismBoreEntry::Negative
                },
            });
        }
    }
    let result =
        blind_bores_normal_prism(rounded.solid(), &specs, Vec3::new(0., 0., 1.), tolerance)
            .unwrap();
    let text = export_step_bounded_analytic_mm(result.kept(), tolerance.linear()).unwrap();
    let imported = import_step_bounded_analytic_mm(&text, tolerance.absolute()).unwrap();
    checked_display(result.kept());
    checked_display(&imported);
}
