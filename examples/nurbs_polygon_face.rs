use hagane::*;
fn main() -> Result<()> {
    let surface = NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]],
        [2, 2],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(0., 10., 0.),
            Point3::new(10., 0., 0.),
            Point3::new(10., 10., 0.),
        ],
        vec![1.; 4],
    )?;
    let face = NurbsPolygonFace::new(
        surface,
        vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]],
        1,
        Tolerance::default(),
    )?;
    let mesh = face.tessellate_affine(0.001, Tolerance::default())?;
    // Export the actual B-rep-derived display as a Wavefront OBJ on stdout.
    for p in mesh.positions {
        println!("v {} {} {}", p.x, p.y, p.z);
    }
    for [a, b, c] in mesh.triangles {
        println!("f {} {} {}", a + 1, b + 1, c + 1);
    }
    Ok(())
}
