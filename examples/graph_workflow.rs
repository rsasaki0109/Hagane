use hagane::*;
use serde_json::Value;
use std::io::Read;
fn read(path: &str, limit: u64) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err("workflow input exceeds its byte limit".into());
    }
    Ok(text)
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut session = GraphWorkflowSession::new();
    if args.first().is_some_and(|s| s == "--session") {
        if args.len() != 2 {
            return Err("usage: graph_workflow --session PATH".into());
        }
        let commands: Vec<Value> = serde_json::from_str(&read(&args[1], 1024 * 1024)?)?;
        let mut reports = Vec::new();
        for command in commands {
            let result = match command["op"].as_str() {
                Some("rebuild") => session.evaluate_json(&command["document"].to_string()),
                Some("export_step") => graph_workflow_step_export_json(&session),
                Some("query") => {
                    match (
                        command["point"].as_array(),
                        command["linear_tolerance"].as_f64(),
                    ) {
                        (Some(p), Some(t))
                            if p.len() == 3 && p.iter().all(|x| x.as_f64().is_some()) =>
                        {
                            graph_workflow_point_query_json(
                                &session,
                                Point3::new(
                                    p[0].as_f64().unwrap(),
                                    p[1].as_f64().unwrap(),
                                    p[2].as_f64().unwrap(),
                                ),
                                t,
                            )
                        }
                        _ => Err(Error::InvalidInput(
                            "query needs numeric XYZ and linear_tolerance",
                        )),
                    }
                }
                Some("reset") => {
                    session = GraphWorkflowSession::new();
                    Ok("{\"ok\":true}".into())
                }
                _ => Err(Error::InvalidInput(
                    "unknown graph workflow session command",
                )),
            };
            let (status, text) = graph_workflow_output(result);
            reports.push(
                serde_json::json!({"status":status,"result":serde_json::from_str::<Value>(&text)?}),
            );
        }
        println!("{}", serde_json::to_string(&reports)?);
    } else {
        if args.len() > 1 {
            return Err("usage: graph_workflow [PATH]".into());
        }
        let text = read(
            args.first()
                .map(String::as_str)
                .unwrap_or("docs/graph-workflow.json"),
            64 * 1024,
        )?;
        let (status, report) = graph_workflow_output(session.evaluate_json(&text));
        println!("{report}");
        if status != 0 {
            std::process::exit(1);
        }
    }
    Ok(())
}
