//! JSON transport for the versioned multi-component rational-frustum workflow.
use crate::*;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Rebuild {
        document: NurbsFrustumWorkflowDocument,
    },
    Undo {},
    Redo {},
    Reset {},
}

pub fn nurbs_frustum_workflow_example_document() -> Result<NurbsFrustumWorkflowDocument> {
    serde_json::from_value(json!({
        "schema_version":1,"document_type":"nurbs_frustum_workflow","units":"mm",
        "tolerance":{"linear":1e-8,"angular":1e-10,"relative":1e-10},
        "display_chord_tolerance":0.1,"output":"selected",
        "operations":[
            {"kind":"frustum","id":"stock","radii":[16.,8.],"height":24.,"origin":[12.,-5.,8.],"axes":[[0.,1.,0.],[0.,0.,1.],[1.,0.,0.]]},
            {"kind":"partition","id":"parts","input":"stock","cuts":[6.,12.,18.]},
            {"kind":"select","id":"selected","input":"parts","component":1}
        ]
    })).map_err(|_|Error::InvalidInput("invalid bundled frustum workflow document"))
}

fn body_report(
    shape: &NurbsFrustumWorkflowShape,
    document: &NurbsFrustumWorkflowDocument,
) -> Result<Value> {
    let tolerance = GeometryTolerance::new(
        document.tolerance.linear,
        document.tolerance.angular,
        document.tolerance.relative,
    )?;
    let mut components = Vec::with_capacity(shape.components().len());
    let mut triangle_count = 0usize;
    for body in shape.components() {
        let mut data = crate::nurbs_frustum_demo::serialize_frustum(
            body,
            tolerance,
            document.display_chord_tolerance,
            0.,
        )?;
        let count = data["mesh"]["triangles"]
            .as_array()
            .ok_or(Error::InvalidInput("frustum mesh report has no triangles"))?
            .len();
        triangle_count = triangle_count.checked_add(count).ok_or(Error::Unsupported(
            "frustum aggregate display triangle count overflow",
        ))?;
        if triangle_count
            > crate::nurbs_frustum_workflow::NURBS_FRUSTUM_WORKFLOW_MAX_DISPLAY_TRIANGLES
        {
            return Err(Error::Unsupported(
                "frustum aggregate display triangle budget exceeded",
            ));
        }
        let frame = body.frame();
        let xyz = |v: Vec3| [v.x, v.y, v.z];
        data["placement"] = json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)});
        data["scope"] = json!("actual typed rational frustum workflow component; exact axial partition, explicit component selection; arbitrary lofts and general Booleans unsupported");
        components.push(data);
    }
    let steps: Result<Vec<_>> = shape
        .components()
        .iter()
        .map(|body| body.export_step_mm(tolerance))
        .collect();
    let mut volume = 0.;
    let mut correction = 0.;
    for body in shape.components() {
        let adjusted = body.volume(tolerance)? - correction;
        let next = volume + adjusted;
        correction = (next - volume) - adjusted;
        volume = next;
    }
    if !volume.is_finite() {
        return Err(Error::Unsupported(
            "frustum component total volume is unresolved",
        ));
    }
    Ok(
        json!({"ok":true,"document":document,"components":components,"component_steps":steps?,
        "output":{"id":document.output,"component_count":shape.components().len(),"total_volume":volume,"status":if shape.components().is_empty(){"empty"}else if shape.components().len()==1{"single"}else{"multiple"}},
        "units":"mm","step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "scope":"versioned typed rational frustum graph; exact axial partitions and explicit component selection; output reports all actual components with separate STEP exports; arbitrary lofts and general Booleans unsupported"}),
    )
}

/// Execute one bounded request. Domain errors are structured JSON with ok=false;
/// the accepted document, shape, cache and undo/redo remain unchanged.
pub fn nurbs_frustum_workflow_session_command_json(
    session: &mut NurbsFrustumWorkflowSession,
    input: &str,
) -> Result<String> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(Error::InvalidInput(
            "frustum workflow request exceeds 2 MiB",
        ));
    }
    let command = match serde_json::from_str::<Command>(input) {
        Ok(command) => command,
        Err(error) => return Ok(json!({"ok":false,"diagnostic":{"category":"invalid_input","code":"invalid_request","operation_id":null,"message":error.to_string()},"accepted_document":session.accepted_document()}).to_string()),
    };
    let result = match command {
        Command::Rebuild { document } => session.rebuild_with(&document, body_report),
        Command::Undo {} => session.undo_with(body_report),
        Command::Redo {} => session.redo_with(body_report),
        Command::Reset {} => {
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
