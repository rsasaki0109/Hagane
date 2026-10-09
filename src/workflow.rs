//! Versioned, deliberately scoped editable modeling intent, not B-rep interchange.
use crate::operations::{
    apply_checked_prism_bore, checked_box_bore_tools, checked_polygon_prism_stock,
    polygon_bore_clearance,
};
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDocument {
    pub schema_version: u32,
    pub units: String,
    pub tolerance: WorkflowTolerance,
    pub operations: Vec<WorkflowOperation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTolerance {
    pub linear: f64,
    pub angular: f64,
    pub relative: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkflowOperation {
    Extrusion {
        id: String,
        outer: Vec<[f64; 2]>,
        height: f64,
    },
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
        let plan = self.checked_plan()?;
        let mut solid = plan
            .make_stock()
            .map_err(|e| geometry_error(e, self.operation_id(0)))?;
        for (i, (&bore, &tool)) in plan.bores.iter().zip(&plan.tools).enumerate() {
            apply_checked_prism_bore(&mut solid, plan.stock, bore, tool, plan.tolerance)
                .map_err(|e| geometry_error(e, self.operation_id(i + 1)))?;
        }
        solid
            .validate(plan.tolerance)
            .map_err(|e| geometry_error(e, self.operation_id(self.operations.len() - 1)))?;
        Ok(solid)
    }
    fn operation_id(&self, index: usize) -> &str {
        match &self.operations[index] {
            WorkflowOperation::Box { id, .. }
            | WorkflowOperation::Extrusion { id, .. }
            | WorkflowOperation::Bore { id, .. } => id,
        }
    }
    fn checked_plan(&self) -> std::result::Result<WorkflowPlan, Box<WorkflowDiagnostic>> {
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
        if self.operations.is_empty() || self.operations.len() > 257 {
            let mut d = diagnostic(
                "unsupported_history",
                None,
                Some("operations"),
                "History must contain one box or polygon extrusion followed by at most 256 chained bores.",
            );
            d.category = "unsupported";
            return Err(d);
        }
        for op in &self.operations {
            let id = match op {
                WorkflowOperation::Box { id, .. }
                | WorkflowOperation::Extrusion { id, .. }
                | WorkflowOperation::Bore { id, .. } => id,
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
        let (box_id, b, profile) = match &self.operations[0] {
            WorkflowOperation::Box { id, size } => {
                let size = Vec3::new(size[0], size[1], size[2]);
                if !size.finite()
                    || [size.x, size.y, size.z]
                        .iter()
                        .any(|v| *v <= 10. * t.linear)
                {
                    let mut d = diagnostic(
                        "invalid_box_size",
                        Some(id),
                        Some("size"),
                        "Box dimensions must be finite and exceed ten linear tolerances.",
                    );
                    d.suggestion =
                        Some("Enter positive, resolved width, length and height.".into());
                    return Err(d);
                }
                (
                    id,
                    BoxSpec {
                        min: size * (-0.5),
                        size,
                    },
                    None,
                )
            }
            WorkflowOperation::Extrusion { id, outer, height } => {
                let b = checked_polygon_prism_stock(outer, *height, t).map_err(|e| {
                    let mut d = geometry_error(e, id);
                    d.code = "profile_rejected";
                    d.field = Some("outer_or_height");
                    d
                })?;
                (
                    id,
                    b,
                    Some(PolygonProfile {
                        origin: Point3::new(0., 0., -*height / 2.),
                        outer: outer.clone(),
                        holes: vec![],
                    }),
                )
            }
            _ => {
                return Err(diagnostic(
                    "invalid_history",
                    None,
                    Some("operations"),
                    "The first operation must create a box or polygon extrusion.",
                ))
            }
        };
        let size = b.size;
        let mut bores: Vec<BoxBore> = Vec::new();
        let mut ids = vec![box_id.as_str()];
        for operation in &self.operations[1..] {
            let WorkflowOperation::Bore {
                id,
                input,
                mode,
                center,
                radius,
                depth,
            } = operation
            else {
                return Err(diagnostic(
                    "unsupported_history",
                    None,
                    Some("operations"),
                    "Only bores may follow the initial stock operation.",
                ));
            };
            if ids.contains(&id.as_str()) || input != ids.last().unwrap() {
                return Err(diagnostic(
                "invalid_reference",
                Some(id),
                Some("input"),
                "Bore ID must be unique and input must reference the immediately preceding operation.",
            ));
            }
            if center.iter().any(|v| !v.is_finite())
                || !radius.is_finite()
                || *radius <= 10. * t.linear
            {
                let mut d = diagnostic(
                    "invalid_bore_parameters",
                    Some(id),
                    Some("radius_or_center"),
                    "Radius and center must be finite; radius must exceed ten linear tolerances.",
                );
                d.suggestion =
                    Some("Enter a positive, resolved radius and finite XY center.".into());
                return Err(d);
            }
            let clearance = if let Some(profile) = &profile {
                polygon_bore_clearance(&profile.outer, *center, *radius)
                    .map_err(|e| geometry_error(e, id))?
            } else {
                (size.x / 2. - center[0].abs()).min(size.y / 2. - center[1].abs()) - radius
            };
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
                    "The circular tool lies outside, reaches or nearly touches a stock boundary.",
                );
                d.category = "unsupported";
                d.measured_clearance = Some(clearance);
                d.required_clearance = Some(required);
                d.suggestion=Some("Reduce the radius, move the center inward, or enlarge the stock until clearance exceeds the required margin.".into());
                return Err(d);
            }
            for (previous, previous_id) in bores.iter().zip(&ids[1..]) {
                let gap = (center[0] - previous.center[0]).hypot(center[1] - previous.center[1])
                    - radius
                    - previous.radius;
                if !gap.is_finite() || gap <= required {
                    let mut d = diagnostic(
                        "bore_clearance",
                        Some(id),
                        Some("center_or_radius"),
                        format!(
                            "Tool overlaps, touches or nearly touches operation {previous_id}."
                        ),
                    );
                    d.category = if gap.is_finite() {
                        "unsupported"
                    } else {
                        "numerically_unresolved"
                    };
                    d.measured_clearance = gap.is_finite().then_some(gap);
                    d.required_clearance = Some(required);
                    d.suggestion = Some(
                        "Move the center or reduce radii so the XY footprints are separated."
                            .into(),
                    );
                    return Err(d);
                }
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
                    bores.push(BoxBore {
                        center: *center,
                        radius: *radius,
                        depth: None,
                    });
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
                    bores.push(BoxBore {
                        center: *center,
                        radius: *radius,
                        depth: Some(depth),
                    });
                }
            }
            ids.push(id.as_str());
        }
        let tools = checked_box_bore_tools(b, &bores, t)
            .map_err(|e| geometry_error(e, ids.last().unwrap()))?;
        Ok(WorkflowPlan {
            stock: b,
            profile,
            bores,
            tools,
            tolerance: t,
        })
    }
    fn candidate_segments(&self, failed: Option<&str>) -> Vec<[[f64; 3]; 2]> {
        let height = match self.operations.first() {
            Some(WorkflowOperation::Box { size, .. }) => size[2],
            Some(WorkflowOperation::Extrusion { height, .. }) => *height,
            _ => return vec![],
        };
        let size = [height; 3];
        let Some(WorkflowOperation::Bore {
            center,
            radius,
            depth,
            mode,
            ..
        }) = self
            .operations
            .iter()
            .find(|op| matches!(op,WorkflowOperation::Bore{id,..} if Some(id.as_str())==failed))
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
        Ok(solid) => match workflow_mesh(&solid, &document) {
            Ok(mesh) => json!({"ok":true,"document":document,"mesh":mesh}),
            Err(error) => display_failure(error, &document),
        },
        Err(d) => {
            json!({"ok":false,"diagnostic":d,"candidate_segments":document.candidate_segments(d.operation_id.as_deref()),"attempted_document":document})
        }
    };
    serde_json::to_string(&report).map_err(|_| Error::InvalidInput("workflow serialization failed"))
}
/// Transport/document failure, distinct from a rejected geometric operation.
pub fn workflow_input_failure(message: &str) -> Result<String> {
    let report = json!({"ok":false,"diagnostic":diagnostic("invalid_document",None,None,message),"candidate_segments":[]});
    serde_json::to_string(&report).map_err(|_| Error::InvalidInput("workflow serialization failed"))
}

struct WorkflowPlan {
    stock: BoxSpec,
    profile: Option<PolygonProfile>,
    bores: Vec<BoxBore>,
    tools: Vec<CylinderSpec>,
    tolerance: Tolerance,
}
impl WorkflowPlan {
    fn make_stock(&self) -> Result<Solid> {
        if let Some(profile) = &self.profile {
            extrude_polygon(
                profile,
                Vec3::new(0., 0., self.stock.size.z),
                self.tolerance,
            )
        } else {
            make_box(self.stock, self.tolerance)
        }
    }
}
/// Actual B-rep operation evaluations, not timing or mesh-cache statistics.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct WorkflowRebuildStats {
    pub reused_operations: usize,
    pub rebuilt_operations: usize,
    pub rebuilt_operation_ids: Vec<String>,
}
/// An immutable exact solid shared with the session's accepted prefix snapshots.
#[derive(Clone, Debug)]
pub struct WorkflowRebuild {
    pub solid: Arc<Solid>,
    pub stats: WorkflowRebuildStats,
}
/// In-memory incremental evaluator for a linear stock-and-bore document.
/// Failed edits never replace the last accepted snapshots. All input checks
/// still run; only geometry construction for unchanged prefixes is skipped.
#[derive(Default)]
pub struct WorkflowSession {
    accepted: Option<WorkflowDocument>,
    snapshots: Vec<Arc<Solid>>,
}
impl WorkflowSession {
    /// Start an empty evaluator with no accepted history.
    pub fn new() -> Self {
        Self::default()
    }
    /// Drop all geometry snapshots; the next valid edit starts from the box.
    pub fn reset(&mut self) {
        self.accepted = None;
        self.snapshots.clear();
    }
    fn prepare(
        &self,
        document: &WorkflowDocument,
    ) -> std::result::Result<(WorkflowRebuild, Vec<Arc<Solid>>), Box<WorkflowDiagnostic>> {
        let plan = document.checked_plan()?;
        let reused = self
            .accepted
            .as_ref()
            .filter(|old| {
                old.schema_version == document.schema_version
                    && old.units == document.units
                    && old.tolerance == document.tolerance
            })
            .map(|old| {
                old.operations
                    .iter()
                    .zip(&document.operations)
                    .take_while(|(a, b)| a == b)
                    .count()
            })
            .unwrap_or(0);
        let stats = WorkflowRebuildStats {
            reused_operations: reused,
            rebuilt_operations: document.operations.len() - reused,
            rebuilt_operation_ids: (reused..document.operations.len())
                .map(|i| document.operation_id(i).to_owned())
                .collect(),
        };
        let mut snapshots = self.snapshots[..reused].to_vec();
        if reused == document.operations.len() {
            let result = WorkflowRebuild {
                solid: Arc::clone(snapshots.last().unwrap()),
                stats,
            };
            return Ok((result, snapshots));
        }
        let mut solid = if reused == 0 {
            let solid = plan
                .make_stock()
                .map_err(|e| geometry_error(e, document.operation_id(0)))?;
            snapshots.push(Arc::new(solid.clone()));
            solid
        } else {
            (*snapshots[reused - 1]).clone()
        };
        for index in reused.max(1)..document.operations.len() {
            apply_checked_prism_bore(
                &mut solid,
                plan.stock,
                plan.bores[index - 1],
                plan.tools[index - 1],
                plan.tolerance,
            )
            .and_then(|()| solid.validate(plan.tolerance))
            .map_err(|e| geometry_error(e, document.operation_id(index)))?;
            snapshots.push(Arc::new(solid.clone()));
        }
        let result = WorkflowRebuild {
            solid: Arc::clone(snapshots.last().unwrap()),
            stats,
        };
        Ok((result, snapshots))
    }
    /// Rebuild only the changed suffix and commit a fully validated result.
    pub fn rebuild(
        &mut self,
        document: &WorkflowDocument,
    ) -> std::result::Result<WorkflowRebuild, Box<WorkflowDiagnostic>> {
        let (result, snapshots) = self.prepare(document)?;
        self.snapshots = snapshots;
        self.accepted = Some(document.clone());
        Ok(result)
    }
    /// Incremental JSON report, adding `rebuild` statistics on success.
    /// Parsing, geometry and display failures all leave the accepted cache intact.
    pub fn evaluate_json(&mut self, input: &str) -> Result<String> {
        if input.len() > 65536 {
            return workflow_input_failure("Operation document exceeds 64 KiB.");
        }
        let document: WorkflowDocument = match serde_json::from_str(input) {
            Ok(document) => document,
            Err(error) => {
                return workflow_input_failure(&format!("Invalid operation document: {error}"))
            }
        };
        let report = match self.prepare(&document) {
            Ok((result, snapshots)) => match workflow_mesh(&result.solid, &document) {
                Ok(mesh) => {
                    let report =
                        json!({"ok":true,"document":document,"mesh":mesh,"rebuild":result.stats});
                    self.accepted = Some(document);
                    self.snapshots = snapshots;
                    report
                }
                Err(error) => display_failure(error, &document),
            },
            Err(d) => {
                json!({"ok":false,"diagnostic":d,"candidate_segments":document.candidate_segments(d.operation_id.as_deref()),"attempted_document":document})
            }
        };
        serde_json::to_string(&report)
            .map_err(|_| Error::InvalidInput("workflow serialization failed"))
    }
}
fn workflow_mesh(solid: &Solid, document: &WorkflowDocument) -> Result<Value> {
    solid
        .mesh_json(0.05, Tolerance::new(document.tolerance.linear)?)
        .and_then(|text| {
            serde_json::from_str(&text).map_err(|_| {
                Error::Unsupported(
                    "display metrics or coordinates exceed finite JSON representation",
                )
            })
        })
}
fn display_failure(error: Error, document: &WorkflowDocument) -> Value {
    let mut d = geometry_error(error, "display");
    d.operation_id = None;
    d.field = Some("display");
    d.code = "display_rejected";
    json!({"ok":false,"diagnostic":d,"candidate_segments":[],"attempted_document":document})
}
