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
    Line {
        component: usize,
        origin: [f64; 3],
        direction: [f64; 3],
    },
    Ray {
        component: usize,
        origin: [f64; 3],
        direction: [f64; 3],
    },
    Segment {
        component: usize,
        start: [f64; 3],
        end: [f64; 3],
    },
    Query {
        component: usize,
        point: [f64; 3],
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
        let mut data = match body {
            NurbsFrustumWorkflowComponent::Canonical(canonical) => {
                crate::nurbs_frustum_demo::serialize_frustum(
                    canonical,
                    tolerance,
                    document.display_chord_tolerance,
                    0.,
                )?
            }
            NurbsFrustumWorkflowComponent::Oblique(oblique) => {
                crate::nurbs_frustum_plane_split_demo::part_report(
                    oblique,
                    tolerance,
                    document.display_chord_tolerance,
                )?
            }
            NurbsFrustumWorkflowComponent::Bored(bored) => {
                crate::nurbs_frustum_bore_demo::serialize_bored_frustum(
                    bored,
                    tolerance,
                    document.display_chord_tolerance,
                )?
            }
            NurbsFrustumWorkflowComponent::BoredOblique(body) => {
                crate::nurbs_frustum_bored_oblique_demo::serialize_bored_oblique_frustum(
                    body,
                    tolerance,
                    document.display_chord_tolerance,
                )?
            }
        };
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
        let frame = match body {
            NurbsFrustumWorkflowComponent::Canonical(canonical) => canonical.frame(),
            NurbsFrustumWorkflowComponent::Oblique(oblique) => oblique.source().frame(),
            NurbsFrustumWorkflowComponent::Bored(bored) => bored.source().frame(),
            NurbsFrustumWorkflowComponent::BoredOblique(body) => body.source().source().frame(),
        };
        let xyz = |v: Vec3| [v.x, v.y, v.z];
        data["placement"] = json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)});
        match body {
            NurbsFrustumWorkflowComponent::Canonical(_) => {
                data["scope"] = json!("actual typed rational frustum workflow component; exact axial partition, explicit component selection; arbitrary lofts and general Booleans unsupported");
            }
            NurbsFrustumWorkflowComponent::Bored(_) => {
                data["component_kind"] = json!("bored_frustum");
                data["placement_reference"] = json!("source frame");
            }
            NurbsFrustumWorkflowComponent::Oblique(_) => {
                data["component_kind"] = json!("oblique_frustum");
                data["placement_reference"] = json!("source frame");
            }
            NurbsFrustumWorkflowComponent::BoredOblique(_) => {
                data["component_kind"] = json!("bored_oblique_frustum");
                data["placement_reference"] = json!("source frame");
            }
        }
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
    let scope = if shape
        .components()
        .iter()
        .any(|body| matches!(body, NurbsFrustumWorkflowComponent::BoredOblique(_)))
    {
        "versioned typed rational frustum graph; actual closed bored oblique children; optional analytic mass properties; point/segment/line/ray queries unsupported; separate actual STEP exports"
    } else if shape
        .components()
        .iter()
        .any(|body| matches!(body, NurbsFrustumWorkflowComponent::Bored(_)))
    {
        "versioned typed rational frustum graph; actual closed coaxial through bores and explicit component selection; output reports all actual components with separate STEP exports; off-axis bores, blind cavities and general Booleans unsupported"
    } else if shape
        .components()
        .iter()
        .any(|body| body.oblique().is_some())
    {
        "versioned typed rational frustum graph; actual closed oblique splits and explicit component selection; output reports all actual components with separate STEP exports; arbitrary lofts and general Booleans unsupported"
    } else {
        "versioned typed rational frustum graph; exact axial partitions and explicit component selection; output reports all actual components with separate STEP exports; arbitrary lofts and general Booleans unsupported"
    };
    Ok(
        json!({"ok":true,"document":document,"components":components,"component_steps":steps?,
        "output":{"id":document.output,"component_count":shape.components().len(),"total_volume":volume,"status":if shape.components().is_empty(){"empty"}else if shape.components().len()==1{"single"}else{"multiple"}},
        "units":"mm","step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "scope":scope}),
    )
}

fn segment_report(
    session: &NurbsFrustumWorkflowSession,
    component: usize,
    start: [f64; 3],
    end: [f64; 3],
) -> std::result::Result<Value, Box<WorkflowDiagnostic>> {
    let diagnostic = |category, code, id: Option<&str>, message: String| {
        Box::new(WorkflowDiagnostic {
            category,
            code,
            operation_id: id.map(str::to_owned),
            field: None,
            message,
            suggestion: None,
            measured_clearance: None,
            required_clearance: None,
        })
    };
    let doc = session.accepted_document().ok_or_else(|| {
        diagnostic(
            "invalid_input",
            "no_accepted_document",
            None,
            "Accept a frustum workflow before querying a segment.".into(),
        )
    })?;
    let shape = session.accepted_shape().ok_or_else(|| {
        diagnostic(
            "validation_failed",
            "output_unavailable",
            Some(&doc.output),
            "Accepted output snapshot is unavailable.".into(),
        )
    })?;
    let body = shape.components().get(component).ok_or_else(|| {
        diagnostic(
            "invalid_input",
            "invalid_component",
            Some(&doc.output),
            "Segment component index is unavailable in the accepted output.".into(),
        )
    })?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let point = |v: [f64; 3]| Vec3::new(v[0], v[1], v[2]);
    let policy = doc.geometry_tol()?;
    let result: Result<_> = (|| {
        if start.iter().chain(end.iter()).any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput("Segment endpoints must be finite."));
        }
        match body {
            NurbsFrustumWorkflowComponent::Canonical(v) => {
                let result = v.intersect_segment(point(start), point(end), policy)?;
                Ok((
                    result.hits,
                    result.material_interval.into_iter().collect::<Vec<_>>(),
                ))
            }
            NurbsFrustumWorkflowComponent::Bored(v) => {
                let result = v.intersect_segment(point(start), point(end), policy)?;
                Ok((result.hits, result.material_intervals))
            }
            NurbsFrustumWorkflowComponent::Oblique(v) => {
                let result = v.intersect_segment(point(start), point(end), policy)?;
                Ok((
                    result.hits,
                    result.material_interval.into_iter().collect::<Vec<_>>(),
                ))
            }
            NurbsFrustumWorkflowComponent::BoredOblique(_) => Err(Error::Unsupported(
                "bored oblique frustum finite and infinite queries are not implemented",
            )),
        }
    })();
    let (hits, intervals) = result.map_err(|error| {
        diagnostic(
            match error {
                Error::InvalidInput(_) => "invalid_input",
                Error::Unsupported(_) => "unsupported",
                _ => "validation_failed",
            },
            "geometry_rejected",
            Some(&doc.output),
            error.to_string(),
        )
    })?;
    let hits:Vec<_>=hits.into_iter().map(|hit|json!({"parameter":hit.parameter,"point":xyz(hit.point),"faces":hit.faces.into_iter().map(|face|json!({"face_id":face.face_id,"uv":face.uv})).collect::<Vec<_>>()})).collect();
    Ok(
        json!({"ok":true,"units":"mm","segment":{"component":component,"start":start,"end":end,"hits":hits,"material_intervals":intervals,"mesh_free":true},"scope":"read-only finite segment intersection with the actual accepted workflow component; no modeling cut or geometry, display, export, cache or history mutation"}),
    )
}

fn line_report(
    session: &NurbsFrustumWorkflowSession,
    component: usize,
    origin: [f64; 3],
    direction: [f64; 3],
    ray: bool,
) -> std::result::Result<Value, Box<WorkflowDiagnostic>> {
    let diagnostic = |category, code, id: Option<&str>, message: String| {
        Box::new(WorkflowDiagnostic {
            category,
            code,
            operation_id: id.map(str::to_owned),
            field: None,
            message,
            suggestion: None,
            measured_clearance: None,
            required_clearance: None,
        })
    };
    let doc = session.accepted_document().ok_or_else(|| {
        diagnostic(
            "invalid_input",
            "no_accepted_document",
            None,
            "Accept a frustum workflow before querying a line.".into(),
        )
    })?;
    let shape = session.accepted_shape().ok_or_else(|| {
        diagnostic(
            "validation_failed",
            "output_unavailable",
            Some(&doc.output),
            "Accepted output snapshot is unavailable.".into(),
        )
    })?;
    let body = shape.components().get(component).ok_or_else(|| {
        diagnostic(
            "invalid_input",
            "invalid_component",
            Some(&doc.output),
            "Line component index is unavailable in the accepted output.".into(),
        )
    })?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let point = |v: [f64; 3]| Vec3::new(v[0], v[1], v[2]);
    let policy = doc.geometry_tol()?;
    let result: Result<_> = (|| {
        if origin
            .iter()
            .chain(direction.iter())
            .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidInput(
                "Line origin and direction must be finite.",
            ));
        }
        match body {
            NurbsFrustumWorkflowComponent::Canonical(v) => {
                let result = if ray {
                    v.intersect_ray(point(origin), point(direction), policy)?
                } else {
                    v.intersect_line(point(origin), point(direction), policy)?
                };
                Ok((
                    result.hits,
                    result.material_interval.into_iter().collect::<Vec<_>>(),
                ))
            }
            NurbsFrustumWorkflowComponent::Bored(v) => {
                let result = if ray {
                    v.intersect_ray(point(origin), point(direction), policy)?
                } else {
                    v.intersect_line(point(origin), point(direction), policy)?
                };
                Ok((result.hits, result.material_intervals))
            }
            NurbsFrustumWorkflowComponent::Oblique(v) => {
                let result = if ray {
                    v.intersect_ray(point(origin), point(direction), policy)?
                } else {
                    v.intersect_line(point(origin), point(direction), policy)?
                };
                Ok((result.hits, result.material_intervals))
            }
            NurbsFrustumWorkflowComponent::BoredOblique(_) => Err(Error::Unsupported(
                "bored oblique frustum finite and infinite queries are not implemented",
            )),
        }
    })();
    let (hits, intervals) = result.map_err(|error| {
        diagnostic(
            match error {
                Error::InvalidInput(_) => "invalid_input",
                Error::Unsupported(_) => "unsupported",
                _ => "validation_failed",
            },
            "geometry_rejected",
            Some(&doc.output),
            error.to_string(),
        )
    })?;
    let hits:Vec<_>=hits.into_iter().map(|hit|json!({"parameter":hit.parameter,"point":xyz(hit.point),"faces":hit.faces.into_iter().map(|face|json!({"face_id":face.face_id,"uv":face.uv})).collect::<Vec<_>>()})).collect();
    Ok(
        json!({"ok":true,"units":"mm","line":{"component":component,"origin":origin,"direction":direction,"kind":if ray {"ray"}else{"line"},"hits":hits,"material_intervals":intervals,"mesh_free":true},"scope":"read-only line or ray intersection with the actual accepted workflow component; no modeling cut or geometry, display, export, cache or history mutation"}),
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
        Command::Line {component,origin,direction} => return Ok(match line_report(session,component,origin,direction,false) {Ok(value)=>value.to_string(),Err(error)=>json!({"ok":false,"diagnostic":error,"accepted_document":session.accepted_document()}).to_string()}),
        Command::Ray {component,origin,direction} => return Ok(match line_report(session,component,origin,direction,true) {Ok(value)=>value.to_string(),Err(error)=>json!({"ok":false,"diagnostic":error,"accepted_document":session.accepted_document()}).to_string()}),
        Command::Segment { component, start, end } => return Ok(match segment_report(session,component,start,end) {Ok(value)=>value.to_string(),Err(error)=>json!({"ok":false,"diagnostic":error,"accepted_document":session.accepted_document()}).to_string()}),
        Command::Query { component, point } => {
            return Ok(match session.classify_point(component, Vec3::new(point[0], point[1], point[2])) {
                Ok(location) => {
                    let location = match location { PointLocation::Inside => "inside", PointLocation::Outside => "outside", PointLocation::Boundary => "boundary" };
                    json!({"ok":true,"units":"mm","query":{"component":component,"point":point,"location":location,"mesh_free":true},"scope":"read-only classification of the actual accepted workflow output component; no geometry, display, export, cache or history mutation"}).to_string()
                }
                Err(error) => json!({"ok":false,"diagnostic":error,"accepted_document":session.accepted_document()}).to_string(),
            });
        }
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
