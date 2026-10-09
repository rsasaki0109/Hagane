use hagane::*;
fn main() -> Result<()> {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            points.push(Point3::new(i as f64 * 3., j as f64 * 3., (i * j) as f64));
            weights.push(1. + 0.05 * (i + j) as f64);
        }
    }
    let surface = NurbsSurface::new(
        [2, 2],
        [
            vec![2., 2., 2., 4., 6., 6., 6.],
            vec![-3., -3., -3., 1., 5., 5., 5.],
        ],
        [4, 4],
        points,
        weights,
    )?;
    let face = NurbsPolygonFace::new(
        surface,
        vec![[2.5, -2.], [5.5, -1.], [3., 4.]],
        1,
        Tolerance::default(),
    )?;
    let display = face.tessellate_bounded(0.05, 65536, Tolerance::default())?;
    eprintln!(
        "{} triangles across four C1 source spans; maximum engineering bound {}",
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
