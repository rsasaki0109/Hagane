fn main() -> hagane::Result<()> {
    let angle = std::env::args()
        .nth(1)
        .map(|s| s.parse::<f64>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("azimuth must be numeric"))?
        .unwrap_or(0.4);
    println!("{}", hagane::oriented_bores_demo_json(angle, 1.)?);
    Ok(())
}
