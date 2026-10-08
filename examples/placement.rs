//! Exact through-bore part placed in an arbitrary right-handed frame.
fn main() -> hagane::Result<()> {
    use hagane::*;
    let tol = Tolerance::default();
    let part = subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-40.0, -30.0, -12.0),
            size: Vec3::new(80.0, 60.0, 24.0),
        },
        CylinderSpec {
            base: Point3::new(0.0, 0.0, -20.0),
            radius: 14.0,
            height: 40.0,
        },
        tol,
    )?;
    let frame = Transform::translation(Vec3::new(8.0, -4.0, 6.0))?
        .compose(Transform::rotation(Vec3::new(1.0, 2.0, 0.5), 0.8)?)?;
    let placed = part.transformed(frame, tol)?;
    println!("{}", placed.mesh_json(0.05, tol)?);
    Ok(())
}
