//! Exact rounded-rectangle line/arc extrusion, placed on an arbitrary plane.
fn main() -> hagane::Result<()> {
    use hagane::*;
    let tolerance = Tolerance::default();
    let profile =
        rounded_rectangle_profile(Point3::new(0.0, 0.0, -12.0), 80.0, 60.0, 14.0, tolerance)?;
    let part = extrude_arc_line(&profile, 24.0, tolerance)?;
    let placed = part.transformed(
        Transform::rotation(Vec3::new(1.0, 2.0, 0.5), 0.3)?,
        tolerance,
    )?;
    println!("{}", placed.mesh_json(0.05, tolerance)?);
    Ok(())
}
