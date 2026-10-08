use hagane::*;
fn main() -> Result<()> {
    // Independent planar patches include one unmatched boundary subdivision.
    println!("{}", demo_preset_json(12, 14.0, 0.05)?);
    Ok(())
}
