use hagane::*;
fn main() -> Result<()> {
    // Exact bounded semicircular rims, not a periodic circle approximation.
    println!("{}", demo_preset_json(10, 14.0, 0.05)?);
    Ok(())
}
