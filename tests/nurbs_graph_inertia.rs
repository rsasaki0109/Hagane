use hagane::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 2e-12 * b.abs().max(1e-100), "{a} != {b}");
}
#[test]
fn flat_box_and_mutation() {
    let tol = Tolerance::default();
    let mut body = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let p = body.inertia_properties(tol).unwrap();
    near(p.volume, 96.);
    for (i, want) in [320., 544., 800.].into_iter().enumerate() {
        near(p.inertia[i][i], want);
    }
    assert!(p.inertia[0][1].abs() < 1e-12);
    body.solid.vertices[0].point.x += 1e-10;
    assert!(body.inertia_properties(tol).is_err());
}
#[test]
fn hole_parallel_axis_and_placement() {
    let tol = Tolerance::default();
    let body = NurbsGraphSolid::new([8., 6., 2.], 0., tol).unwrap();
    let hole = NurbsGraphHoledSolid::new(&body, [[0.2, 0.4], [0.3, 0.7]], tol).unwrap();
    let p = hole.inertia_properties(tol).unwrap();
    let removed = 96. * 0.2 * 0.4;
    let fullcenter = Point3::new(4., 3., 1.);
    let cutcenter = Point3::new(2.4, 3., 1.);
    let full = [320., 544., 800.];
    let cut = [
        removed * (2.4f64.powi(2) + 4.) / 12.,
        removed * (1.6f64.powi(2) + 4.) / 12.,
        removed * (1.6f64.powi(2) + 2.4f64.powi(2)) / 12.,
    ];
    for i in 0..3 {
        let da = fullcenter - p.centroid;
        let db = cutcenter - p.centroid;
        let a = [da.x, da.y, da.z];
        let b = [db.x, db.y, db.z];
        let want = full[i] + 96. * (da.dot(da) - a[i] * a[i])
            - cut[i]
            - removed * (db.dot(db) - b[i] * b[i]);
        near(p.inertia[i][i], want);
    }
    let translation = Transform::translation(Vec3::new(1e5, -2e5, 3e5)).unwrap();
    let moved = body
        .transformed(translation, Tolerance::new(1e-3).unwrap())
        .unwrap();
    let shifted = NurbsGraphHoledSolid::new(&moved, hole.hole(), Tolerance::new(1e-3).unwrap())
        .unwrap()
        .inertia_properties(Tolerance::new(1e-3).unwrap())
        .unwrap();
    assert_eq!(p.inertia, shifted.inertia);
}
#[test]
fn scaling_and_representability() {
    for scale in [1e-50, 1e50, 1e-80, 1e100] {
        let tol = Tolerance::new(scale * 1e-10).unwrap();
        let body = NurbsGraphSolid::new([8. * scale, 6. * scale, 2. * scale], 0., tol).unwrap();
        assert!(body.mass_properties(tol).is_ok());
        let result = body.inertia_properties(tol);
        if scale == 1e-80 || scale == 1e100 {
            assert!(result.is_err());
        } else {
            let p = result.unwrap();
            near(p.inertia[0][0] / scale.powi(5), 320.);
        }
    }
}
