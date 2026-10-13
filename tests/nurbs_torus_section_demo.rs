use hagane::*;
use serde_json::Value;
#[test]
fn exact_section_reports_actual_curves_all_seam_uses_and_preserves_accepted_state() {
    let p = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    for cardinal in [false, true] {
        let mut s = NurbsTorusDemoSession::new();
        if cardinal {
            let source = NurbsTorusSolid::new(
                Frame3::new_with_tolerance(
                    Point3::new(12., -5., 8.),
                    [
                        Vec3::new(0., 1., 0.),
                        Vec3::new(0., 0., 1.),
                        Vec3::new(1., 0., 0.),
                    ],
                    p,
                )
                .unwrap(),
                [16., 4.],
                p,
            )
            .unwrap();
            s.import_step_cardinal(&source.export_step_mm(p).unwrap(), 0.1, p)
                .unwrap();
        } else {
            s.apply(&[16., 4., 0.37, 12., -5., 8., 1e-6, 0.1]).unwrap();
        }
        s.split_horizontal(1., 0.1).unwrap();
        let ptr = s.accepted_horizontal_split().unwrap() as *const _;
        let before = format!("{:?}", s.accepted_horizontal_split().unwrap());
        let step = s.accepted_body().unwrap().export_step_mm(p).unwrap();
        for height in [-1., 0., 1.] {
            let report: Value =
                serde_json::from_str(&s.section_horizontal(height, 0.1).unwrap()).unwrap();
            assert_eq!(report["stock_unchanged"], true);
            assert_eq!(report["cut_created"], false);
            assert_eq!(report["loops"][0]["role"], "outer");
            assert_eq!(report["loops"][1]["role"], "inner");
            let source = s.accepted_body().unwrap();
            let section = source.section_by_local_z(height, p).unwrap();
            assert_eq!(report["curves"].as_array().unwrap().len(), 8);
            let delta = (16. - height * height).sqrt();
            let frame = source.frame();
            for (i, edge) in section.edges().iter().enumerate() {
                let Curve::Nurbs(c) = &edge.curve else {
                    panic!("section curve must remain rational")
                };
                assert_eq!(
                    report["curves"][i]["curve"]["weights"],
                    serde_json::json!(c.weights())
                );
                assert_eq!(
                    report["curves"][i]["source_uses"].as_array().unwrap().len(),
                    if height == 0. { 2 } else { 1 }
                );
                for j in 0..33 {
                    let t = j as f64 / 32.;
                    let point = c.evaluate(t).unwrap();
                    let local = frame.axes().map(|a| a.dot(point - frame.origin()));
                    assert!((local[2] - height).abs() < 1e-9);
                    assert!(
                        (local[0].hypot(local[1]) - (16. + if i < 4 { delta } else { -delta }))
                            .abs()
                            < 1e-9
                    );
                    for u in section.source_uses().iter().filter(|u| u.edge == i) {
                        let uv = u.pcurve.try_evaluate(t).unwrap();
                        assert!(
                            (source.solid().shell.faces[u.face]
                                .surface
                                .try_evaluate(uv[0], uv[1])
                                .unwrap()
                                - point)
                                .norm()
                                < 1e-9
                        );
                    }
                }
            }
            for w in section.loops() {
                for c in &w.coedges {
                    let uv = c.pcurve.try_evaluate(0.37).unwrap();
                    assert!(
                        (section.plane().try_evaluate(uv[0], uv[1]).unwrap()
                            - section.edges()[c.edge].curve.try_evaluate(0.37).unwrap())
                        .norm()
                            < 1e-9
                    );
                }
            }
            let fine: Value =
                serde_json::from_str(&s.section_horizontal(height, 0.05).unwrap()).unwrap();
            for i in 0..8 {
                assert_eq!(fine["curves"][i]["curve"], report["curves"][i]["curve"]);
            }
            assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
        }
        for (height, chord) in [
            (4., 0.1),
            (-4., 0.1),
            (1., 0.),
            (1., 1e-14),
            (f64::NAN, 0.1),
        ] {
            assert!(s.section_horizontal(height, chord).is_err());
            assert_eq!(s.accepted_horizontal_split().unwrap() as *const _, ptr);
        }
        assert_eq!(
            format!("{:?}", s.accepted_horizontal_split().unwrap()),
            before
        );
        assert_eq!(s.accepted_body().unwrap().export_step_mm(p).unwrap(), step);
    }
}
