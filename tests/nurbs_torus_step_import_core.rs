use hagane::*;
#[test]
fn exact_cardinal_preimage_retains_actual_body_and_old_reader_scope() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let frame = Frame3::new_with_tolerance(
        Point3::new(12., -5., 8.),
        [
            Vec3::new(0., 1., 0.),
            Vec3::new(0., 0., 1.),
            Vec3::new(1., 0., 0.),
        ],
        p,
    )
    .unwrap();
    let source = NurbsTorusSolid::new(frame, [16., 4.], p).unwrap();
    let text = source.export_step_mm(p).unwrap();
    let actual = import_step_nurbs_torus_cardinal_mm(&text, p).unwrap();
    assert_eq!(
        format!("{:?}", actual.solid()),
        format!("{:?}", source.solid())
    );
    assert_eq!(actual.export_step_mm(p).unwrap(), text);
    assert!(import_step_nurbs_frustum_mm(&text, p).is_err());
    assert!(import_step_nurbs_frustum_translated_mm(&text, p).is_err());
    assert!(import_step_nurbs_frustum_cardinal_mm(&text, p).is_err());
    assert!(import_step_nurbs_bored_frustum_cardinal_mm(&text, p).is_err());
    let wrong = text
        .lines()
        .map(|line| {
            let prefix = "CLOSED_SHELL('',(";
            if let Some(start) = line.find(prefix) {
                let begin = start + prefix.len();
                let end = begin + line[begin..].find(')').unwrap();
                let mut ids: Vec<_> = line[begin..end].split(',').collect();
                ids.rotate_left(1);
                format!("{}{}{}", &line[..begin], ids.join(","), &line[end..])
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(import_step_nurbs_torus_cardinal_mm(&wrong, p).is_err());
    let pose = Transform::rotation(Vec3::new(1., 2., 3.), 0.37).unwrap();
    let source = NurbsTorusSolid::new(pose, [16., 4.], p).unwrap();
    assert!(import_step_nurbs_torus_cardinal_mm(&source.export_step_mm(p).unwrap(), p).is_err());
}
