use hagane::*;
fn integral(a: f64, b: f64, p: usize) -> f64 {
    (b.powi(p as i32 + 1) - a.powi(p as i32 + 1)) / (p as f64 + 1.)
}
fn moments(r: [[f64; 2]; 2], dimensions: [f64; 3], b: f64) -> [f64; 4] {
    let [l, w, h] = dimensions;
    let i = r.map(|[a, b]| integral(a, b, 0));
    let a = r.map(|[lo, hi]| integral(lo, hi, 1) - integral(lo, hi, 2));
    let xa = r.map(|[lo, hi]| integral(lo, hi, 2) - integral(lo, hi, 3));
    let linear = r.map(|[lo, hi]| integral(lo, hi, 1));
    let square =
        r.map(|[lo, hi]| integral(lo, hi, 2) - 2. * integral(lo, hi, 3) + integral(lo, hi, 4));
    [
        l * w * (h * i[0] * i[1] + 4. * b * a[0] * a[1]),
        l * l * w * (h * linear[0] * i[1] + 4. * b * xa[0] * a[1]),
        l * w * w * (h * i[0] * linear[1] + 4. * b * a[0] * xa[1]),
        l * w / 2.
            * (h * h * i[0] * i[1]
                + 8. * h * b * a[0] * a[1]
                + 16. * b * b * square[0] * square[1]),
    ]
}
fn check(actual: NurbsGraphMassProperties, expected: [f64; 4]) {
    assert!((actual.volume - expected[0]).abs() < 1e-11 * expected[0].max(1.));
    let centroid = Point3::new(
        expected[1] / expected[0],
        expected[2] / expected[0],
        expected[3] / expected[0],
    );
    assert!((actual.centroid - centroid).norm() < 1e-10);
}
#[test]
fn polynomial_antiderivatives_verify_signed_trimmed_and_offcenter_hole_centroids() {
    let tol = Tolerance::default();
    let dimensions = [20., 12., 3.];
    let ranges = [[0.2, 0.8], [0.1, 0.7]];
    let hole = [[0.35, 0.5], [0.25, 0.4]];
    for b in [-2., 0., 20.] {
        let graph = NurbsGraphSolid::new(dimensions, b, tol)
            .unwrap()
            .trimmed_uv(ranges, tol)
            .unwrap();
        check(
            graph.mass_properties(tol).unwrap(),
            moments(ranges, dimensions, b),
        );
        let graph = NurbsGraphHoledSolid::new(&graph, hole, tol).unwrap();
        let outer = moments(ranges, dimensions, b);
        let inner = moments(hole, dimensions, b);
        check(
            graph.mass_properties(tol).unwrap(),
            std::array::from_fn(|i| outer[i] - inner[i]),
        );
    }
}
#[test]
fn mass_weighted_partition_and_rigid_placement_preserve_properties() {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([20., 12., 3.], 20., tol)
        .unwrap()
        .trimmed_uv([[0.2, 0.8], [0.1, 0.7]], tol)
        .unwrap();
    let whole = graph.mass_properties(tol).unwrap();
    for (axis, cut) in [(0, 0.45), (1, 0.4)] {
        let split = graph.split_uv(axis, cut, tol).unwrap();
        let a = split.negative.mass_properties(tol).unwrap();
        let b = split.positive.mass_properties(tol).unwrap();
        assert!((a.volume + b.volume - whole.volume).abs() < 1e-10);
        assert!(
            ((a.centroid * a.volume + b.centroid * b.volume) * (1. / (a.volume + b.volume))
                - whole.centroid)
                .norm()
                < 1e-10
        );
    }
    let transform = Transform::translation(Vec3::new(40., -30., 10.))
        .unwrap()
        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6).unwrap())
        .unwrap();
    let moved = graph
        .transformed(transform, tol)
        .unwrap()
        .mass_properties(tol)
        .unwrap();
    assert_eq!(moved.volume, whole.volume);
    assert!((moved.centroid - transform.point(whole.centroid)).norm() < 1e-10);
}
#[test]
fn thin_asymmetric_material_uses_positive_moments_and_auto_import_delegates() {
    let tol = Tolerance::new(1e-12).unwrap();
    let graph = NurbsGraphSolid::new([1.; 3], 0., tol).unwrap();
    let hole = [[1e-8, 1. - 2e-8], [1e-8, 1. - 1e-8]];
    let holed = NurbsGraphHoledSolid::new(&graph, hole, tol).unwrap();
    let [[u0, u1], [v0, v1]] = hole;
    let strips = [
        [[0., u0], [0., 1.]],
        [[u1, 1.], [0., 1.]],
        [[u0, u1], [0., v0]],
        [[u0, u1], [v1, 1.]],
    ];
    let expected = strips
        .map(|r| {
            let area = (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]);
            [
                area,
                area * (r[0][0] + r[0][1]) / 2.,
                area * (r[1][0] + r[1][1]) / 2.,
                area / 2.,
            ]
        })
        .into_iter()
        .fold([0.; 4], |a, b| std::array::from_fn(|i| a[i] + b[i]));
    let actual = holed.mass_properties(tol).unwrap();
    assert!((actual.volume - expected[0]).abs() < 1e-14 * expected[0]);
    assert!((actual.centroid.x - expected[1] / expected[0]).abs() < 1e-12);
    assert!((actual.centroid.y - expected[2] / expected[0]).abs() < 1e-12);
    assert_eq!(actual.centroid.z, 0.5);
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let holed = NurbsGraphHoledSolid::new(&graph, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
    for (step, expected) in [
        (
            graph.export_step_mm(tol).unwrap(),
            graph.mass_properties(tol).unwrap(),
        ),
        (
            holed.export_step_mm(tol).unwrap(),
            holed.mass_properties(tol).unwrap(),
        ),
    ] {
        let imported = import_step_nurbs_graph_auto_mm(&step, tol)
            .unwrap()
            .mass_properties(tol)
            .unwrap();
        assert!((imported.volume - expected.volume).abs() < 1e-10);
        assert!((imported.centroid - expected.centroid).norm() < 1e-10);
    }
}
#[test]
fn mass_properties_reject_public_geometry_corruption() {
    let tol = Tolerance::default();
    let mut graph = NurbsGraphSolid::new([20., 12., 3.], 20., tol).unwrap();
    graph.solid.vertices[0].point.x += 0.01;
    assert!(graph.mass_properties(tol).is_err());
    let graph = NurbsGraphSolid::new([20., 12., 3.], 20., tol).unwrap();
    let mut holed = NurbsGraphHoledSolid::new(&graph, [[0.35, 0.5], [0.25, 0.4]], tol).unwrap();
    holed.solid.shell.faces[0].wires[1].coedges[0].edge = 999;
    assert!(holed.mass_properties(tol).is_err());
}
