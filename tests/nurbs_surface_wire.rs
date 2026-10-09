use hagane::*;
fn surface() -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 10., 0.),
            Point3::new(10., 0., 0.),
            Point3::new(10., 10., 4.),
        ],
        vec![1.; 4],
    )
    .unwrap()
}
#[test]
fn diagonal_triangle_shares_closure_and_exact_pcurves() {
    let uv = vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]];
    let w = NurbsSurfaceWire::new(surface(), uv.clone(), Tolerance::default()).unwrap();
    assert_eq!(w.orientation().unwrap(), Orientation::CounterClockwise);
    for i in 0..3 {
        assert_eq!(w.edges[i].vertices, [i, (i + 1) % 3]);
        let Curve::Nurbs(c) = &w.edges[i].curve else {
            panic!()
        };
        for j in 0..=32 {
            let t = j as f64 / 32.;
            let p = w.wire.coedges[i].pcurve.evaluate(t);
            assert!(
                (c.evaluate(t).unwrap() - surface().evaluate(p[0], p[1]).unwrap()).norm() < 1e-10
            );
        }
    }
    let lines = w
        .tessellate_boundary(0.01, 1024, Tolerance::default())
        .unwrap();
    assert_eq!(lines.len(), 3);
    let mut reverse = uv;
    reverse.reverse();
    assert_eq!(
        NurbsSurfaceWire::new(surface(), reverse, Tolerance::default())
            .unwrap()
            .orientation()
            .unwrap(),
        Orientation::Clockwise
    );
}
#[test]
fn unsupported_polygons_and_dirty_topology_are_rejected() {
    for uv in [
        vec![[0., 0.], [1., 1.], [0., 1.], [1., 0.]],
        vec![[0., 0.], [1., 0.], [0.4, 0.4], [0., 1.]],
        vec![[0., 0.], [0.5, 0.], [1., 0.]],
        vec![[0., 0.], [1., 0.], [0., 1.], [0., 0.]],
        vec![[0., 0.], [2., 0.], [0., 1.]],
    ] {
        assert!(NurbsSurfaceWire::new(surface(), uv, Tolerance::default()).is_err());
    }
    let w = NurbsSurfaceWire::new(
        surface(),
        vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]],
        Tolerance::default(),
    )
    .unwrap();
    let mut dirty = w.clone();
    dirty.edges[2].vertices[1] = 1;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = w.clone();
    dirty.wire.coedges[1].forward = false;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = w;
    dirty.vertices[0].point.z += 1.;
    assert!(dirty.validate(Tolerance::default()).is_err());
}

#[test]
fn stars_nonfinite_and_resource_limits_fail_explicitly() {
    let points: Vec<_> = (0..5)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 5.;
            [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
        })
        .collect();
    let star = [0, 2, 4, 1, 3].into_iter().map(|i| points[i]).collect();
    assert!(NurbsSurfaceWire::new(surface(), star, Tolerance::default()).is_err());
    assert!(NurbsSurfaceWire::new(
        surface(),
        vec![[0., 0.], [f64::NAN, 0.], [0., 1.]],
        Tolerance::default()
    )
    .is_err());
    assert!(NurbsSurfaceWire::new(surface(), vec![[0., 0.]; 65], Tolerance::default()).is_err());
    let wire = NurbsSurfaceWire::new(
        surface(),
        vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]],
        Tolerance::default(),
    )
    .unwrap();
    assert!(wire
        .tessellate_boundary(0.000001, 1, Tolerance::default())
        .is_err());
}
