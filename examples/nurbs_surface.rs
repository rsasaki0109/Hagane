fn main() -> hagane::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let mut value = |default: f64| -> hagane::Result<f64> {
        arguments
            .next()
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("expected numeric height, weight, u, v"))
            .map(|v| v.unwrap_or(default))
    };
    let height = value(35.0)?;
    let weight = value(1.0)?;
    let u = value(0.5)?;
    let v = value(0.5)?;
    println!("{}", hagane::nurbs_surface_demo_json(height, weight, u, v)?);
    Ok(())
}
