//! Versioned editable graph-family intent, replayed through actual typed B-reps.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphWorkflowDocument {
    pub schema_version: u32,
    pub units: String,
    pub tolerance: WorkflowTolerance,
    pub display: GraphWorkflowDisplay,
    pub operations: Vec<GraphWorkflowOperation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphWorkflowDisplay {
    pub max_error: f64,
    pub max_triangles: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GraphWorkflowOperation {
    GraphStock {
        id: String,
        dimensions: [f64; 3],
        bulge: f64,
    },
    UvTrim {
        id: String,
        input: String,
        ranges: [[f64; 2]; 2],
    },
    Placement {
        id: String,
        input: String,
        axis: [f64; 3],
        angle: f64,
        translation: [f64; 3],
    },
    CircularThroughBore {
        id: String,
        input: String,
        center: [f64; 2],
        radius: f64,
    },
}
#[derive(Clone, Debug)]
pub enum GraphWorkflowShape {
    Plain(Box<NurbsGraphSolid>),
    Circular(Box<NurbsGraphCircularHoledSolid>),
}
pub(crate) fn graph_workflow_diagnostic(
    category: &'static str,
    code: &'static str,
    id: Option<&str>,
    field: Option<&'static str>,
    message: impl Into<String>,
) -> Box<WorkflowDiagnostic> {
    Box::new(WorkflowDiagnostic {
        category,
        code,
        operation_id: id.map(str::to_owned),
        field,
        message: message.into(),
        suggestion: None,
        measured_clearance: None,
        required_clearance: None,
    })
}
fn geometry_error(e: Error, id: &str) -> Box<WorkflowDiagnostic> {
    let category = match e {
        Error::InvalidInput(_) => "invalid_input",
        Error::Unsupported(_) => "unsupported",
        _ => "validation_failed",
    };
    graph_workflow_diagnostic(category, "geometry_rejected", Some(id), None, e.to_string())
}
impl GraphWorkflowOperation {
    pub fn id(&self) -> &str {
        match self {
            Self::GraphStock { id, .. }
            | Self::UvTrim { id, .. }
            | Self::Placement { id, .. }
            | Self::CircularThroughBore { id, .. } => id,
        }
    }
    fn input(&self) -> Option<&str> {
        match self {
            Self::GraphStock { .. } => None,
            Self::UvTrim { input, .. }
            | Self::Placement { input, .. }
            | Self::CircularThroughBore { input, .. } => Some(input),
        }
    }
}
pub(crate) struct GraphWorkflowPlan<'a> {
    document: &'a GraphWorkflowDocument,
    pub tolerance: GeometryTolerance,
}
impl GraphWorkflowDocument {
    pub fn geometry_tol(&self) -> std::result::Result<GeometryTolerance, Box<WorkflowDiagnostic>> {
        GeometryTolerance::new(
            self.tolerance.linear,
            self.tolerance.angular,
            self.tolerance.relative,
        )
        .map_err(|e| {
            graph_workflow_diagnostic(
                "invalid_input",
                "invalid_tolerance",
                None,
                Some("tolerance"),
                e.to_string(),
            )
        })
    }
    pub(crate) fn operation_id(&self, index: usize) -> &str {
        self.operations[index].id()
    }
    pub(crate) fn checked_plan(
        &self,
    ) -> std::result::Result<GraphWorkflowPlan<'_>, Box<WorkflowDiagnostic>> {
        if self.schema_version != 1 {
            return Err(graph_workflow_diagnostic(
                "unsupported",
                "unsupported_schema",
                None,
                Some("schema_version"),
                "Only graph workflow schema version 1 is supported.",
            ));
        }
        if self.units != "mm" {
            return Err(graph_workflow_diagnostic(
                "unsupported",
                "unsupported_units",
                None,
                Some("units"),
                "Graph workflow lengths must use mm.",
            ));
        }
        let tolerance = self.geometry_tol()?;
        if !self.display.max_error.is_finite()
            || self.display.max_error <= 0.
            || !(32..=65536).contains(&self.display.max_triangles)
        {
            return Err(graph_workflow_diagnostic(
                "invalid_input",
                "invalid_display",
                None,
                Some("display"),
                "Display requires a finite positive error and 32 through 65536 triangles.",
            ));
        }
        if !(1..=16).contains(&self.operations.len()) {
            return Err(graph_workflow_diagnostic(
                "invalid_input",
                "invalid_operation_count",
                None,
                Some("operations"),
                "Graph workflow requires 1 through 16 operations.",
            ));
        }
        let mut ids = BTreeSet::new();
        for (i, op) in self.operations.iter().enumerate() {
            let id = op.id();
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                || !ids.insert(id)
            {
                return Err(graph_workflow_diagnostic("invalid_input","invalid_operation_id",Some(id),Some("id"),"Operation IDs must be unique 1 through 64 ASCII letters, digits, hyphens or underscores."));
            }
            if i == 0 {
                if !matches!(op, GraphWorkflowOperation::GraphStock { .. }) {
                    return Err(graph_workflow_diagnostic(
                        "unsupported",
                        "invalid_operation_order",
                        Some(id),
                        Some("kind"),
                        "The first operation must be graph_stock.",
                    ));
                }
            } else if op.input() != Some(self.operations[i - 1].id()) {
                return Err(graph_workflow_diagnostic(
                    "invalid_input",
                    "invalid_input_reference",
                    Some(id),
                    Some("input"),
                    "Each operation must reference the immediately previous operation.",
                ));
            }
            let bad = |field, message| {
                graph_workflow_diagnostic(
                    "invalid_input",
                    "invalid_operation_parameter",
                    Some(id),
                    Some(field),
                    message,
                )
            };
            match op {
                GraphWorkflowOperation::GraphStock {
                    dimensions, bulge, ..
                } => {
                    if i != 0 {
                        return Err(graph_workflow_diagnostic(
                            "unsupported",
                            "invalid_operation_order",
                            Some(id),
                            Some("kind"),
                            "Only one initial graph_stock is supported.",
                        ));
                    }
                    if dimensions.iter().any(|x| !x.is_finite() || *x <= 0.) || !bulge.is_finite() {
                        return Err(bad(
                            "dimensions",
                            "Graph stock dimensions must be finite and positive, and bulge finite.",
                        ));
                    }
                }
                GraphWorkflowOperation::UvTrim { ranges, .. } => {
                    if ranges
                        .iter()
                        .any(|r| r.iter().any(|x| !x.is_finite()) || r[0] >= r[1])
                    {
                        return Err(bad(
                            "ranges",
                            "UV trim intervals must be finite and strictly ordered.",
                        ));
                    }
                }
                GraphWorkflowOperation::Placement {
                    axis,
                    angle,
                    translation,
                    ..
                } => {
                    let length = axis[0].hypot(axis[1]).hypot(axis[2]);
                    if axis.iter().chain(translation).any(|x| !x.is_finite())
                        || !angle.is_finite()
                        || !length.is_finite()
                        || length == 0.
                    {
                        return Err(bad("placement","Placement requires a finite nonzero axis, finite angle and translation."));
                    }
                }
                GraphWorkflowOperation::CircularThroughBore { center, radius, .. } => {
                    if i != self.operations.len() - 1 {
                        return Err(graph_workflow_diagnostic(
                            "unsupported",
                            "invalid_operation_order",
                            Some(id),
                            Some("kind"),
                            "A single circular bore is supported only as the final operation.",
                        ));
                    }
                    if center.iter().any(|x| !x.is_finite()) || !radius.is_finite() || *radius <= 0.
                    {
                        return Err(bad(
                            "radius",
                            "Circular bore center must be finite and radius finite and positive.",
                        ));
                    }
                }
            }
        }
        Ok(GraphWorkflowPlan {
            document: self,
            tolerance,
        })
    }
    /// Rebuild exact typed geometry from modeling intent. Display meshes are never input.
    pub fn rebuild(&self) -> std::result::Result<GraphWorkflowShape, Box<WorkflowDiagnostic>> {
        let plan = self.checked_plan()?;
        let mut shape = None;
        for index in 0..self.operations.len() {
            shape = Some(plan.apply_operation(index, shape.as_ref())?);
        }
        Ok(shape.unwrap())
    }
}
impl GraphWorkflowPlan<'_> {
    pub(crate) fn apply_operation(
        &self,
        index: usize,
        previous: Option<&GraphWorkflowShape>,
    ) -> std::result::Result<GraphWorkflowShape, Box<WorkflowDiagnostic>> {
        let op = self.document.operations.get(index).ok_or_else(|| {
            graph_workflow_diagnostic(
                "invalid_input",
                "invalid_operation_index",
                None,
                None,
                "Operation index is outside the checked document.",
            )
        })?;
        let tol = self.tolerance.absolute();
        let id = op.id();
        let plain = || -> Result<&NurbsGraphSolid> {
            match previous {
                Some(GraphWorkflowShape::Plain(s)) => Ok(s),
                _ => Err(Error::Unsupported(
                    "this operation requires a preceding plain graph solid",
                )),
            }
        };
        let result = (|| -> Result<GraphWorkflowShape> {
            match op {
                GraphWorkflowOperation::GraphStock {
                    dimensions, bulge, ..
                } => {
                    if index != 0 || previous.is_some() {
                        return Err(Error::Unsupported(
                            "graph_stock must begin the operation history",
                        ));
                    }
                    Ok(GraphWorkflowShape::Plain(Box::new(NurbsGraphSolid::new(
                        *dimensions,
                        *bulge,
                        tol,
                    )?)))
                }
                GraphWorkflowOperation::UvTrim { ranges, .. } => Ok(GraphWorkflowShape::Plain(
                    Box::new(plain()?.trimmed_uv(*ranges, tol)?),
                )),
                GraphWorkflowOperation::Placement {
                    axis,
                    angle,
                    translation,
                    ..
                } => {
                    let pose = Transform::translation(Vec3::new(
                        translation[0],
                        translation[1],
                        translation[2],
                    ))?
                    .compose(Transform::rotation(
                        Vec3::new(axis[0], axis[1], axis[2]),
                        *angle,
                    )?)?;
                    Ok(GraphWorkflowShape::Plain(Box::new(
                        plain()?.transformed(pose, tol)?,
                    )))
                }
                GraphWorkflowOperation::CircularThroughBore { center, radius, .. } => {
                    Ok(GraphWorkflowShape::Circular(Box::new(
                        plain()?.through_xy_circle(*center, *radius, tol)?,
                    )))
                }
            }
        })();
        let result = result.map_err(|e| geometry_error(e, id))?;
        result.validate(tol).map_err(|e| geometry_error(e, id))?;
        Ok(result)
    }
}
impl GraphWorkflowShape {
    pub fn source(&self) -> &NurbsGraphSolid {
        match self {
            Self::Plain(s) => s,
            Self::Circular(s) => s.source(),
        }
    }
    pub fn brep(&self) -> &Solid {
        match self {
            Self::Plain(s) => s.brep(),
            Self::Circular(s) => s.brep(),
        }
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        match self {
            Self::Plain(s) => s.validate(tol),
            Self::Circular(s) => s.validate(tol),
        }
    }
    pub fn volume(&self) -> Result<f64> {
        match self {
            Self::Plain(s) => s.volume(),
            Self::Circular(s) => s.volume(),
        }
    }
    pub fn bounds(&self) -> Result<Bounds> {
        match self {
            Self::Plain(s) => s.bounds(),
            Self::Circular(s) => s.bounds(),
        }
    }
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        match self {
            Self::Plain(s) => s.mass_properties(tol),
            Self::Circular(s) => s.mass_properties(tol),
        }
    }
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        match self {
            Self::Plain(s) => s.inertia_properties(tol),
            Self::Circular(s) => s.inertia_properties(tol),
        }
    }
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        match self {
            Self::Plain(s) => s.export_step_mm(tol),
            Self::Circular(s) => s.export_step_mm(tol),
        }
    }
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        match self {
            Self::Plain(s) => s.classify_point(point, tol),
            Self::Circular(s) => s.classify_point(point, tol),
        }
    }
    /// The shared budget counts triangles for both families (the plain kernel counts cells).
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        if !(32..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "graph workflow triangle budget must be 32 through 65536",
            ));
        }
        let mesh = match self {
            Self::Plain(s) => s.tessellate_bounded(error, max_triangles / 2, tol)?,
            Self::Circular(s) => s.tessellate_bounded(error, max_triangles, tol)?,
        };
        if mesh.mesh.triangles.len() > max_triangles {
            return Err(Error::Tessellation(
                "graph workflow display exceeded its triangle budget",
            ));
        }
        Ok(mesh)
    }
}
