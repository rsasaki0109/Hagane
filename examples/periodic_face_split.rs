use hagane::*;
fn main() -> Result<()> {
    // The original through-bore uses periodic full-circle rims.
    println!("{}", demo_preset_json(11, 14.0, 0.05)?);
    Ok(())
}
