fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("docs/workflow-skew-extrusion-example.json".into());
    let text = std::fs::read_to_string(path)?;
    let report: serde_json::Value =
        serde_json::from_str(&hagane::export_workflow_step_mm_json(&text)?)?;
    print!("{}", report["step"].as_str().unwrap());
    Ok(())
}
