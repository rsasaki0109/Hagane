use hagane::*;
use std::collections::BTreeMap;
use std::sync::Arc;

fn value(angle: f64) -> serde_json::Value {
    serde_json::json!({"schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm","tolerance":{"linear":1e-6,"angular":1e-10,"relative":0.},"output":"parts","display_chord_tolerance":0.2,"operations":[{"kind":"posed_frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-3.,5.],"rotation_axis":[1.,2.,3.],"angle":angle},{"kind":"partition","id":"parts","input":"stock","cuts":[6.,12.,18.]}]})
}
fn doc(v: serde_json::Value) -> NurbsFrustumWorkflowDocument {
    serde_json::from_value(v).unwrap()
}
fn policy() -> GeometryTolerance {
    GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap()
}
fn integral(power: usize, moment: usize) -> f64 {
    let mut binomial = 1.;
    let mut sum = 0.;
    for k in 0..=power {
        if k > 0 {
            binomial *= (power + 1 - k) as f64 / k as f64;
        }
        sum += binomial * 16_f64.powi((power - k) as i32) * (-8_f64).powi(k as i32)
            / (moment + k + 1) as f64;
    }
    sum
}
fn rotation(angle: f64) -> [[f64; 3]; 3] {
    let n = 14_f64.sqrt();
    let a = [1. / n, 2. / n, 3. / n];
    let c = angle.cos();
    let s = angle.sin();
    let skew = [[0., -a[2], a[1]], [a[2], 0., -a[0]], [-a[1], a[0], 0.]];
    std::array::from_fn(|i| {
        std::array::from_fn(
            |j| if i == j { c } else { 0. } + (1. - c) * a[i] * a[j] + s * skew[i][j],
        )
    })
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 2e-10 * b.abs().max(1.), "{a} vs {b}");
}

#[test]
fn actual_pose_and_independent_world_moments_partition_conservation() {
    let angle = 0.7;
    let document = doc(value(angle));
    let shape = document.rebuild().unwrap();
    let frame = Transform::translation(Vec3::new(12., -3., 5.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), angle).unwrap())
        .unwrap();
    let parent = NurbsFrustumSolid::new(frame, [16., 8.], 24., policy()).unwrap();
    let direct = parent.split_axial_many(&[6., 12., 18.], policy()).unwrap();
    for (actual, expected) in shape.components().iter().zip(&direct.parts) {
        assert_eq!(
            format!("{:?}", actual.solid()),
            format!("{:?}", expected.solid())
        );
        assert_eq!(
            actual.export_step_mm(policy()).unwrap(),
            expected.export_step_mm(policy()).unwrap()
        );
    }
    let q0 = integral(2, 0);
    let q1 = integral(2, 1);
    let q2 = integral(2, 2);
    let q4 = integral(4, 0);
    let volume = std::f64::consts::PI * 24. * q0;
    let cz = 24. * q1 / q0;
    let transverse = std::f64::consts::PI * 24. * q4 / 4.
        + std::f64::consts::PI * 24_f64.powi(3) * (q2 - q1 * q1 / q0);
    let axial = std::f64::consts::PI * 24. * q4 / 2.;
    let r = rotation(angle);
    let centroid = Point3::new(12. + r[0][2] * cz, -3. + r[1][2] * cz, 5. + r[2][2] * cz);
    let expected: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|k| r[i][k] * r[j][k] * [transverse, transverse, axial][k])
                .sum()
        })
    });
    let actual = parent.inertia_properties(policy()).unwrap();
    close(actual.volume, volume);
    assert!((actual.centroid - centroid).norm() < 1e-10);
    let mut sum_volume = 0.;
    let mut moment = Vec3::new(0., 0., 0.);
    let mut tensor = [[0.; 3]; 3];
    for part in shape.components() {
        let p = part.inertia_properties(policy()).unwrap();
        sum_volume += p.volume;
        moment = moment + p.centroid * p.volume;
        let d = p.centroid - centroid;
        let xyz = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                tensor[i][j] += p.inertia[i][j]
                    + p.volume * ((if i == j { d.dot(d) } else { 0. }) - xyz[i] * xyz[j]);
            }
        }
    }
    close(sum_volume, volume);
    assert!((moment * (1. / sum_volume) - centroid).norm() < 1e-10);
    for i in 0..3 {
        for j in 0..3 {
            close(actual.inertia[i][j], expected[i][j]);
            close(tensor[i][j], expected[i][j]);
        }
    }
}

#[test]
fn actual_pcurves_closed_display_and_native_report() {
    let document = doc(value(0.7));
    let shape = document.rebuild().unwrap();
    for body in shape.components() {
        let solid = body.solid();
        for face in &solid.shell.faces {
            for coedge in &face.wires[0].coedges {
                let curve = &solid.edges[coedge.edge].curve;
                let range = curve.range();
                for i in 0..=32 {
                    let t = range[0] + (range[1] - range[0]) * i as f64 / 32.;
                    let uv = coedge.pcurve.try_evaluate(t).unwrap();
                    assert!(
                        (curve.try_evaluate(t).unwrap()
                            - face.surface.try_evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                            < 1e-9
                    );
                }
            }
        }
        let mesh = body.tessellate(0.2, policy()).unwrap();
        let key = |p: Point3| [p.x, p.y, p.z].map(|x| if x == 0. { 0 } else { x.to_bits() });
        let mut edges = BTreeMap::new();
        for tri in &mesh.triangles {
            for i in 0..3 {
                let a = key(mesh.positions[tri[i]]);
                let b = key(mesh.positions[tri[(i + 1) % 3]]);
                assert_ne!(a, b);
                let (k, s) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                let count = edges.entry(k).or_insert((0, 0));
                count.0 += 1;
                count.1 += s;
            }
        }
        assert!(edges.values().all(|e| *e == (2, 0)));
    }
    let mut imported = value(0.7);
    imported["operations"] = serde_json::json!([{"kind":"step_stock","id":"stock","step":shape.components()[0].export_step_mm(policy()).unwrap()}]);
    imported["output"] = serde_json::json!("stock");
    assert!(
        doc(imported).rebuild().is_err(),
        "StepStock retains cardinal-only import scope"
    );
    let mut session = NurbsFrustumWorkflowSession::new();
    let request = serde_json::json!({"command":"rebuild","document":document}).to_string();
    let report: serde_json::Value = serde_json::from_str(
        &nurbs_frustum_workflow_session_command_json(&mut session, &request).unwrap(),
    )
    .unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["components"].as_array().unwrap().len(), 4);
    for (body, step) in shape
        .components()
        .iter()
        .zip(report["component_steps"].as_array().unwrap())
    {
        assert_eq!(
            step.as_str().unwrap(),
            body.export_step_mm(policy()).unwrap()
        );
    }
}

#[test]
fn angle_axis_and_static_reused_graph_errors_are_atomic() {
    let original = doc(value(0.7));
    let mut session = NurbsFrustumWorkflowSession::new();
    let accepted = session.rebuild(&original).unwrap();
    let prefix = session.prefix_snapshot(0).unwrap().clone();
    let mut invalids = Vec::new();
    for angle in [std::f64::consts::PI + 1e-12, -std::f64::consts::PI - 1e-12] {
        invalids.push(doc(value(angle)));
    }
    let mut v = value(0.7);
    v["operations"][0]["rotation_axis"] = serde_json::json!([0., 0., 0.]);
    invalids.push(doc(v));
    let mut v = value(0.7);
    v["operations"][0]["origin"] = serde_json::json!([1e12, 0., 0.]);
    invalids.push(doc(v));
    let mut v = value(0.7);
    v["operations"].as_array_mut().unwrap().push(serde_json::json!({"kind":"posed_frustum","id":"bad","radii":[16.,8.],"height":24.,"origin":[0.,0.,0.],"rotation_axis":[0.,0.,0.],"angle":0.}));
    invalids.push(doc(v));
    for mut invalid in [original.clone(), original.clone()] {
        let NurbsFrustumWorkflowOperation::PosedFrustum {
            angle,
            rotation_axis,
            ..
        } = &mut invalid.operations[0]
        else {
            panic!("posed node");
        };
        if invalids.len() == 5 {
            *angle = f64::NAN;
        } else {
            rotation_axis[0] = f64::INFINITY;
        }
        invalids.push(invalid);
    }
    for invalid in invalids {
        assert!(session.rebuild(&invalid).is_err());
        assert_eq!(session.accepted_document(), Some(&original));
        assert!(Arc::ptr_eq(
            session.accepted_shape().unwrap(),
            &accepted.shape
        ));
        assert!(Arc::ptr_eq(session.prefix_snapshot(0).unwrap(), &prefix));
        assert_eq!(session.undo_count(), 0);
    }
    for angle in [-std::f64::consts::PI, std::f64::consts::PI, 0.] {
        let shape = doc(value(angle)).rebuild().unwrap();
        assert_eq!(shape.components().len(), 4);
    }
    let mut old = value(0.7);
    old["operations"][0] = serde_json::json!({"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-3.,5.],"axes":[[0.8,0.6,0.],[-0.6,0.8,0.],[0.,0.,1.]]});
    assert!(doc(old).rebuild().is_err());
}
