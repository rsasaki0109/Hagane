fn main() -> hagane::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let mut read = |default| {
        arguments
            .next()
            .map(|v| v.parse::<f64>())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("scale and angle must be numbers"))
            .map(|v| v.unwrap_or(default))
    };
    let scale = read(10.0)?;
    let angle = read(0.1)?;
    println!("{}", hagane::predicates_demo_json(scale, angle)?);
    Ok(())
}
