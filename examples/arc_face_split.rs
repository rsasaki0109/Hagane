use hagane::*;
fn main() -> hagane::Result<()> {
    let tol = GeometryTolerance::default();
    let profile =
        rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, tol.absolute())?;
    let solid = extrude_arc_line(&profile, 2.0, tol.absolute())?;
    let split = split_planar_face(
        &solid,
        0,
        Point3::new(0.0, 4.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        tol,
    )?;
    split.solid.validate(tol.absolute())?;
    println!(
        "faces: {:?}; total faces: {}; cut edge: {}; volume: {}",
        split.faces,
        split.solid.shell.faces.len(),
        split.cut_edge,
        split.solid.volume()?
    );
    Ok(())
}
