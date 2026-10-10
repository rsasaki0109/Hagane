use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let request = match std::env::args().nth(1) {
        Some(text) => text,
        None => {
            let mut doc = serde_json::to_value(nurbs_frustum_workflow_example_document()?)?;
            doc["operations"][0] = serde_json::json!({"kind":"posed_frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-5.,8.],"rotation_axis":[1.,2.,3.],"angle":0.37});
            serde_json::json!({"command":"rebuild","document":doc}).to_string()
        }
    };
    let mut session = NurbsFrustumWorkflowSession::new();
    if let Ok(serde_json::Value::Array(commands)) = serde_json::from_str(&request) {
        let mut reports = Vec::new();
        for command in commands {
            reports.push(serde_json::from_str::<serde_json::Value>(
                &nurbs_frustum_workflow_session_command_json(&mut session, &command.to_string())?,
            )?);
        }
        println!("{}", serde_json::to_string(&reports)?);
    } else {
        println!(
            "{}",
            nurbs_frustum_workflow_session_command_json(&mut session, &request)?
        );
    }
    Ok(())
}
