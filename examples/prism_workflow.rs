use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let text = match std::env::args().nth(1) {
        Some(text) => text,
        None => {
            serde_json::json!({"command":"rebuild","document":prism_workflow_example_document()?})
                .to_string()
        }
    };
    let mut session = PrismWorkflowSession::new();
    if let Ok(serde_json::Value::Array(commands)) = serde_json::from_str(&text) {
        let mut reports = Vec::new();
        for command in commands {
            reports.push(serde_json::from_str::<serde_json::Value>(
                &prism_workflow_session_command_json(&mut session, &command.to_string())?,
            )?);
        }
        println!("{}", serde_json::to_string(&reports)?);
    } else {
        println!(
            "{}",
            prism_workflow_session_command_json(&mut session, &text)?
        );
    }
    Ok(())
}
