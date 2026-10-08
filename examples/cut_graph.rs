use hagane::*;
fn main() -> Result<()> {
    // The identical fixture runs in the browser as preset 9.
    println!("{}", demo_preset_json(9, 14.0, 0.05)?);
    Ok(())
}
