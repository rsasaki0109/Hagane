fn main() -> hagane::Result<()> {
    println!("{}", hagane::demo_json(14.0, 0.05)?);
    Ok(())
}
