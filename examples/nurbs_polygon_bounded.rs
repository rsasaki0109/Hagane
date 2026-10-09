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
            Point3::new(10., 10., 8.),
        ],
        vec![1., 1.25, 1.5, 1.],
    )?;
    let face = NurbsPolygonFace::new(
        surface,
        vec![[0.125, 0.125], [0.875, 0.25], [0.75, 0.875], [0.25, 0.75]],
        1,
        Tolerance::default(),
    )?;
    let display = face.tessellate_bilinear_bounded(0.02, 65536, Tolerance::default())?;
    eprintln!(
        "{} triangles; maximum engineering bound {}",
        display.mesh.triangles.len(),
        display.error_bounds.iter().copied().fold(0f64, f64::max)
    );
    for p in display.mesh.positions {
        println!("v {} {} {}", p.x, p.y, p.z);
    }
    for [a, b, c] in display.mesh.triangles {
        println!("f {} {} {}", a + 1, b + 1, c + 1);
    }
    Ok(())
}
