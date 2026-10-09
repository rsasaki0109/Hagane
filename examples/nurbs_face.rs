use hagane::*;
fn main() -> Result<()> {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (i, (x, y)) in [(1., 0.), (1., 1.), (0., 1.)].into_iter().enumerate() {
        for z in [0., 2.] {
            points.push(Point3::new(x, y, z));
            weights.push(if i == 1 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.
            });
        }
    }
    let surface = NurbsSurface::new(
        [2, 1],
        [vec![0., 0., 0., 1., 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        points,
        weights,
    )?;
    let face = NurbsFace::new(surface, 1, Tolerance::default())?;
    let mesh = face.sample_grid([32, 8], Tolerance::default())?;
    println!(
        "{}",
        serde_json::json!({"vertices":face.vertices.len(),"edges":face.edges.len(),"faces":1,"closed":false,"display_triangles":mesh.triangles.len(),"surface_error_bound":null})
    );
    Ok(())
}
