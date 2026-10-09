use hagane::*;
fn shifted(mut tensor: [[f64; 3]; 3], mass: f64, offset: Point3) -> [[f64; 3]; 3] {
    let a = [offset.x, offset.y, offset.z];
    let norm = offset.dot(offset);
    for i in 0..3 {
        for j in 0..3 {
            tensor[i][j] += mass * (if i == j { norm } else { 0. } - a[i] * a[j]);
        }
    }
    tensor
}
fn box_tensor(dim: [f64; 3]) -> [[f64; 3]; 3] {
    let v = dim.iter().product::<f64>();
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                v * (dim[(i + 1) % 3].powi(2) + dim[(i + 2) % 3].powi(2)) / 12.
            } else {
                0.
            }
        })
    })
}
fn compare(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) {
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (a[i][j] - b[i][j]).abs() <= 1e-10 * b[i][j].abs().max(1.),
                "tensor[{i}][{j}] {} != {}",
                a[i][j],
                b[i][j]
            );
        }
    }
}
#[test]
fn boxes_and_asymmetric_rectangular_openings_match_parallel_axis_analytic_tensors() {
    let tol = Tolerance::default();
    let dims = [20., 12., 3.];
    let graph = NurbsGraphSolid::new(dims, 0., tol).unwrap();
    let actual = graph.inertia_properties(tol).unwrap();
    assert_eq!(actual.volume, 720.);
    assert_eq!(actual.centroid, Point3::new(10., 6., 1.5));
    compare(actual.inertia, box_tensor(dims));
    let hole = [[0.2, 0.6], [0.25, 0.75]];
    let holed = NurbsGraphHoledSolid::new(&graph, hole, tol)
        .unwrap()
        .inertia_properties(tol)
        .unwrap();
    let removed_dims = [8., 6., 3.];
    let removed_volume = 144.;
    let whole_center = Point3::new(10., 6., 1.5);
    let removed_center = Point3::new(8., 6., 1.5);
    let volume = 720. - removed_volume;
    let center = (whole_center * 720. - removed_center * removed_volume) * (1. / volume);
    assert!((holed.centroid - center).norm() < 1e-12);
    let whole_origin = shifted(box_tensor(dims), 720., whole_center);
    let removed_origin = shifted(box_tensor(removed_dims), removed_volume, removed_center);
    let origin =
        std::array::from_fn(|i| std::array::from_fn(|j| whole_origin[i][j] - removed_origin[i][j]));
    let expected = shifted(origin, -volume, center);
    compare(holed.inertia, expected);
}
#[test]
fn curved_partition_parallel_axis_additivity_rotation_translation_and_positive_forms() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([20., 12., 3.], 20., tol)
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], tol)
        .unwrap();
    let whole = graph.inertia_properties(tol).unwrap();
    for (axis, cut) in [(0, 0.45), (1, 0.4)] {
        let split = graph.split_uv(axis, cut, tol).unwrap();
        let a = split.negative.inertia_properties(tol).unwrap();
        let b = split.positive.inertia_properties(tol).unwrap();
        let ia = shifted(a.inertia, a.volume, a.centroid - whole.centroid);
        let ib = shifted(b.inertia, b.volume, b.centroid - whole.centroid);
        compare(
            std::array::from_fn(|i| std::array::from_fn(|j| ia[i][j] + ib[i][j])),
            whole.inertia,
        );
    }
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap();
    let transform = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(rotation)
        .unwrap();
    let placed = graph
        .transformed(transform, tol)
        .unwrap()
        .inertia_properties(tol)
        .unwrap();
    let axes = rotation.axes();
    let matrix: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| [axes[j].x, axes[j].y, axes[j].z][i]));
    let rotated = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .flat_map(|k| {
                    (0..3).map(move |l| matrix[i][k] * whole.inertia[k][l] * matrix[j][l])
                })
                .sum()
        })
    });
    compare(placed.inertia, rotated);
    assert!((placed.centroid - transform.point(whole.centroid)).norm() < 1e-10);
    for q in [
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [1., 2., 3.],
        [-2., 3., 1.],
    ] {
        let value = (0..3)
            .flat_map(|i| (0..3).map(move |j| q[i] * placed.inertia[i][j] * q[j]))
            .sum::<f64>();
        assert!(value > 0.);
    }
    let loose = Tolerance::new(1e-3).unwrap();
    let translated = graph
        .transformed(
            Transform::translation(Vec3::new(1e6, -1e6, 1e6)).unwrap(),
            loose,
        )
        .unwrap()
        .inertia_properties(loose)
        .unwrap();
    compare(translated.inertia, whole.inertia);
    assert!((translated.centroid - (whole.centroid + Vec3::new(1e6, -1e6, 1e6))).norm() < 1e-8);
}
#[test]
fn signed_holed_imports_delegate_and_mutated_geometry_rejects_inertia() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([20., 12., 3.], -2., tol).unwrap();
    let holed = NurbsGraphHoledSolid::new(&graph, [[0.35, 0.5], [0.25, 0.4]], tol).unwrap();
    for (step, expected) in [
        (
            graph.export_step_mm(tol).unwrap(),
            graph.inertia_properties(tol).unwrap(),
        ),
        (
            holed.export_step_mm(tol).unwrap(),
            holed.inertia_properties(tol).unwrap(),
        ),
    ] {
        let imported = import_step_nurbs_graph_auto_mm(&step, tol)
            .unwrap()
            .inertia_properties(tol)
            .unwrap();
        compare(imported.inertia, expected.inertia);
        assert!((imported.centroid - expected.centroid).norm() < 1e-10);
    }
    let mut dirty = graph;
    dirty.solid.vertices[0].point.x += 0.01;
    assert!(dirty.inertia_properties(tol).is_err());
    let mut dirty = holed;
    dirty.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(dirty.inertia_properties(tol).is_err());
}
