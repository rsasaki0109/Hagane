use hagane::*;
#[test]
fn shared_surface_helpers_preserve_original_control_arithmetic_bitwise() {
    let tol = Tolerance::default();
    let full = [[0., 1.], [0., 1.]];
    let hole = [[0.3, 0.7], [0.35, 0.65]];
    for (h, b) in [
        (20., -12.),
        (20., 0.001),
        (20., 30.),
        (20., 36.),
        (3., -2.),
        (20., 0.),
    ] {
        let source = NurbsGraphSolid::new([80., 60., h], b, tol).unwrap();
        let controls = (0..3)
            .flat_map(|i| {
                (0..3).map(move |j| {
                    Point3::new(
                        80. * i as f64 / 2.,
                        60. * j as f64 / 2.,
                        h + if i == 1 && j == 1 { b } else { 0. },
                    )
                })
            })
            .collect();
        let knots = vec![0., 0., 0., 1., 1., 1.];
        let original = NurbsSurface::new(
            [2, 2],
            [knots.clone(), knots],
            [3, 3],
            controls,
            vec![1.; 9],
        )
        .unwrap();
        let Surface::Nurbs(actual) = &source.brep().shell.faces[1].surface else {
            panic!()
        };
        assert_eq!(actual.control_points(), original.control_points());
        assert_eq!(actual.weights(), original.weights());
        let mut expected = original.restricted(full).unwrap();
        for (axis, parameters) in [(0, hole[0]), (1, hole[1])] {
            for parameter in parameters {
                expected = expected.insert_knot(axis, parameter, 2).unwrap();
            }
        }
        let body = NurbsGraphHoledSolid::new(&source, hole, tol).unwrap();
        let Surface::Nurbs(actual) = &body.brep().shell.faces[1].surface else {
            panic!()
        };
        assert_eq!(actual.control_points(), expected.control_points());
        assert_eq!(actual.weights(), expected.weights());
        assert_eq!(actual.knots(0).unwrap(), expected.knots(0).unwrap());
        assert_eq!(actual.knots(1).unwrap(), expected.knots(1).unwrap());
        let step = body.export_step_mm(tol).unwrap();
        assert_eq!(
            import_step_nurbs_graph_holed_mm(&step, tol)
                .unwrap()
                .export_step_mm(tol)
                .unwrap(),
            step
        );
    }
}
#[test]
fn matching_roof_never_bypasses_actual_wall_coefficient_validation() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 36., tol).unwrap();
    let body = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol).unwrap();
    let step = body.export_step_mm(tol).unwrap();
    let wall = step
        .lines()
        .filter(|line| line.contains("=B_SPLINE_SURFACE_WITH_KNOTS("))
        .nth(2)
        .unwrap();
    let refs = wall
        .split_once('=')
        .unwrap()
        .1
        .split('#')
        .skip(1)
        .map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<usize>()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let id = refs[10];
    let line = step
        .lines()
        .find(|line| line.starts_with(&format!("#{id}=")))
        .unwrap();
    let xyz = line
        .split_once("('',(")
        .unwrap()
        .1
        .split_once("))")
        .unwrap()
        .0
        .split(',')
        .map(|v| v.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    let mutated = format!(
        "#{id}=CARTESIAN_POINT('',({},{},{}));",
        xyz[0],
        xyz[1],
        xyz[2].next_up()
    );
    let bad = step.replacen(line, &mutated, 1);
    assert!(import_step_nurbs_graph_holed_mm(&bad, tol).is_err());
    assert_eq!(
        import_step_nurbs_graph_holed_mm(&step, tol)
            .unwrap()
            .export_step_mm(tol)
            .unwrap(),
        step
    );
}
