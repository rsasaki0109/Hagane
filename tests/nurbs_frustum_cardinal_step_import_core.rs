use hagane::*;
fn axes() -> Vec<[Vec3; 3]> {
    let basis = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut out = vec![];
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }
            for si in [-1., 1.] {
                for sj in [-1., 1.] {
                    let u = basis[i] * si;
                    let v = basis[j] * sj;
                    out.push([u, v, u.cross(v)]);
                }
            }
        }
    }
    out
}
#[test]
fn all_cardinal_frames_retain_full_actual_coefficients_and_upper_children() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let frames = axes();
    assert_eq!(frames.len(), 24);
    for a in frames {
        for radii in [[16., 8.], [8., 16.], [12., 12.]] {
            let frame = Frame3::new(Point3::new(12., -5., 8.), a, p.absolute()).unwrap();
            let source = NurbsFrustumSolid::new(frame, radii, 24., p).unwrap();
            for body in [source.clone(), source.split_axial(10., p).unwrap().upper] {
                let step = body.export_step_mm(p).unwrap();
                let got = import_step_nurbs_frustum_cardinal_mm(&step, p).unwrap();
                assert_eq!(got.export_step_mm(p).unwrap(), step);
                assert_eq!(format!("{:?}", got.solid()), format!("{:?}", body.solid()));
                if a != Frame3::IDENTITY.axes() {
                    assert!(import_step_nurbs_frustum_translated_mm(&step, p).is_err());
                }
                assert!(import_step_nurbs_frustum_mm(&step, p).is_err());
            }
        }
    }
}
#[test]
fn arbitrary_angle_and_reversed_cap_orientation_are_rejected() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let (s, c) = 0.3f64.sin_cos();
    let frame = Frame3::new(
        Point3::new(0., 0., 0.),
        [
            Vec3::new(c, s, 0.),
            Vec3::new(-s, c, 0.),
            Vec3::new(0., 0., 1.),
        ],
        p.absolute(),
    )
    .unwrap();
    let source = NurbsFrustumSolid::new(frame, [16., 8.], 24., p).unwrap();
    assert!(import_step_nurbs_frustum_cardinal_mm(&source.export_step_mm(p).unwrap(), p).is_err());
    let source = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., p).unwrap();
    let step = source.export_step_mm(p).unwrap();
    let changed = step
        .lines()
        .map(|line| {
            if line.contains("ADVANCED_FACE") && line.ends_with(",.F.);") {
                line.replace(",.F.);", ",.T.);")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(step, changed);
    assert!(import_step_nurbs_frustum_cardinal_mm(&changed, p).is_err());
}
