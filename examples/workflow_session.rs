//! Evaluate a series of edits in one native incremental session.
fn main() -> hagane::Result<()> {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let inputs = if files.is_empty() {
        let text = include_str!("../docs/workflow-multiple-example.json");
        let mut edited: serde_json::Value = serde_json::from_str(text).unwrap();
        edited["operations"][2]["radius"] = serde_json::json!(5.);
        vec![text.to_owned(), text.to_owned(), edited.to_string()]
    } else {
        files
            .iter()
            .map(std::fs::read_to_string)
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|_| hagane::Error::InvalidInput("cannot read operation document"))?
    };
    let mut session = hagane::WorkflowSession::new();
    for input in inputs {
        println!("{}", session.evaluate_json(&input)?);
    }
    Ok(())
}
