//! Versioned branching intent for exact normal-prism Boolean bodies.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
type WorkflowResult<T> = std::result::Result<T, Box<WorkflowDiagnostic>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrismWorkflowDocument {
    pub schema_version: u32,
    pub document_type: String,
    pub units: String,
    pub tolerance: WorkflowTolerance,
    pub output: String,
    pub display_chord_tolerance: f64,
    pub operations: Vec<PrismWorkflowOperation>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrismWorkflowPlacement {
    pub y_angle: f64,
    pub translation: [f64; 3],
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PrismWorkflowBooleanKind {
    Difference,
    Intersection,
    Union,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrismWorkflowOperation {
    RoundedBox {
        id: String,
        size: [f64; 3],
        corner_radius: f64,
        #[serde(default)]
        placement: PrismWorkflowPlacement,
    },
    ArcLineExtrusion {
        id: String,
        outer: Vec<WorkflowProfileSegment>,
        #[serde(default)]
        holes: Vec<Vec<WorkflowProfileSegment>>,
        height: f64,
        #[serde(default)]
        placement: PrismWorkflowPlacement,
    },
    StepStock {
        id: String,
        step: String,
        axis: [f64; 3],
    },
    Boolean {
        id: String,
        left: String,
        right: String,
        operation: PrismWorkflowBooleanKind,
        axis: [f64; 3],
    },
    Select {
        id: String,
        input: String,
        component: usize,
    },
}
#[derive(Clone, Debug)]
pub enum PrismWorkflowShape {
    Components(Vec<Arc<Solid>>),
}
impl PrismWorkflowShape {
    pub fn components(&self) -> &[Arc<Solid>] {
        match self {
            Self::Components(v) => v,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct PrismWorkflowRebuildStats {
    pub evaluated_nodes: usize,
    pub reused_nodes: usize,
}
#[derive(Clone, Debug)]
pub struct PrismWorkflowRebuild {
    pub shape: Arc<PrismWorkflowShape>,
    pub stats: PrismWorkflowRebuildStats,
}
fn diagnostic(
    category: &'static str,
    code: &'static str,
    id: Option<&str>,
    message: impl Into<String>,
) -> Box<WorkflowDiagnostic> {
    Box::new(WorkflowDiagnostic {
        category,
        code,
        operation_id: id.map(str::to_owned),
        field: None,
        message: message.into(),
        suggestion: None,
        measured_clearance: None,
        required_clearance: None,
    })
}
fn geometry(error: Error, id: &str) -> Box<WorkflowDiagnostic> {
    let category = match error {
        Error::InvalidInput(_) => "invalid_input",
        Error::Unsupported(_) => "unsupported",
        _ => "validation_failed",
    };
    diagnostic(category, "geometry_rejected", Some(id), error.to_string())
}
impl PrismWorkflowOperation {
    pub fn id(&self) -> &str {
        match self {
            Self::RoundedBox { id, .. }
            | Self::ArcLineExtrusion { id, .. }
            | Self::StepStock { id, .. }
            | Self::Boolean { id, .. }
            | Self::Select { id, .. } => id,
        }
    }
    fn inputs(&self) -> Vec<&str> {
        match self {
            Self::Boolean { left, right, .. } => vec![left, right],
            Self::Select { input, .. } => vec![input],
            _ => Vec::new(),
        }
    }
}
fn segment(value: &WorkflowProfileSegment) -> PlanarSegment {
    match value {
        WorkflowProfileSegment::Line { start, end } => PlanarSegment::Line { a: *start, b: *end },
        WorkflowProfileSegment::Arc {
            center,
            radius,
            start_angle,
            sweep,
        } => PlanarSegment::Arc {
            center: *center,
            radius: *radius,
            start_angle: *start_angle,
            sweep: *sweep,
        },
    }
}
fn quarter_domain(body: &Solid, axis: Vec3, policy: GeometryTolerance) -> Result<()> {
    let tol = policy.absolute();
    let stock =
        crate::arc_line_prism_validation::recognize_validated_normal_prism(body, axis, tol)?;
    if stock.region.holes.len()>16 || std::iter::once(&stock.region.outer).chain(&stock.region.holes).flatten().any(|s|matches!(s,PlanarSegment::Arc{sweep,..}if sweep.abs()>std::f64::consts::FRAC_PI_2+64.*f64::EPSILON)) {
        return Err(Error::Unsupported("prism workflow stock requires at most sixteen holes and quarter/smaller arcs"));
    }
    if stock.region.outer.len() + stock.region.holes.iter().map(Vec::len).sum::<usize>() > 128 {
        return Err(Error::Unsupported("prism workflow profile budget exceeded"));
    }
    let span = body.bounds().max - body.bounds().min;
    let scale = span.x.hypot(span.y).hypot(span.z);
    let effective = Tolerance::new(policy.length_at_scale(scale)?)?;
    if stock.height <= 10. * effective.linear {
        return Err(Error::Unsupported(
            "prism workflow stock height is unresolved under its effective tolerance",
        ));
    }
    let mut rings = vec![stock.region.outer];
    rings.extend(stock.region.holes);
    crate::mixed::validate_mixed_region_trim(&rings, effective)?;
    Ok(())
}
fn place(body: Solid, placement: &PrismWorkflowPlacement, tol: GeometryTolerance) -> Result<Solid> {
    let rotation = Transform::rotation(Vec3::new(0., 1., 0.), placement.y_angle)?;
    let transform = Transform::new_with_tolerance(
        Vec3::new(
            placement.translation[0],
            placement.translation[1],
            placement.translation[2],
        ),
        rotation.axes(),
        tol,
    )?;
    let body = body.transformed(transform, tol.absolute())?;
    quarter_domain(&body, transform.vector(Vec3::new(0., 0., 1.)), tol)?;
    Ok(body)
}
impl PrismWorkflowDocument {
    pub fn geometry_tol(&self) -> WorkflowResult<GeometryTolerance> {
        GeometryTolerance::new(
            self.tolerance.linear,
            self.tolerance.angular,
            self.tolerance.relative,
        )
        .map_err(|e| diagnostic("invalid_input", "invalid_tolerance", None, e.to_string()))
    }
    fn checked(&self) -> WorkflowResult<GeometryTolerance> {
        if self.schema_version != 1
            || self.document_type != "hagane_prism_workflow"
            || self.units != "mm"
        {
            return Err(diagnostic(
                "unsupported",
                "unsupported_schema",
                None,
                "Expected hagane_prism_workflow schema 1 in mm.",
            ));
        }
        if !self.display_chord_tolerance.is_finite() || self.display_chord_tolerance <= 0. {
            return Err(diagnostic(
                "invalid_input",
                "invalid_display_tolerance",
                None,
                "Display chord tolerance must be positive and finite.",
            ));
        }
        if self.operations.is_empty() || self.operations.len() > 16 {
            return Err(diagnostic(
                "invalid_input",
                "operation_limit",
                None,
                "Prism workflow requires 1..16 operations.",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut step_bytes = 0usize;
        for operation in &self.operations {
            let id = operation.id();
            match operation {
                PrismWorkflowOperation::ArcLineExtrusion { outer, holes, .. } => {
                    let profile_count = holes
                        .iter()
                        .try_fold(outer.len(), |total, hole| total.checked_add(hole.len()));
                    if holes.len() > 16 || profile_count.is_none_or(|count| count > 128) {
                        return Err(diagnostic("unsupported", "profile_resource_limit", Some(id), "Stock profiles require at most sixteen holes and 128 combined segments."));
                    }
                }
                PrismWorkflowOperation::StepStock { step, .. } => {
                    if step.len() > 1024 * 1024 {
                        return Err(diagnostic(
                            "unsupported",
                            "step_resource_limit",
                            Some(id),
                            "Each STEP stock is limited to one MiB.",
                        ));
                    }
                    step_bytes += step.len();
                    if step_bytes > 2 * 1024 * 1024 {
                        return Err(diagnostic(
                            "unsupported",
                            "document_step_resource_limit",
                            Some(id),
                            "Combined STEP stocks are limited to two MiB.",
                        ));
                    }
                }
                _ => {}
            }
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                || ids.contains(id)
            {
                return Err(diagnostic(
                    "invalid_input",
                    "invalid_operation_id",
                    Some(id),
                    "IDs must be unique ASCII identifiers of 1..64 characters.",
                ));
            }
            for input in operation.inputs() {
                if !ids.contains(input) {
                    return Err(diagnostic(
                        "invalid_input",
                        "invalid_reference",
                        Some(id),
                        "Inputs must reference earlier nodes.",
                    ));
                }
            }
            ids.insert(id);
        }
        if !ids.contains(self.output.as_str()) {
            return Err(diagnostic(
                "invalid_input",
                "invalid_output",
                None,
                "Output must name an existing node.",
            ));
        }
        self.geometry_tol()
    }
    fn apply(
        &self,
        index: usize,
        nodes: &[Arc<PrismWorkflowShape>],
        tol: GeometryTolerance,
    ) -> WorkflowResult<PrismWorkflowShape> {
        let operation = &self.operations[index];
        let lookup: BTreeMap<_, _> = self.operations[..index]
            .iter()
            .zip(nodes)
            .map(|(op, shape)| (op.id(), shape))
            .collect();
        let single = |input: &str| -> WorkflowResult<&Solid> {
            let shape = lookup.get(input).ok_or_else(|| {
                diagnostic(
                    "invalid_input",
                    "invalid_reference",
                    Some(operation.id()),
                    "Input node is unavailable.",
                )
            })?;
            if shape.components().len() != 1 {
                return Err(diagnostic(
                    "unsupported",
                    "component_selection_required",
                    Some(operation.id()),
                    "Boolean parents must each have exactly one component; add Select.",
                ));
            }
            Ok(shape.components()[0].as_ref())
        };
        let result: Result<PrismWorkflowShape> = (|| {
            let body = match operation {
                PrismWorkflowOperation::RoundedBox {
                    size,
                    corner_radius,
                    placement,
                    ..
                } => {
                    let profile = rounded_rectangle_profile(
                        Point3::new(0., 0., 0.),
                        size[0],
                        size[1],
                        *corner_radius,
                        tol.absolute(),
                    )?;
                    let body = extrude_arc_line(&profile, size[2], tol.absolute())?;
                    place(body, placement, tol)?
                }
                PrismWorkflowOperation::ArcLineExtrusion {
                    outer,
                    holes,
                    height,
                    placement,
                    ..
                } => {
                    let region = ArcLineRegion {
                        origin: Point3::new(0., 0., 0.),
                        outer: outer.iter().map(segment).collect(),
                        holes: holes
                            .iter()
                            .map(|h| h.iter().map(segment).collect())
                            .collect(),
                    };
                    let body = extrude_arc_line_region(&region, *height, tol.absolute())?;
                    place(body, placement, tol)?
                }
                PrismWorkflowOperation::StepStock { step, axis, .. } => {
                    let body = import_step_bounded_analytic_mm(step, tol.absolute())?;
                    quarter_domain(&body, Vec3::new(axis[0], axis[1], axis[2]), tol)?;
                    body
                }
                _ => return Err(Error::Unsupported("not a stock operation")),
            };
            Ok(PrismWorkflowShape::Components(vec![Arc::new(body)]))
        })();
        let shape = match operation {
            PrismWorkflowOperation::Boolean {
                left,
                right,
                operation: kind,
                axis,
                ..
            } => {
                let bodies = boolean_normal_prism_regions(
                    single(left)?,
                    single(right)?,
                    Vec3::new(axis[0], axis[1], axis[2]),
                    tol,
                )
                .map_err(|e| geometry(e, operation.id()))?;
                let (difference, intersection, union) = bodies.into_solids();
                let selected = match kind {
                    PrismWorkflowBooleanKind::Difference => difference,
                    PrismWorkflowBooleanKind::Intersection => intersection,
                    PrismWorkflowBooleanKind::Union => union,
                };
                PrismWorkflowShape::Components(selected.into_iter().map(Arc::new).collect())
            }
            PrismWorkflowOperation::Select {
                input, component, ..
            } => {
                let shape = lookup.get(input.as_str()).ok_or_else(|| {
                    diagnostic(
                        "invalid_input",
                        "invalid_reference",
                        Some(operation.id()),
                        "Input node is unavailable.",
                    )
                })?;
                let body = shape.components().get(*component).ok_or_else(|| {
                    diagnostic(
                        "invalid_input",
                        "invalid_component",
                        Some(operation.id()),
                        "Component index is unavailable.",
                    )
                })?;
                PrismWorkflowShape::Components(vec![body.clone()])
            }
            _ => result.map_err(|e| geometry(e, operation.id()))?,
        };
        for body in shape.components() {
            body.validate(tol.absolute())
                .map_err(|e| geometry(e, operation.id()))?;
            let volume = body.volume().map_err(|e| geometry(e, operation.id()))?;
            if !volume.is_finite() || volume <= 0. {
                return Err(diagnostic(
                    "validation_failed",
                    "invalid_volume",
                    Some(operation.id()),
                    "Every material component needs positive finite volume.",
                ));
            }
        }
        Ok(shape)
    }
    pub fn rebuild(&self) -> WorkflowResult<PrismWorkflowShape> {
        let tol = self.checked()?;
        let mut nodes = Vec::new();
        for i in 0..self.operations.len() {
            nodes.push(Arc::new(self.apply(i, &nodes, tol)?));
        }
        let index = self
            .operations
            .iter()
            .position(|op| op.id() == self.output)
            .ok_or_else(|| {
                diagnostic(
                    "invalid_input",
                    "invalid_output",
                    None,
                    "Output is unavailable.",
                )
            })?;
        Ok(nodes[index].as_ref().clone())
    }
}

/// Accepted node snapshots share actual immutable B-reps. Candidate callbacks
/// must finish display/export validation before any cache/history state commits.
#[derive(Default)]
pub struct PrismWorkflowSession {
    accepted: Option<PrismWorkflowDocument>,
    nodes: Vec<Arc<PrismWorkflowShape>>,
    undo: Vec<PrismWorkflowDocument>,
    redo: Vec<PrismWorkflowDocument>,
}
impl PrismWorkflowSession {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn accepted_document(&self) -> Option<&PrismWorkflowDocument> {
        self.accepted.as_ref()
    }
    pub fn prefix_snapshot(&self, index: usize) -> Option<&Arc<PrismWorkflowShape>> {
        self.nodes.get(index)
    }
    pub fn accepted_shape(&self) -> Option<&Arc<PrismWorkflowShape>> {
        let doc = self.accepted.as_ref()?;
        let index = doc.operations.iter().position(|op| op.id() == doc.output)?;
        self.nodes.get(index)
    }
    fn prepare(
        &self,
        doc: &PrismWorkflowDocument,
    ) -> WorkflowResult<(PrismWorkflowRebuild, Vec<Arc<PrismWorkflowShape>>)> {
        let tol = doc.checked()?;
        let reused = self
            .accepted
            .as_ref()
            .filter(|old| {
                old.tolerance == doc.tolerance
                    && old.schema_version == doc.schema_version
                    && old.units == doc.units
                    && old.document_type == doc.document_type
            })
            .map(|old| {
                old.operations
                    .iter()
                    .zip(&doc.operations)
                    .take_while(|(a, b)| a == b)
                    .count()
            })
            .unwrap_or(0);
        let mut nodes = self.nodes[..reused].to_vec();
        for i in reused..doc.operations.len() {
            nodes.push(Arc::new(doc.apply(i, &nodes, tol)?));
        }
        let output = doc
            .operations
            .iter()
            .position(|op| op.id() == doc.output)
            .ok_or_else(|| {
                diagnostic(
                    "invalid_input",
                    "invalid_output",
                    None,
                    "Output is unavailable.",
                )
            })?;
        Ok((
            PrismWorkflowRebuild {
                shape: nodes[output].clone(),
                stats: PrismWorkflowRebuildStats {
                    evaluated_nodes: doc.operations.len() - reused,
                    reused_nodes: reused,
                },
            },
            nodes,
        ))
    }
    pub fn rebuild_with<T, F>(
        &mut self,
        doc: &PrismWorkflowDocument,
        validate: F,
    ) -> WorkflowResult<(PrismWorkflowRebuild, T)>
    where
        F: FnOnce(&PrismWorkflowShape, &PrismWorkflowDocument) -> Result<T>,
    {
        let (report, nodes) = self.prepare(doc)?;
        let data = validate(report.shape.as_ref(), doc).map_err(|e| geometry(e, &doc.output))?;
        if self.accepted.as_ref() != Some(doc) {
            if let Some(old) = self.accepted.take() {
                self.undo.push(old);
                if self.undo.len() > 32 {
                    self.undo.remove(0);
                }
            }
            self.redo.clear();
        }
        self.accepted = Some(doc.clone());
        self.nodes = nodes;
        Ok((report, data))
    }
    /// Default acceptance verifies actual bounded display and STEP for every
    /// selected component. Empty output is a valid material result.
    pub fn rebuild(&mut self, doc: &PrismWorkflowDocument) -> WorkflowResult<PrismWorkflowRebuild> {
        self.rebuild_with(doc, Self::validate_output)
            .map(|(report, ())| report)
    }
    fn validate_output(shape: &PrismWorkflowShape, doc: &PrismWorkflowDocument) -> Result<()> {
        let tol = GeometryTolerance::new(
            doc.tolerance.linear,
            doc.tolerance.angular,
            doc.tolerance.relative,
        )?;
        for body in shape.components() {
            crate::edge_fillet_demo::solid_json(body, doc.display_chord_tolerance, tol.absolute())?;
            export_step_bounded_analytic_mm(body, tol.linear())?;
        }
        Ok(())
    }
    pub fn undo_with<T, F>(&mut self, validate: F) -> WorkflowResult<(PrismWorkflowRebuild, T)>
    where
        F: FnOnce(&PrismWorkflowShape, &PrismWorkflowDocument) -> Result<T>,
    {
        let doc = self.undo.last().cloned().ok_or_else(|| {
            diagnostic(
                "invalid_input",
                "no_undo",
                None,
                "No accepted edit to undo.",
            )
        })?;
        let (report, nodes) = self.prepare(&doc)?;
        let data = validate(report.shape.as_ref(), &doc).map_err(|e| geometry(e, &doc.output))?;
        let current = self
            .accepted
            .replace(doc)
            .ok_or_else(|| diagnostic("invalid_input", "no_undo", None, "No accepted document."))?;
        self.undo.pop();
        self.redo.push(current);
        self.nodes = nodes;
        Ok((report, data))
    }
    pub fn redo_with<T, F>(&mut self, validate: F) -> WorkflowResult<(PrismWorkflowRebuild, T)>
    where
        F: FnOnce(&PrismWorkflowShape, &PrismWorkflowDocument) -> Result<T>,
    {
        let doc = self.redo.last().cloned().ok_or_else(|| {
            diagnostic(
                "invalid_input",
                "no_redo",
                None,
                "No accepted edit to redo.",
            )
        })?;
        let (report, nodes) = self.prepare(&doc)?;
        let data = validate(report.shape.as_ref(), &doc).map_err(|e| geometry(e, &doc.output))?;
        let current = self
            .accepted
            .replace(doc)
            .ok_or_else(|| diagnostic("invalid_input", "no_redo", None, "No accepted document."))?;
        self.redo.pop();
        self.undo.push(current);
        self.nodes = nodes;
        Ok((report, data))
    }
    pub fn undo(&mut self) -> WorkflowResult<PrismWorkflowRebuild> {
        self.undo_with(Self::validate_output)
            .map(|(report, ())| report)
    }
    pub fn redo(&mut self) -> WorkflowResult<PrismWorkflowRebuild> {
        self.redo_with(Self::validate_output)
            .map(|(report, ())| report)
    }
    pub fn undo_count(&self) -> usize {
        self.undo.len()
    }
    pub fn redo_count(&self) -> usize {
        self.redo.len()
    }
}
