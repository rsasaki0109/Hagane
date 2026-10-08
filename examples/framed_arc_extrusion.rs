use hagane::*;
fn main() -> Result<()> {
    let radius = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "14".into())
        .parse()
        .map_err(|_| Error::InvalidInput("radius must be a number"))?;
    println!(
        "{}",
        framed_arc_extrusion_demo(radius)?.mesh_json(0.05, Tolerance::default())?
    );
    Ok(())
}
