use hagane::*;
fn surface(z: f64) -> NurbsSurface {
    NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 10., 0.),
            Point3::new(10., 0., 0.),
            Point3::new(10., 10., z),
        ],
        vec![1.; 4],
    )
    .unwrap()
}
fn corners() -> Vec<[f64; 2]> {
    vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]]
}
#[test]
fn retained_face_and_affine_interior_have_analytic_area_and_orientation() {
    for orientation in [-1, 1] {
        let face = NurbsPolygonFace::new(surface(0.), corners(), orientation, Tolerance::default())
            .unwrap();
        let mesh = face.tessellate_affine(1e-6, Tolerance::default()).unwrap();
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.positions.len(), 3);
        let [a, b, c] = mesh.triangles[0];
        let cross =
            (mesh.positions[b] - mesh.positions[a]).cross(mesh.positions[c] - mesh.positions[a]);
        assert!((cross.z - 62. * orientation as f64).abs() < 1e-12);
        for n in mesh.normals {
            assert_eq!(n.z, orientation as f64);
        }
        assert_eq!(face.face.wires[0].coedges[2].edge, 2);
    }
}
#[test]
fn curved_face_is_retained_but_display_is_explicitly_unsupported() {
    let face = NurbsPolygonFace::new(surface(4.), corners(), 1, Tolerance::default()).unwrap();
    face.validate(Tolerance::default()).unwrap();
    assert!(matches!(
        face.tessellate_affine(1., Tolerance::default()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn corrupt_surface_wire_orientation_and_precision_are_rejected() {
    let face = NurbsPolygonFace::new(surface(0.), corners(), 1, Tolerance::default()).unwrap();
    let mut dirty = face.clone();
    dirty.face.wires[0].coedges[0].edge = 2;
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = face.clone();
    dirty.face.surface = Surface::Nurbs(Box::new(surface(1.)));
    assert!(dirty.validate(Tolerance::default()).is_err());
    let mut dirty = face.clone();
    dirty.face.orientation = 0;
    assert!(dirty.validate(Tolerance::default()).is_err());
    assert!(face.tessellate_affine(1e-30, Tolerance::default()).is_err());
    let mut clockwise = corners();
    clockwise.reverse();
    assert!(NurbsPolygonFace::new(surface(0.), clockwise, 1, Tolerance::default()).is_err());
}
