fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("docs/step-tetrahedron-metres.step".into());
    let input = std::fs::read_to_string(path)?;
    println!("{}", hagane::import_step_json(&input)?);
    Ok(())
}
