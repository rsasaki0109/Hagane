use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mode = args
        .first()
        .map(String::as_str)
        .unwrap_or("0")
        .parse()
        .map_err(|_| Error::InvalidInput("mode must be 0..2"))?;
    let value = |i: usize, default: &str| -> Result<f64> {
        args.get(i)
            .map(String::as_str)
            .unwrap_or(default)
            .parse()
            .map_err(|_| Error::InvalidInput("offset and placement must be numbers"))
    };
    println!(
        "{}",
        extrusion_intersections_demo_json(mode, value(1, "14")?, value(2, "0")?)?
    );
    Ok(())
}
