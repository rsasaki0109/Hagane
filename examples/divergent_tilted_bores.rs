fn main() -> hagane::Result<()> {
    let tilt = std::env::args()
        .nth(1)
        .map(|s| s.parse::<f64>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("tilt must be numeric"))?
        .unwrap_or(0.7);
    let offset = std::env::args()
        .nth(2)
        .map(|s| s.parse::<f64>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("offset must be numeric"))?
        .unwrap_or(3.);
    println!(
        "{}",
        hagane::divergent_tilted_bores_demo_json(tilt, offset)?
    );
    Ok(())
}
