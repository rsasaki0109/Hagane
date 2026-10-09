use hagane::*;

#[test]
fn shell_face_order_and_cyclic_wire_starts_preserve_actual_geometry() {
    let tolerance = Tolerance::default();
    let source = NurbsGraphSolid::new([20., 12., 3.], -2., tolerance).unwrap();
    let original = source.export_step_mm(tolerance).unwrap();
    let reordered = original
        .lines()
        .map(|line| {
            for entity in ["CLOSED_SHELL", "EDGE_LOOP"] {
                let start = format!("{entity}('',(");
                if let Some((prefix, rest)) = line.split_once(&start) {
                    let values = rest.strip_suffix("));").unwrap();
                    let mut references = values.split(',').collect::<Vec<_>>();
                    if entity == "CLOSED_SHELL" {
                        references.reverse();
                    } else {
                        references.rotate_left(1);
                    }
                    return format!("{prefix}{start}{}));", references.join(","));
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let imported = import_step_nurbs_graph_mm(&reordered, tolerance).unwrap();
    imported.validate(tolerance).unwrap();
    assert_eq!(imported.export_step_mm(tolerance).unwrap(), original);
    assert_eq!(imported.volume().unwrap(), source.volume().unwrap());
    assert_eq!(
        imported
            .classify_point(Point3::new(10., 6., 1.), GeometryTolerance::default())
            .unwrap(),
        PointLocation::Inside
    );
}
