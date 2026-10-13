use hagane::*;
#[test]
fn actual_closed_section_rims_keep_all_original_source_seam_owners() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    let before = source.export_step_mm(p).unwrap();
    for cut in [0., 1.5, -1.5] {
        let section = source.section_by_local_z(cut, p).unwrap();
        let split = source.split_by_local_z(cut, p).unwrap();
        let child = &split.lower;
        let n = (child.solid().shell.faces.len() - 1) / 4;
        assert_eq!(section.source_uses().len(), if cut == 0. { 16 } else { 8 });
        for ring in 0..2 {
            for i in 0..4 {
                let edge = &section.edges()[ring * 4 + i];
                let old = &child.solid().edges[i * (n + 1) + if ring == 0 { n } else { 0 }];
                assert_eq!(format!("{:?}", edge.curve), format!("{:?}", old.curve));
                for t in [0., 0.23, 0.57, 1.] {
                    let point = edge.curve.try_evaluate(t).unwrap();
                    let radius = 16.
                        + if ring == 0 { 1. } else { -1. }
                            * 4.
                            * ((1. - cut / 4.) * (1. + cut / 4.)).sqrt();
                    assert!((point.x.hypot(point.y) - radius).abs() < 1e-11);
                    assert!((point.z - cut).abs() < 1e-11);
                }
            }
        }
        for use_ in section.source_uses() {
            let uv = use_.pcurve.try_evaluate(0.37).unwrap();
            let actual = source.solid().shell.faces[use_.face]
                .surface
                .try_evaluate(uv[0], uv[1])
                .unwrap();
            assert!(
                (actual - section.edges()[use_.edge].curve.try_evaluate(0.37).unwrap()).norm()
                    < 1e-11
            );
        }
        assert!(section
            .loops()
            .iter()
            .all(|w| w.coedges.iter().all(|c| c.forward)));
    }
    assert!(source.section_by_local_z(4., p).is_err());
    assert_eq!(before, source.export_step_mm(p).unwrap());
}
