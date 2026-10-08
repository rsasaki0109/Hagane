fn main() -> hagane::Result<()> {
    let preset = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("demo preset must be an integer"))?
        .unwrap_or(0);
    println!("{}", hagane::demo_preset_json(preset, 14.0, 0.05)?);
    Ok(())
}
