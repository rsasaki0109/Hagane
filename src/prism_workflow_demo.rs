//! JSON transport for the versioned multi-component normal-prism workflow.
use crate::*;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Rebuild { document: PrismWorkflowDocument },
    Undo,
    Redo,
    Reset,
}

pub fn prism_workflow_example_document() -> Result<PrismWorkflowDocument> {
    serde_json::from_str(include_str!("../docs/prism-workflow-example.json"))
        .map_err(|_| Error::InvalidInput("invalid bundled prism workflow document"))
}

fn body_report(shape: &PrismWorkflowShape, document: &PrismWorkflowDocument) -> Result<Value> {
    let tolerance = GeometryTolerance::new(
        document.tolerance.linear,
        document.tolerance.angular,
        document.tolerance.relative,
    )?;
    let components: Result<Vec<_>> = shape
        .components()
        .iter()
        .map(|s| {
            crate::edge_fillet_demo::solid_json(
                s,
                document.display_chord_tolerance,
                tolerance.absolute(),
            )
        })
        .collect();
    let steps: Result<Vec<_>> = shape
        .components()
        .iter()
        .map(|s| export_step_bounded_analytic_mm(s, tolerance.linear()))
        .collect();
    let mut volume = 0.;
    let mut correction = 0.;
    for body in shape.components() {
        let adjusted = body.volume()? - correction;
        let next = volume + adjusted;
        correction = (next - volume) - adjusted;
        volume = next;
    }
    if !volume.is_finite() {
        return Err(Error::Unsupported(
            "prism component total volume is unresolved",
        ));
    }
    Ok(
        json!({"ok":true,"document":document,"components":components?,"component_steps":steps?,
        "output":{"id":document.output,"component_count":shape.components().len(),"total_volume":volume},
        "units":"mm","step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "scope":"versioned normal line/quarter-arc prism graph; exact single-component operands with explicit component selection; same physical axis and cap interval; contacts and general 3D Booleans unsupported"}),
    )
}

/// Execute one bounded request. Domain errors are structured JSON with ok=false;
/// the accepted document, shape, cache and undo/redo remain unchanged.
pub fn prism_workflow_session_command_json(
    session: &mut PrismWorkflowSession,
    input: &str,
) -> Result<String> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(Error::InvalidInput("prism workflow request exceeds 2 MiB"));
    }
    let command = match serde_json::from_str::<Command>(input) {
        Ok(command) => command,
        Err(error) => return Ok(json!({"ok":false,"diagnostic":{"category":"invalid_input","code":"invalid_request","operation_id":null,"message":error.to_string()},"accepted_document":session.accepted_document()}).to_string()),
    };
    let result = match command {
        Command::Rebuild { document } => session.rebuild_with(&document, body_report),
        Command::Undo => session.undo_with(body_report),
        Command::Redo => session.redo_with(body_report),
        Command::Reset => {
            session.reset();
            return Ok(json!({"ok":true,"reset":true,"document":null,"components":[],"component_steps":[],"cache":{"evaluated_nodes":0,"reused_nodes":0},"history":{"can_undo":false,"can_redo":false}}).to_string());
        }
    };
    match result {
        Ok((report, mut data)) => {
            data["cache"] = json!(report.stats);
            data["history"] =
                json!({"can_undo":session.undo_count()>0,"can_redo":session.redo_count()>0});
            Ok(data.to_string())
        }
        Err(error) => Ok(
            json!({"ok":false,"diagnostic":error,"accepted_document":session.accepted_document()})
                .to_string(),
        ),
    }
}
