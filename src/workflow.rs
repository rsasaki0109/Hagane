//! Versioned, deliberately scoped editable modeling intent, not B-rep interchange.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDocument {
    pub schema_version: u32,
    pub units: String,
    pub tolerance: WorkflowTolerance,
    pub operations: Vec<WorkflowOperation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTolerance {
    pub linear: f64,
    pub angular: f64,
    pub relative: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkflowOperation {
    Box {
        id: String,
        size: [f64; 3],
    },
    Bore {
        id: String,
        input: String,
        mode: WorkflowBoreMode,
        center: [f64; 2],
        radius: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        depth: Option<f64>,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBoreMode {
    Through,
    Blind,
}
#[derive(Clone, Debug, Serialize)]
pub struct WorkflowDiagnostic {
    pub category: &'static str,
    pub code: &'static str,
    pub operation_id: Option<String>,
    pub field: Option<&'static str>,
    pub message: String,
    pub suggestion: Option<String>,
    pub measured_clearance: Option<f64>,
    pub required_clearance: Option<f64>,
}
fn diagnostic(
    code: &'static str,
    id: Option<&str>,
    field: Option<&'static str>,
    message: impl Into<String>,
) -> Box<WorkflowDiagnostic> {
    Box::new(WorkflowDiagnostic {
        category: "invalid_input",
        code,
        operation_id: id.map(str::to_owned),
        field,
        message: message.into(),
        suggestion: None,
        measured_clearance: None,
        required_clearance: None,
    })
}
fn geometry_error(error: Error, id: &str) -> Box<WorkflowDiagnostic> {
    let mut d = diagnostic("geometry_rejected", Some(id), None, error.to_string());
    d.category = match error {
        Error::InvalidInput(_) => "invalid_input",
        Error::Unsupported(_) => "unsupported",
        _ => "validation_failed",
    };
    // The low-level API does not yet distinguish all numerical failure causes;
    // do not infer numerical certainty or a repair from its free-text message.
    d
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
impl WorkflowDocument {
    /// Rebuild exact supported operations. IDs identify operations, not persistent faces.
    pub fn rebuild(&self) -> std::result::Result<Solid, Box<WorkflowDiagnostic>> {
        if self.schema_version != 1 {
            let mut d = diagnostic(
                "unsupported_schema",
                None,
                Some("schema_version"),
                "Only operation-document schema version 1 is supported.",
            );
            d.category = "unsupported";
            return Err(d);
        }
        if self.units != "mm" {
            let mut d = diagnostic(
                "unsupported_units",
                None,
                Some("units"),
                "Only explicit millimetre documents are supported.",
            );
            d.category = "unsupported";
            return Err(d);
        }
        let policy = GeometryTolerance::new(
            self.tolerance.linear,
            self.tolerance.angular,
            self.tolerance.relative,
        )
        .map_err(|e| diagnostic("invalid_tolerance", None, Some("tolerance"), e.to_string()))?;
        let t = policy.absolute();
        if self.operations.is_empty() || self.operations.len() > 2 {
            let mut d = diagnostic(
                "unsupported_history",
                None,
                Some("operations"),
                "History must contain one box followed by at most one bore.",
            );
            d.category = "unsupported";
            return Err(d);
        }
        for op in &self.operations {
            let id = match op {
                WorkflowOperation::Box { id, .. } | WorkflowOperation::Bore { id, .. } => id,
            };
            if !valid_id(id) {
                return Err(diagnostic(
                    "invalid_operation_id",
                    Some(id),
                    Some("id"),
                    "Operation IDs require 1–64 ASCII letters, digits, underscores or hyphens.",
                ));
            }
        }
        let WorkflowOperation::Box { id: box_id, size } = &self.operations[0] else {
            return Err(diagnostic(
                "invalid_history",
                None,
                Some("operations"),
                "The first operation must create a box.",
            ));
        };
        let size = Vec3::new(size[0], size[1], size[2]);
        if !size.finite()
            || [size.x, size.y, size.z]
                .iter()
                .any(|v| *v <= 10. * t.linear)
        {
            let mut d = diagnostic(
                "invalid_box_size",
                Some(box_id),
                Some("size"),
                "Box dimensions must be finite and exceed ten linear tolerances.",
            );
            d.suggestion = Some("Enter positive, resolved width, length and height.".into());
            return Err(d);
        }
        let b = BoxSpec {
            min: size * (-0.5),
            size,
        };
        let base = make_box(b, t).map_err(|e| geometry_error(e, box_id))?;
        if self.operations.len() == 1 {
            return Ok(base);
        }
        let WorkflowOperation::Bore {
            id,
            input,
            mode,
            center,
            radius,
            depth,
        } = &self.operations[1]
        else {
            return Err(diagnostic(
                "unsupported_history",
                None,
                Some("operations"),
                "Only a bore may follow the box in version 1.",
            ));
        };
        if id == box_id || input != box_id {
            return Err(diagnostic(
                "invalid_reference",
                Some(id),
                Some("input"),
                "Bore ID must be unique and input must reference the preceding box.",
            ));
        }
        if center.iter().any(|v| !v.is_finite()) || !radius.is_finite() || *radius <= 10. * t.linear
        {
            let mut d = diagnostic(
                "invalid_bore_parameters",
                Some(id),
                Some("radius_or_center"),
                "Radius and center must be finite; radius must exceed ten linear tolerances.",
            );
            d.suggestion = Some("Enter a positive, resolved radius and finite XY center.".into());
            return Err(d);
        }
        let clearance = (size.x / 2. - center[0].abs()).min(size.y / 2. - center[1].abs()) - radius;
        let required = 10. * t.linear;
        if !clearance.is_finite() {
            let mut d = diagnostic(
                "finite_clearance",
                Some(id),
                Some("center_or_radius"),
                "Clearance cannot be represented with finite binary64 arithmetic.",
            );
            d.category = "numerically_unresolved";
            return Err(d);
        }
        if clearance <= required {
            let mut d = diagnostic(
                "side_clearance",
                Some(id),
                Some("center_or_radius"),
                "The circular tool reaches or nearly touches a box side.",
            );
            d.category = "unsupported";
            d.measured_clearance = Some(clearance);
            d.required_clearance = Some(required);
            d.suggestion=Some("Reduce the radius, move the center inward, or enlarge the box until clearance exceeds the required margin.".into());
            return Err(d);
        }
        match mode {
            WorkflowBoreMode::Through => {
                if depth.is_some() {
                    return Err(diagnostic(
                        "unexpected_depth",
                        Some(id),
                        Some("depth"),
                        "Through bores must omit depth; it is not silently ignored.",
                    ));
                }
                subtract_through_cylinder(
                    b,
                    CylinderSpec {
                        base: Point3::new(center[0], center[1], -size.z),
                        radius: *radius,
                        height: 2. * size.z,
                    },
                    t,
                )
                .map_err(|e| geometry_error(e, id))
            }
            WorkflowBoreMode::Blind => {
                let depth = depth
                    .filter(|v| v.is_finite() && *v > required)
                    .ok_or_else(|| {
                        diagnostic(
                            "invalid_depth",
                            Some(id),
                            Some("depth"),
                            "Blind bores require finite depth greater than ten linear tolerances.",
                        )
                    })?;
                let floor = size.z - depth;
                if floor <= required {
                    let mut d = diagnostic(
                        "floor_thickness",
                        Some(id),
                        Some("depth"),
                        "The requested blind depth breaks through or leaves an unresolved floor.",
                    );
                    d.category = "unsupported";
                    d.measured_clearance = Some(floor);
                    d.required_clearance = Some(required);
                    d.suggestion=Some("Reduce depth or increase box height so the remaining floor exceeds the required margin.".into());
                    return Err(d);
                }
                subtract_blind_cylinder(
                    b,
                    CylinderSpec {
                        base: Point3::new(center[0], center[1], size.z / 2. - depth),
                        radius: *radius,
                        height: depth + size.z,
                    },
                    t,
                )
                .map_err(|e| geometry_error(e, id))
            }
        }
    }
    fn candidate_segments(&self) -> Vec<[[f64; 3]; 2]> {
        let [WorkflowOperation::Box { size, .. }, WorkflowOperation::Bore {
            center,
            radius,
            depth,
            mode,
            ..
        }] = self.operations.as_slice()
        else {
            return vec![];
        };
        if size
            .iter()
            .chain(center.iter())
            .any(|v| !v.is_finite() || v.abs() > 1e6)
            || !radius.is_finite()
            || *radius <= 0.
            || *radius > 1e6
        {
            return vec![];
        }
        let top = size[2] / 2.;
        let bottom = if *mode == WorkflowBoreMode::Blind {
            top - depth.unwrap_or(size[2])
        } else {
            -top
        };
        if !bottom.is_finite() || bottom.abs() > 1e6 {
            return vec![];
        }
        let point = |z: f64, i: usize| {
            let a = std::f64::consts::TAU * i as f64 / 48.;
            [
                center[0] + radius * a.cos(),
                center[1] + radius * a.sin(),
                z,
            ]
        };
        let mut segments = vec![];
        for z in [bottom, top] {
            for i in 0..48 {
                segments.push([point(z, i), point(z, i + 1)]);
            }
        }
        for i in [0, 12, 24, 36] {
            segments.push([point(bottom, i), point(top, i)]);
        }
        segments
    }
}
/// Bounded UTF-8 JSON input. `ok` is authoritative; processing is not model success.
/// Failed reports contain no replacement mesh; the UI may retain a labeled prior result.
pub fn evaluate_workflow_json(input: &str) -> Result<String> {
    if input.len() > 65536 {
        return workflow_input_failure("Operation document exceeds 64 KiB.");
    }
    let document: WorkflowDocument = match serde_json::from_str(input) {
        Ok(d) => d,
        Err(e) => return workflow_input_failure(&format!("Invalid operation document: {e}")),
    };
    let report = match document.rebuild() {
        Ok(solid) => match solid
            .mesh_json(0.05, Tolerance::new(document.tolerance.linear)?)
            .and_then(|text| {
                serde_json::from_str::<Value>(&text).map_err(|_| {
                    Error::Unsupported(
                        "display metrics or coordinates exceed finite JSON representation",
                    )
                })
            }) {
            Ok(mesh) => json!({"ok":true,"document":document,"mesh":mesh}),
            Err(error) => {
                let mut d = geometry_error(error, "display");
                d.operation_id = None;
                d.field = Some("display");
                d.code = "display_rejected";
                json!({"ok":false,"diagnostic":d,"candidate_segments":document.candidate_segments(),"attempted_document":document})
            }
        },
        Err(d) => {
            json!({"ok":false,"diagnostic":d,"candidate_segments":document.candidate_segments(),"attempted_document":document})
        }
    };
    serde_json::to_string(&report).map_err(|_| Error::InvalidInput("workflow serialization failed"))
}
/// Transport/document failure, distinct from a rejected geometric operation.
pub fn workflow_input_failure(message: &str) -> Result<String> {
    let report = json!({"ok":false,"diagnostic":diagnostic("invalid_document",None,None,message),"candidate_segments":[]});
    serde_json::to_string(&report).map_err(|_| Error::InvalidInput("workflow serialization failed"))
}
