use hagane::*;
use serde_json::Value;
#[test]
fn actual_slab_reports_preserve_separate_caches_and_replace_only_after_success() {
    let mut s = NurbsTorusDemoSession::new();
    assert!(s.slab_horizontal(-1., 1., 0.1).is_err());
    assert!(s.slab_step().is_err());
    let model = [16., 4., 0.37, 12., -5., 8., 1e-6, 0.1];
    s.apply(&model).unwrap();
    s.split_horizontal(1., 0.1).unwrap();
    let old_pair = format!("{:?}", s.accepted_horizontal_split().unwrap());
    let policy = GeometryTolerance::new(1e-6, GeometryTolerance::default().angular(), 0.).unwrap();
    let source_step = s.accepted_body().unwrap().export_step_mm(policy).unwrap();
    for [lo, hi] in [[-1., 1.], [0.5, 2.], [-2., -0.5], [0., 1.]] {
        let data: Value = serde_json::from_str(&s.slab_horizontal(lo, hi, 0.1).unwrap()).unwrap();
        let j = |z: f64| z * (16. - z * z).sqrt() + 16. * (z / 4.).asin();
        let expected = 32. * std::f64::consts::PI * (j(hi) - j(lo));
        assert!((data["volume"].as_f64().unwrap() - expected).abs() < 1e-8);
        let actual = s.accepted_slab().unwrap();
        assert_eq!(actual.heights(), [lo, hi]);
        assert_eq!(
            data["step"],
            actual
                .export_step_mm(s.accepted_slab_policy().unwrap())
                .unwrap()
        );
        assert_eq!(
            format!("{:?}", s.accepted_horizontal_split().unwrap()),
            old_pair
        );
        assert_eq!(
            s.accepted_body().unwrap().export_step_mm(policy).unwrap(),
            source_step
        );
    }
    let cached_step: Value = serde_json::from_str(&s.slab_step().unwrap()).unwrap();
    assert_eq!(
        cached_step["step"],
        s.accepted_slab().unwrap().export_step_mm(policy).unwrap()
    );
    let slab = format!("{:?}", s.accepted_slab().unwrap());
    let step = s.accepted_slab().unwrap().export_step_mm(policy).unwrap();
    for (lo, hi, chord) in [
        (-4., 1., 0.1),
        (-1., 4., 0.1),
        (1., -1., 0.1),
        (1., 1., 0.1),
        (-1., 1., 0.),
        (-1., 1., 1e-14),
    ] {
        assert!(s.slab_horizontal(lo, hi, chord).is_err());
        assert_eq!(format!("{:?}", s.accepted_slab().unwrap()), slab);
        assert_eq!(
            serde_json::from_str::<Value>(&s.slab_step().unwrap()).unwrap(),
            cached_step
        );
        assert_eq!(
            format!("{:?}", s.accepted_horizontal_split().unwrap()),
            old_pair
        );
    }
    s.split_horizontal(0., 0.1).unwrap();
    assert_eq!(format!("{:?}", s.accepted_slab().unwrap()), slab);
    assert_eq!(
        s.accepted_slab().unwrap().export_step_mm(policy).unwrap(),
        step
    );
    assert!(s.apply(&[4., 4., 0., 0., 0., 0., 1e-6, 0.1]).is_err());
    assert_eq!(format!("{:?}", s.accepted_slab().unwrap()), slab);
    s.apply(&model).unwrap();
    assert!(s.accepted_slab().is_none());
    assert!(s.slab_step().is_err());
    assert!(s.accepted_slab_policy().is_none());
    assert!(s.accepted_horizontal_split().is_none());
    s.slab_horizontal(-1., 1., 0.1).unwrap();
    s.split_horizontal(1., 0.1).unwrap();
    let text = s.accepted_body().unwrap().export_step_mm(policy).unwrap();
    assert!(s.import_step_cardinal(&text, 0.1, policy).is_err());
    assert!(s.accepted_slab().is_some());
    let source = NurbsTorusSolid::new(
        Frame3::new_with_tolerance(
            Point3::new(0., 0., 0.),
            [
                Vec3::new(1., 0., 0.),
                Vec3::new(0., 1., 0.),
                Vec3::new(0., 0., 1.),
            ],
            policy,
        )
        .unwrap(),
        [16., 4.],
        policy,
    )
    .unwrap();
    s.import_step_cardinal(&source.export_step_mm(policy).unwrap(), 0.1, policy)
        .unwrap();
    assert!(s.accepted_slab().is_none());
    assert!(s.slab_step().is_err());
    assert!(s.accepted_horizontal_split().is_none());
}
