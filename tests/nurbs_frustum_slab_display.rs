use hagane::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
fn key(p: Point3) -> [u64; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}
fn exact_closed(mesh: &Mesh) {
    let mut edges: BTreeMap<([u64; 3], [u64; 3]), (usize, i32)> = BTreeMap::new();
    for triangle in &mesh.triangles {
        for i in 0..3 {
            let a = key(mesh.positions[triangle[i]]);
            let b = key(mesh.positions[triangle[(i + 1) % 3]]);
            assert_ne!(a, b);
            let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let entry = edges.entry(edge).or_default();
            entry.0 += 1;
            entry.1 += sign;
        }
    }
    assert!(edges.values().all(|entry| *entry == (2, 0)));
    assert!(mesh.signed_volume() > 0.);
}
fn dense_bound(child: &NurbsBoredFrustumSlabSolid, mesh: &Mesh, error: f64) {
    for face in 2..10 {
        let tris: Vec<_> = mesh
            .triangles
            .iter()
            .zip(&mesh.face_ids)
            .filter_map(|(tri, id)| (*id == face).then_some(*tri))
            .collect();
        let n = ((tris.len() / 2) as f64).sqrt() as usize;
        assert_eq!(tris.len(), 2 * n * n);
        assert!(n.is_power_of_two());
        let Surface::Nurbs(surface) = &child.solid().shell.faces[face].surface else {
            panic!("missing actual ruled surface")
        };
        for i in 0..n {
            for j in 0..n {
                for a in 0..=4 {
                    for b in 0..=4 {
                        let x = a as f64 / 4.;
                        let y = b as f64 / 4.;
                        let (offset, weights) = if y <= x {
                            (0, [1. - x, x - y, y])
                        } else {
                            (1, [1. - y, x, y - x])
                        };
                        let mut tri = tris[2 * (i * n + j) + offset];
                        if child.solid().shell.faces[face].orientation < 0 {
                            tri.swap(1, 2);
                        }
                        let chord = mesh.positions[tri[0]] * weights[0]
                            + mesh.positions[tri[1]] * weights[1]
                            + mesh.positions[tri[2]] * weights[2];
                        let exact = surface
                            .evaluate((i as f64 + x) / n as f64, (j as f64 + y) / n as f64)
                            .unwrap();
                        assert!(
                            (exact - chord).norm() <= error,
                            "actual patch {face} cell{i},{j} error{} >{error}",
                            (exact - chord).norm()
                        );
                    }
                }
            }
        }
    }
}
fn document(bored: bool) -> Value {
    let mut operations = vec![
        json!({"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}),
    ];
    let input = if bored {
        operations.push(json!({"kind":"coaxial_bore","id":"bored","input":"stock","radius":4.}));
        "bored"
    } else {
        "stock"
    };
    operations.extend([json!({"kind":"plane_split","id":"first","input":input,"origin":[0.,0.,12.],"normal":[0.2,0.,1.]}),json!({"kind":"select","id":"selected","input":"first","component":0}),json!({"kind":"plane_split","id":"second","input":"selected","origin":[0.,0.,6.],"normal":[0.2,0.,1.]})]);
    json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"display_chord_tolerance":0.2,"output":"second","operations":operations})
}
fn rebuild(session: &mut NurbsFrustumWorkflowSession, doc: Value) -> Value {
    serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(
            session,
            &json!({"command":"rebuild","document":doc}).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn two_cap_bored_slabs_keep_annular_faces_exact_shared_edges_and_actual_patch_bounds() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let report = rebuild(&mut session, document(true));
    assert_eq!(report["ok"], true, "{report}");
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for (i, component) in session
        .accepted_shape()
        .unwrap()
        .components()
        .iter()
        .enumerate()
    {
        let NurbsFrustumWorkflowComponent::BoredSlab(body) = component else {
            panic!("expected actual bored slab")
        };
        let mesh = body.tessellate(0.2, policy).unwrap();
        exact_closed(&mesh);
        dense_bound(body, &mesh, 0.2);
        assert_eq!(
            report["components"][i]["step"],
            body.export_step_mm(policy).unwrap()
        );
        assert_eq!(
            report["components"][i]["mesh"]["triangles"],
            json!(mesh.triangles)
        );
        assert_eq!(report["components"][i]["mass_properties"]["ok"], false);
        for cap in 0..2 {
            assert_eq!(body.solid().shell.faces[cap].wires.len(), 2);
        }
        for error in [0., f64::NAN, 1e-12] {
            assert!(body.tessellate(error, policy).is_err());
        }
    }
}
#[test]
fn canonical_slabs_display_actual_two_cap_geometry_and_explicit_metric_scope() {
    let mut session = NurbsFrustumWorkflowSession::new();
    let doc = document(false);
    let report = rebuild(&mut session, doc.clone());
    assert_eq!(report["ok"], true, "{report}");
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    for (i, component) in session
        .accepted_shape()
        .unwrap()
        .components()
        .iter()
        .enumerate()
    {
        let NurbsFrustumWorkflowComponent::Slab(body) = component else {
            panic!("expected actual slab")
        };
        let mesh = body.tessellate(0.2, policy).unwrap();
        exact_closed(&mesh);
        assert_eq!(
            report["components"][i]["step"],
            body.export_step_mm(policy).unwrap()
        );
        assert_eq!(
            report["components"][i]["volume"],
            body.volume(policy).unwrap()
        );
        assert_eq!(report["components"][i]["mass_properties"]["ok"], false);
        for (tri, id) in mesh.triangles.iter().zip(&mesh.face_ids) {
            if *id < 2 {
                let Surface::Plane { origin, u, v } = &body.solid().shell.faces[*id].surface else {
                    panic!("actual plane")
                };
                let normal = u.cross(*v).normalized().unwrap();
                for vertex in tri {
                    assert!((mesh.positions[*vertex] - *origin).dot(normal).abs() < 1e-9);
                }
            }
        }
    }
    let replay = rebuild(&mut session, doc);
    assert_eq!(replay["components"], report["components"]);
    assert_eq!(replay["cache"]["evaluated_nodes"], 0);
}
