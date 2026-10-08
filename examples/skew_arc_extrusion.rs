use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let value = |index: usize, default: &str| -> Result<f64> {
        args.get(index)
            .map(String::as_str)
            .unwrap_or(default)
            .parse()
            .map_err(|_| Error::InvalidInput("expected radius, skew offset and normal height"))
    };
    println!(
        "{}",
        skew_arc_extrusion_demo_json(value(0, "14")?, value(1, "14")?, value(2, "-24")?)?
    );
    Ok(())
}
