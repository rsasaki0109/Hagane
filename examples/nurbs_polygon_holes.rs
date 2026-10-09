use hagane::*;
fn main() -> Result<()> {
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                i as f64 * 20.,
                j as f64 * 20.,
                if i == 1 && j == 1 { 15. } else { 0. },
            ));
            weights.push(if i == 1 && j == 1 { 1.5 } else { 1. });
        }
    }
    let knots = vec![0., 0., 0., 1., 1., 1.];
    let surface = NurbsSurface::new([2, 2], [knots.clone(), knots], [3, 3], points, weights)?;
    let face = NurbsPolygonHoledFace::new(
        surface,
        vec![[0.125, 0.125], [0.875, 0.25], [0.75, 0.875], [0.25, 0.75]],
        vec![
            [[0.3125, 0.4375], [0.3125, 0.4375]],
            [[0.5625, 0.6875], [0.5625, 0.6875]],
        ],
        1,
        Tolerance::default(),
    )?;
    let display = face.tessellate_bounded(0.1, 65536, Tolerance::default())?;
    eprintln!("{} exact B-rep wires, {} shared boundary vertices; {} material triangles; maximum engineering bound {}",face.face.wires.len(),face.vertices.len(),display.mesh.triangles.len(),display.error_bounds.iter().copied().fold(0f64,f64::max));
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
