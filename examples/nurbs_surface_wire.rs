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
            Point3::new(10., 10., 4.),
        ],
        vec![1.; 4],
    )?;
    let wire = NurbsSurfaceWire::new(
        surface,
        vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]],
        Tolerance::default(),
    )?;
    let lines = wire.tessellate_boundary(0.01, 1024, Tolerance::default())?;
    println!(
        "Closed {:?} UV boundary: {} shared vertices, {} exact edges, {} display polylines",
        wire.orientation()?,
        wire.vertices.len(),
        wire.edges.len(),
        lines.len()
    );
    Ok(())
}
