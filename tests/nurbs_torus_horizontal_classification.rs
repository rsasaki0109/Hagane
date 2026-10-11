use hagane::*;
#[test]
fn finite_annular_cap_and_retained_arc_witnesses_exclude_physical_rims() {
    let p = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
    let source = NurbsTorusSolid::new(Frame3::IDENTITY, [16., 4.], p).unwrap();
    for c in [0., 1.5, -1.5] {
        let split = source.split_by_local_z(c, p).unwrap();
        for body in [split.lower, split.upper] {
            let before = body.export_step_mm(p).unwrap();
            let cap = Point3::new(16., 0., c);
            let query = body.point_query(cap, p).unwrap();
            assert_eq!(query.location, PointLocation::Boundary);
            let witness = query.boundary.unwrap();
            assert_eq!(witness.face, body.solid().shell.faces.len() - 1);
            assert!(
                (body.solid().shell.faces[witness.face]
                    .surface
                    .try_evaluate(witness.uv[0], witness.uv[1])
                    .unwrap()
                    - witness.point)
                    .norm()
                    < 1e-12
            );
            let side = if body.is_lower() { -1. } else { 1. };
            assert_eq!(
                body.classify_point(Point3::new(16., 0., c + side * 0.1), p)
                    .unwrap(),
                PointLocation::Inside
            );
            assert_eq!(
                body.classify_point(Point3::new(16., 0., c - side * 0.1), p)
                    .unwrap(),
                PointLocation::Outside
            );
            assert_eq!(
                body.classify_point(Point3::new(0., 0., c), p).unwrap(),
                PointLocation::Outside
            );
            let rim = 16. + 4. * ((1. - c / 4.) * (1. + c / 4.)).sqrt();
            assert!(body.classify_point(Point3::new(rim, 0., c), p).is_err());
            let Surface::Nurbs(face) = &body.solid().shell.faces[0].surface else {
                panic!()
            };
            let domain = face.domain();
            let v = (domain[1][0] + domain[1][1]) * 0.5;
            let point = face.evaluate(0.37, v).unwrap();
            let q = body.point_query(point, p).unwrap();
            assert_eq!(q.location, PointLocation::Boundary);
            assert!(q.boundary.unwrap().face < body.solid().shell.faces.len() - 1);
            assert_eq!(before, body.export_step_mm(p).unwrap());
        }
    }
}
