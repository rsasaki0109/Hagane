use hagane::*;
fn main() -> Result<()> {
    let tol = GeometryTolerance::new(1e-4, 1e-10, 1e-12)?;
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol.absolute())?;
    let body = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol.absolute())?;
    for uv in [[0.25, 0.25], [0.5, 0.5], [-0.2, 0.5]] {
        let section = body.vertical_section(uv, tol)?;
        println!(
            "UV {uv:?}: physical-height intervals {:?}, {} crossings",
            section.intervals,
            section.events.len()
        );
    }
    Ok(())
}
