use hagane::*;
fn main() -> Result<()> {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                i as f64 * 4.,
                j as f64 * 4.,
                if i == 1 { 3. } else { 0. } + if j == 1 { 2. } else { 0. },
            ));
            weights.push(1. + 0.1 * (i + j) as f64);
        }
    }
    let knots = vec![0., 0., 0.5, 1., 1.];
    let surface = NurbsSurface::new([1, 1], [knots.clone(), knots], [3, 3], points, weights)?;
    let face = NurbsPolygonFace::new(
        surface,
        vec![[0.125, 0.125], [0.875, 0.25], [0.25, 0.875]],
        1,
        Tolerance::default(),
    )?;
    let display = face.tessellate_crease_bounded(0.02, 65536, Tolerance::default())?;
    eprintln!("{} triangles; {} display vertices with shared geometric nodes; maximum engineering bound {}",display.mesh.triangles.len(),display.mesh.positions.len(),display.error_bounds.iter().copied().fold(0f64,f64::max));
    for p in display.mesh.positions {
        println!("v {} {} {}", p.x, p.y, p.z);
    }
    for n in display.mesh.normals {
        println!("vn {} {} {}", n.x, n.y, n.z);
    }
    for [a, b, c] in display.mesh.triangles {
        println!(
            "f {}//{} {}//{} {}//{}",
            a + 1,
            a + 1,
            b + 1,
            b + 1,
            c + 1,
            c + 1
        );
    }
    Ok(())
}
