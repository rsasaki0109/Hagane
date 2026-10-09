fn main() -> hagane::Result<()> {
    let input = std::env::args()
        .nth(1)
        .map(std::fs::read_to_string)
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("cannot read operation document"))?
        .unwrap_or_else(|| include_str!("../docs/workflow-example.json").to_owned());
    println!("{}", hagane::evaluate_workflow_json(&input)?);
    Ok(())
}
