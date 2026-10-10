use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.)?;
    let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy)?;
    let result = body.split_axial_many(&[6., 12., 18.], policy)?;
    for (index, part) in result.parts.iter().enumerate() {
        println!(
            "Part {index}: height={}, radii={:?}, volume={}",
            part.height(),
            part.radii(),
            part.volume(policy)?
        );
        if let Some(directory) = std::env::args().nth(1) {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("part-{index}.step")),
                part.export_step_mm(policy)?,
            )?;
        }
    }
    Ok(())
}
