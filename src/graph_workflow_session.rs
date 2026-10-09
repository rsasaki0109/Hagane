//! Transactional incremental evaluation of retained graph B-rep histories.
use crate::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct GraphWorkflowRebuild {
    pub shape: Arc<GraphWorkflowShape>,
    pub stats: WorkflowRebuildStats,
}

/// An unchanged prefix shares immutable typed geometry. Failed candidate geometry,
/// display or serialization never replaces the last accepted document or cache.
#[derive(Default)]
pub struct GraphWorkflowSession {
    accepted: Option<GraphWorkflowDocument>,
    snapshots: Vec<Arc<GraphWorkflowShape>>,
}

fn diagnostic(code: &'static str, message: impl Into<String>) -> Box<WorkflowDiagnostic> {
    Box::new(WorkflowDiagnostic {
        category: "invalid_input",
        code,
        operation_id: None,
        field: None,
        message: message.into(),
        suggestion: None,
        measured_clearance: None,
        required_clearance: None,
    })
}
fn geometry_diagnostic(code: &'static str, error: Error) -> Box<WorkflowDiagnostic> {
    let mut d = diagnostic(code, error.to_string());
    d.category = match error {
        Error::InvalidInput(_) => "invalid_input",
        Error::Unsupported(_) => "unsupported",
        _ => "validation_failed",
    };
    d
}

impl GraphWorkflowSession {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self) {
        self.accepted = None;
        self.snapshots.clear();
    }
    pub fn accepted_document(&self) -> Option<&GraphWorkflowDocument> {
        self.accepted.as_ref()
    }
    pub fn accepted_shape(&self) -> Option<&Arc<GraphWorkflowShape>> {
        self.snapshots.last()
    }
    /// Read-only access to an accepted operation's immutable prefix snapshot.
    pub fn prefix_snapshot(&self, index: usize) -> Option<&Arc<GraphWorkflowShape>> {
        self.snapshots.get(index)
    }
    fn prepare(
        &self,
        document: &GraphWorkflowDocument,
    ) -> std::result::Result<
        (GraphWorkflowRebuild, Vec<Arc<GraphWorkflowShape>>),
        Box<WorkflowDiagnostic>,
    > {
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
        let mut snapshots = self.snapshots[..reused].to_vec();
        let mut rebuilt_operation_ids = Vec::new();
        for index in reused..document.operations.len() {
            let shape = plan.apply_operation(index, snapshots.last().map(Arc::as_ref))?;
            shape.validate(plan.tolerance.absolute()).map_err(|error| {
                let mut d = geometry_diagnostic("geometry_rejected", error);
                d.operation_id = Some(document.operation_id(index).to_owned());
                d
            })?;
            snapshots.push(Arc::new(shape));
            rebuilt_operation_ids.push(document.operation_id(index).to_owned());
        }
        let shape = snapshots
            .last()
            .cloned()
            .ok_or_else(|| diagnostic("empty_history", "Graph workflow has no shape."))?;
        Ok((
            GraphWorkflowRebuild {
                shape,
                stats: WorkflowRebuildStats {
                    reused_operations: reused,
                    rebuilt_operations: rebuilt_operation_ids.len(),
                    rebuilt_operation_ids,
                },
            },
            snapshots,
        ))
    }
    /// Commit a successfully validated typed result without generating a display.
    pub fn rebuild(
        &mut self,
        document: &GraphWorkflowDocument,
    ) -> std::result::Result<GraphWorkflowRebuild, Box<WorkflowDiagnostic>> {
        let (result, snapshots) = self.prepare(document)?;
        self.accepted = Some(document.clone());
        self.snapshots = snapshots;
        Ok(result)
    }
    /// Serialize a candidate using the caller's callback before committing it.
    /// The callback owns display validation and the returned payload. This method
    /// guarantees rollback on callback errors; `evaluate_json` additionally uses
    /// Hagane's retained-shape bounded display serializer.
    pub fn evaluate_json_with(
        &mut self,
        input: &str,
        serialize_shape: impl FnOnce(&GraphWorkflowShape, &GraphWorkflowDocument) -> Result<Value>,
    ) -> Result<String> {
        let parsed = if input.len() > 65536 {
            Err(diagnostic(
                "document_too_large",
                "Graph workflow exceeds 64 KiB.",
            ))
        } else {
            serde_json::from_str::<GraphWorkflowDocument>(input).map_err(|error| {
                diagnostic(
                    "invalid_document",
                    format!("Invalid graph workflow: {error}"),
                )
            })
        };
        let document = match parsed {
            Ok(document) => document,
            Err(d) => return encode(&json!({"ok":false,"diagnostic":d})),
        };
        let (result, snapshots) = match self.prepare(&document) {
            Ok(candidate) => candidate,
            Err(d) => {
                return encode(&json!({"ok":false,"diagnostic":d,"attempted_document":document}))
            }
        };
        let shape = match serialize_shape(&result.shape, &document) {
            Ok(shape) => shape,
            Err(error) => {
                let mut d = geometry_diagnostic("display_rejected", error);
                d.field = Some("display");
                return encode(&json!({"ok":false,"diagnostic":d,"attempted_document":document}));
            }
        };
        let text =
            encode(&json!({"ok":true,"document":document,"shape":shape,"rebuild":result.stats}))?;
        self.accepted = Some(document);
        self.snapshots = snapshots;
        Ok(text)
    }
    pub fn evaluate_json(&mut self, input: &str) -> Result<String> {
        self.evaluate_json_with(
            input,
            crate::graph_workflow_demo::serialize_graph_workflow_shape,
        )
    }
}
fn encode(value: &Value) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|_| Error::InvalidInput("graph workflow serialization failed"))
}
