fn main() -> hagane::Result<()> {
    let number = |i, default| {
        std::env::args()
            .nth(i)
            .map(|s| s.parse::<f64>())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("radius/depth must be numeric"))
            .map(|v| v.unwrap_or(default))
    };
    println!(
        "{}",
        hagane::blind_bore_demo_json(number(1, 14.)?, number(2, 16.)?)?
    );
    Ok(())
}
