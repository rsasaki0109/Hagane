use hagane::*;

// args: world_offset radius chord_error [step]
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| -> std::result::Result<f64, Box<dyn std::error::Error>> {
        Ok(if let Some(value) = args.get(i) {
            value.parse()?
        } else {
            default
        })
    };
    let offset = number(0, 1e12)?;
    let radius = number(1, 8.)?;
    let error = number(2, 0.05)?;
    let tolerance = Tolerance::default();
    let solid = make_cylinder(
        CylinderSpec {
            base: Point3::new(0., 0., -12.),
            radius,
            height: 24.,
        },
        tolerance,
    )?
    .transformed(
        Transform::translation(Vec3::new(offset, -offset, offset))?,
        tolerance,
    )?;
    certify_circular_prism(&solid, tolerance)?;
    if args.get(3).map(String::as_str) == Some("step") {
        print!("{}", export_step_mm(&solid, tolerance)?);
        return Ok(());
    }
    let allowance = solid.tessellation_roundoff_budget(tolerance)?;
    let mesh = solid.tessellate(error, tolerance)?;
    println!(
        "{}",
        serde_json::json!({
            "units": "mm", "world_offset": offset, "radius": radius,
            "requested_chord_error": error, "coordinate_allowance": allowance,
            "exact_volume": solid.volume()?, "mesh_volume": mesh.signed_volume(),
            "triangles": mesh.triangles.len(),
        })
    );
    Ok(())
}
