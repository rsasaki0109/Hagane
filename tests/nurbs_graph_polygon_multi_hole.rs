use hagane::*;
fn openings() -> Vec<Vec<[f64; 2]>> {
    vec![
        vec![[0.12, 0.18], [0.3, 0.18], [0.3, 0.4], [0.12, 0.4]],
        vec![[0.6, 0.15], [0.82, 0.2], [0.78, 0.4]],
        vec![[0.38, 0.65], [0.65, 0.6], [0.62, 0.83], [0.4, 0.85]],
    ]
}
#[test]
fn closed_three_openings_actual_geometry_and_genus() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let body = source.through_uv_polygons(openings(), tol).unwrap();
    body.validate(tol).unwrap();
    let n = 4 + openings().iter().map(Vec::len).sum::<usize>();
    assert_eq!(body.genus(), 3);
    assert_eq!(
        (
            body.solid.vertices.len(),
            body.solid.edges.len(),
            body.solid.shell.faces.len()
        ),
        (2 * n, 3 * n, n + 2)
    );
    assert_eq!(body.solid.shell.faces[0].wires.len(), 4);
    assert_eq!(body.solid.shell.faces[1].wires.len(), 4);
    let mut uses = vec![[0i32; 2]; 3 * n];
    for f in &body.solid.shell.faces {
        for w in &f.wires {
            for c in &w.coedges {
                uses[c.edge][0] += 1;
                uses[c.edge][1] += f.orientation as i32 * if c.forward { 1 } else { -1 };
                let e = &body.solid.edges[c.edge];
                for k in 0..21 {
                    let t = k as f64 / 20.;
                    let uv = c.pcurve.evaluate(t);
                    assert!(
                        (e.curve.try_evaluate(t).unwrap()
                            - f.surface.try_evaluate(uv[0], uv[1]).unwrap())
                        .norm()
                            < 1e-10
                    );
                }
            }
        }
    }
    assert!(uses.iter().all(|u| *u == [2, 0]));
    assert!(body.brep().volume().is_err());
}
#[test]
fn pair_contacts_nesting_and_public_corruption() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let a = vec![[0.2, 0.2], [0.6, 0.2], [0.6, 0.6], [0.2, 0.6]];
    for b in [
        a.clone(),
        vec![[0.3, 0.3], [0.4, 0.3], [0.4, 0.4], [0.3, 0.4]],
        vec![[0.6, 0.2], [0.8, 0.2], [0.8, 0.5], [0.6, 0.5]],
        vec![[0.59, 0.3], [0.8, 0.3], [0.8, 0.5], [0.59, 0.5]],
    ] {
        assert!(source.through_uv_polygons(vec![a.clone(), b], tol).is_err());
    }
    let mut body = source.through_uv_polygons(openings(), tol).unwrap();
    body.solid.shell.faces[0].wires[2].coedges[0].forward =
        !body.solid.shell.faces[0].wires[2].coedges[0].forward;
    assert!(body.validate(tol).is_err());
    assert!(body.bounds().is_err());
    assert!(source.through_uv_polygons(vec![], tol).is_err());
    assert!(source.through_uv_polygons(vec![a; 5], tol).is_err());
}
#[test]
fn sixty_four_corner_resource_limit() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let circle = |center: [f64; 2], radius: f64, n: usize, phase: f64| {
        (0..n)
            .map(|i| {
                let t = std::f64::consts::TAU * i as f64 / n as f64 + phase;
                [center[0] + radius * t.cos(), center[1] + radius * t.sin()]
            })
            .collect::<Vec<_>>()
    };
    let outer =
        NurbsGraphPolygonSolid::new(&source, circle([0.5, 0.5], 0.49, 16, 0.01), tol).unwrap();
    let holes = vec![
        circle([0.3, 0.3], 0.06, 16, 0.03),
        circle([0.7, 0.3], 0.06, 16, 0.12),
        circle([0.5, 0.7], 0.06, 16, 0.07),
    ];
    let body = outer.through_uv_polygons(holes.clone(), tol).unwrap();
    assert_eq!(body.brep().vertices.len(), 128);
    body.validate(tol).unwrap();
    let mut too_many = holes;
    too_many.push(circle([0.5, 0.5], 0.03, 3, 0.1));
    assert!(outer.through_uv_polygons(too_many, tol).is_err());
}

#[test]
fn four_aligned_rectangular_openings() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol).unwrap();
    let rect =
        |u: [f64; 2], v: [f64; 2]| vec![[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    let body = source
        .through_uv_polygons(
            vec![
                rect([0.2, 0.35], [0.2, 0.35]),
                rect([0.55, 0.7], [0.2, 0.35]),
                rect([0.2, 0.35], [0.55, 0.7]),
                rect([0.55, 0.7], [0.55, 0.7]),
            ],
            tol,
        )
        .unwrap();
    body.validate(tol).unwrap();
    assert_eq!(body.genus(), 4);
}
#[test]
fn strict_pair_clearance_above_and_below_budget() {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([8., 6., 3.], 2., tol).unwrap();
    let rect = |u: [f64; 2]| vec![[u[0], 0.2], [u[1], 0.2], [u[1], 0.5], [u[0], 0.5]];
    let a = rect([0.2, 0.4]);
    assert!(source
        .through_uv_polygons(
            vec![a.clone(), rect([0.4 + 2. * tol.linear / 8., 0.7])],
            tol
        )
        .is_err());
    let body = source
        .through_uv_polygons(
            vec![a.clone(), rect([0.4 + 16. * tol.linear / 8., 0.7])],
            tol,
        )
        .unwrap();
    body.validate(tol).unwrap();
    let mut cw = a.clone();
    cw.reverse();
    assert!(source.through_uv_polygons(vec![cw], tol).is_err());
    assert!(source
        .through_uv_polygons(
            vec![vec![[0.2, 0.2], [0.6, 0.2], [0.4, 0.3], [0.4, 0.6]]],
            tol
        )
        .is_err());
}
