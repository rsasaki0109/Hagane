use hagane::*;
#[test]
fn posed_minor_seams_choose_the_same_actual_quarter_on_both_arithmetic_sides() {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.37).unwrap();
    let frame =
        Frame3::new_with_tolerance(Point3::new(12., -5., 8.), rotation.axes(), policy).unwrap();
    let body = NurbsTorusSolid::new(frame, [16., 4.], policy).unwrap();
    for (j, radial, z) in [(0, 4., 0.), (1, 0., 4.), (2, -4., 0.), (3, 0., -4.)] {
        for offset in [-1e-12, 0., 1e-12] {
            let local = if j % 2 == 0 {
                Point3::new(16. + radial, offset, offset)
            } else {
                Point3::new(16. + offset, offset, z)
            };
            let query = body.point_query(frame.point(local), policy).unwrap();
            assert_eq!(query.location, PointLocation::Boundary);
            let witness = query.boundary.unwrap();
            assert_eq!(witness.face, j);
            assert_eq!(witness.uv, [0., 0.]);
            let actual = body.solid().shell.faces[j]
                .surface
                .try_evaluate(0., 0.)
                .unwrap();
            assert_eq!(actual, witness.point);
            assert!(witness.distance < 1e-10);
        }
    }
    let result = body
        .intersect_segment(
            frame.point(Point3::new(16., 0., -8.)),
            frame.point(Point3::new(16., 0., 8.)),
            policy,
        )
        .unwrap();
    assert_eq!(result.hits[0].boundary.face, 3);
    assert_eq!(result.hits[0].boundary.uv, [0., 0.]);
    assert_eq!(result.hits[1].boundary.face, 1);
}
