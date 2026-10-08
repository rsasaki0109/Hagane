use hagane::*;
fn main() -> hagane::Result<()> {
    let tol = GeometryTolerance::default();
    let solid = make_box(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        tol.absolute(),
    )?;
    let result = split_planar_face(
        &solid,
        0,
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        tol,
    )?;
    result.solid.validate(tol.absolute())?;
    println!(
        "faces: {:?}; cut edge: {}; vertices: {:?}; volume: {}",
        result.faces,
        result.cut_edge,
        result.cut_vertices,
        result.solid.volume()?
    );
    Ok(())
}
