//! Versioned branching replay of checked cardinal and axis-angle rational frusta.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
/// Maximum total display triangles accepted across all selected components.
pub(crate) const NURBS_FRUSTUM_WORKFLOW_MAX_DISPLAY_TRIANGLES: usize = 131072;
type WorkflowResult<T> = std::result::Result<T, Box<WorkflowDiagnostic>>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NurbsFrustumWorkflowDocument {
    pub schema_version: u32,
    pub document_type: String,
    pub units: String,
    pub tolerance: WorkflowTolerance,
    pub output: String,
    pub display_chord_tolerance: f64,
    pub operations: Vec<NurbsFrustumWorkflowOperation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NurbsFrustumWorkflowOperation {
    Frustum {
        id: String,
        radii: [f64; 2],
        height: f64,
        origin: [f64; 3],
        axes: [[f64; 3]; 3],
    },
    PosedFrustum {
        id: String,
        radii: [f64; 2],
        height: f64,
        origin: [f64; 3],
        rotation_axis: [f64; 3],
        angle: f64,
    },
    StepStock {
        id: String,
        step: String,
    },
    Partition {
        id: String,
        input: String,
        cuts: Vec<f64>,
    },
    PlaneSplit {
        id: String,
        input: String,
        origin: [f64; 3],
        normal: [f64; 3],
    },
    CoaxialBore {
        id: String,
        input: String,
        radius: f64,
    },
    Select {
        id: String,
        input: String,
        component: usize,
    },
}
/// Typed retained bodies; canonical-only operations require explicit matching.
#[derive(Clone, Debug)]
pub enum NurbsFrustumWorkflowComponent {
    Canonical(Arc<NurbsFrustumSolid>),
    Oblique(Arc<NurbsObliqueFrustumSolid>),
    Bored(Arc<NurbsBoredFrustumSolid>),
    BoredOblique(Arc<NurbsBoredObliqueFrustumSolid>),
    Slab(Arc<NurbsFrustumSlabSolid>),
    BoredSlab(Arc<NurbsBoredFrustumSlabSolid>),
}
impl NurbsFrustumWorkflowComponent {
    pub fn canonical(&self) -> Option<&Arc<NurbsFrustumSolid>> {
        if let Self::Canonical(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn bored(&self) -> Option<&Arc<NurbsBoredFrustumSolid>> {
        if let Self::Bored(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn oblique(&self) -> Option<&Arc<NurbsObliqueFrustumSolid>> {
        if let Self::Oblique(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn classify_point(
        &self,
        query: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        match self {
            Self::Canonical(v) => v.classify_point(query, policy),
            Self::Oblique(v) => v.classify_point(query, policy),
            Self::Bored(v) => v.classify_point(query, policy),
            Self::BoredOblique(v) => v.classify_point(query, policy),
            Self::Slab(_) | Self::BoredSlab(_) => Err(Error::Unsupported(
                "parallel frustum slab classify_point is not implemented",
            )),
        }
    }
    pub fn solid(&self) -> &Solid {
        match self {
            Self::Canonical(v) => v.solid(),
            Self::Oblique(v) => v.solid(),
            Self::Bored(v) => v.solid(),
            Self::BoredOblique(v) => v.solid(),
            Self::Slab(v) => v.solid(),
            Self::BoredSlab(v) => v.solid(),
        }
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        match self {
            Self::Canonical(v) => v.validate(policy),
            Self::Oblique(v) => v.validate(policy),
            Self::Bored(v) => v.validate(policy),
            Self::BoredOblique(v) => v.validate(policy),
            Self::Slab(v) => v.validate(policy),
            Self::BoredSlab(v) => v.validate(policy),
        }
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        match self {
            Self::Canonical(v) => v.volume(policy),
            Self::Oblique(v) => v.volume(policy),
            Self::Bored(v) => v.volume(policy),
            Self::BoredOblique(v) => v.volume(policy),
            Self::Slab(v) => v.volume(policy),
            Self::BoredSlab(v) => v.volume(policy),
        }
    }
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        match self {
            Self::Canonical(v) => v.mass_properties(policy),
            Self::Oblique(v) => v.mass_properties(policy),
            Self::Bored(v) => v.mass_properties(policy),
            Self::BoredOblique(v) => v.mass_properties(policy),
            Self::Slab(_) | Self::BoredSlab(_) => Err(Error::Unsupported(
                "parallel frustum slab mass_properties is not implemented",
            )),
        }
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        match self {
            Self::Canonical(v) => v.inertia_properties(policy),
            Self::Oblique(v) => v.inertia_properties(policy),
            Self::Bored(v) => v.inertia_properties(policy),
            Self::BoredOblique(v) => v.inertia_properties(policy),
            Self::Slab(_) | Self::BoredSlab(_) => Err(Error::Unsupported(
                "parallel frustum slab inertia_properties is not implemented",
            )),
        }
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        match self {
            Self::Canonical(v) => v.export_step_mm(policy),
            Self::Oblique(v) => v.export_step_mm(policy),
            Self::Bored(v) => v.export_step_mm(policy),
            Self::BoredOblique(v) => v.export_step_mm(policy),
            Self::Slab(v) => v.export_step_mm(policy),
            Self::BoredSlab(v) => v.export_step_mm(policy),
        }
    }
    pub fn tessellate(&self, chord: f64, policy: GeometryTolerance) -> Result<Mesh> {
        match self {
            Self::Canonical(v) => v.tessellate(chord, policy),
            Self::Oblique(v) => v.tessellate(chord, policy),
            Self::Bored(v) => v.tessellate(chord, policy),
            Self::BoredOblique(v) => v.tessellate(chord, policy),
            Self::Slab(v) => v.tessellate(chord, policy),
            Self::BoredSlab(v) => v.tessellate(chord, policy),
        }
    }
}
#[derive(Clone, Debug)]
pub enum NurbsFrustumWorkflowShape {
    Components(Vec<NurbsFrustumWorkflowComponent>),
}
impl NurbsFrustumWorkflowShape {
    pub fn components(&self) -> &[NurbsFrustumWorkflowComponent] {
        match self {
            Self::Components(v) => v,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct NurbsFrustumWorkflowRebuildStats {
    pub evaluated_nodes: usize,
    pub reused_nodes: usize,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumWorkflowRebuild {
    pub shape: Arc<NurbsFrustumWorkflowShape>,
    pub stats: NurbsFrustumWorkflowRebuildStats,
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
impl NurbsFrustumWorkflowOperation {
    pub fn id(&self) -> &str {
        match self {
            Self::Frustum { id, .. }
            | Self::PosedFrustum { id, .. }
            | Self::StepStock { id, .. }
            | Self::Partition { id, .. }
            | Self::PlaneSplit { id, .. }
            | Self::CoaxialBore { id, .. }
            | Self::Select { id, .. } => id,
        }
    }
    fn input(&self) -> Option<&str> {
        match self {
            Self::Partition { input, .. }
            | Self::PlaneSplit { input, .. }
            | Self::CoaxialBore { input, .. }
            | Self::Select { input, .. } => Some(input),
            _ => None,
        }
    }
}
fn cardinal(axes: [[f64; 3]; 3]) -> bool {
    axes.iter().all(|a| {
        a.iter().filter(|&&x| x == 1. || x == -1.).count() == 1
            && a.iter().all(|&x| x == 0. || x == 1. || x == -1.)
    }) && {
        let a = axes.map(|a| Vec3::new(a[0], a[1], a[2]));
        a[0].dot(a[1]) == 0. && a[0].cross(a[1]) == a[2]
    }
}
impl NurbsFrustumWorkflowDocument {
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
            || self.document_type != "nurbs_frustum_workflow"
            || self.units != "mm"
        {
            return Err(diagnostic(
                "unsupported",
                "unsupported_schema",
                None,
                "Expected nurbs_frustum_workflow schema 1 in mm.",
            ));
        }
        if !self.display_chord_tolerance.is_finite() || self.display_chord_tolerance <= 0. {
            return Err(diagnostic(
                "invalid_input",
                "invalid_display_tolerance",
                None,
                "Display tolerance must be positive and finite.",
            ));
        }
        if self.operations.is_empty() || self.operations.len() > 16 {
            return Err(diagnostic(
                "invalid_input",
                "operation_limit",
                None,
                "Expected 1..16 operations.",
            ));
        }
        let tol = self.geometry_tol()?;
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        for op in &self.operations {
            let id = op.id();
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                || ids.contains(id)
            {
                return Err(diagnostic(
                    "invalid_input",
                    "invalid_id",
                    Some(id),
                    "IDs must be unique bounded ASCII names.",
                ));
            }
            if let Some(input) = op.input() {
                if !ids.contains(input) {
                    return Err(diagnostic(
                        "invalid_input",
                        "invalid_input_reference",
                        Some(id),
                        "Inputs must reference an earlier node.",
                    ));
                }
            }
            match op {
                NurbsFrustumWorkflowOperation::Frustum {
                    radii,
                    height,
                    origin,
                    axes,
                    ..
                } => {
                    if !cardinal(*axes)
                        || !origin.iter().all(|x| x.is_finite())
                        || !radii.iter().all(|x| x.is_finite() && *x > 0.)
                        || !height.is_finite()
                        || *height <= 0.
                    {
                        return Err(diagnostic("invalid_input","invalid_frustum",Some(id),"Frustum requires positive finite dimensions and an exact right-handed cardinal frame."));
                    }
                }
                NurbsFrustumWorkflowOperation::PosedFrustum {
                    radii,
                    height,
                    origin,
                    rotation_axis,
                    angle,
                    ..
                } => {
                    let axis = Vec3::new(rotation_axis[0], rotation_axis[1], rotation_axis[2]);
                    if !origin.iter().all(|x| x.is_finite())
                        || !radii.iter().all(|x| x.is_finite() && *x > 0.)
                        || !height.is_finite()
                        || *height <= 0.
                        || !angle.is_finite()
                        || angle.abs() > std::f64::consts::PI
                        || axis.normalized().is_err()
                    {
                        return Err(diagnostic("invalid_input","invalid_posed_frustum",Some(id),"Posed frustum requires positive finite dimensions, finite origin, nonzero finite rotation axis and angle in [-pi,pi] radians."));
                    }
                    Transform::rotation(axis, *angle).map_err(|e| geometry(e, id))?;
                }
                NurbsFrustumWorkflowOperation::StepStock { step, .. } => {
                    bytes = bytes.checked_add(step.len()).ok_or_else(|| {
                        diagnostic(
                            "invalid_input",
                            "step_limit",
                            Some(id),
                            "STEP budget exceeded.",
                        )
                    })?;
                    if step.is_empty() || step.len() > 1024 * 1024 || bytes > 2 * 1024 * 1024 {
                        return Err(diagnostic(
                            "invalid_input",
                            "step_limit",
                            Some(id),
                            "STEP source is empty or exceeds its bounded byte budget.",
                        ));
                    }
                }
                NurbsFrustumWorkflowOperation::CoaxialBore { radius, .. }
                    if !radius.is_finite() || *radius <= 0. =>
                {
                    return Err(diagnostic(
                        "invalid_input",
                        "invalid_radius",
                        Some(id),
                        "Coaxial bore radius must be positive and finite.",
                    ));
                }
                NurbsFrustumWorkflowOperation::PlaneSplit { origin, normal, .. }
                    if origin.iter().chain(normal).any(|x| !x.is_finite())
                        || Vec3::new(normal[0], normal[1], normal[2])
                            .normalized()
                            .is_err() =>
                {
                    return Err(diagnostic(
                        "invalid_input",
                        "invalid_plane",
                        Some(id),
                        "Plane origin must be finite and its normal finite and nonzero.",
                    ));
                }
                NurbsFrustumWorkflowOperation::Partition { cuts, .. }
                    if cuts.is_empty()
                        || cuts.len() > 16
                        || cuts.iter().any(|x| !x.is_finite() || *x <= 0.)
                        || cuts.windows(2).any(|w| w[0] >= w[1]) =>
                {
                    return Err(diagnostic(
                        "invalid_input",
                        "invalid_cuts",
                        Some(id),
                        "Cuts must be 1..16 positive finite strictly increasing axial distances.",
                    ));
                }
                _ => (),
            }
            ids.insert(id);
        }
        if !ids.contains(self.output.as_str()) {
            return Err(diagnostic(
                "invalid_input",
                "invalid_output",
                None,
                "Output must name an existing operation.",
            ));
        }
        Ok(tol)
    }
    fn apply(
        &self,
        index: usize,
        nodes: &[Arc<NurbsFrustumWorkflowShape>],
        tol: GeometryTolerance,
    ) -> WorkflowResult<NurbsFrustumWorkflowShape> {
        let op = &self.operations[index];
        let id = op.id();
        let inputs: BTreeMap<_, _> = self.operations[..index]
            .iter()
            .zip(nodes)
            .map(|(o, s)| (o.id(), s.as_ref()))
            .collect();
        let bodies = match op {
            NurbsFrustumWorkflowOperation::Frustum {
                radii,
                height,
                origin,
                axes,
                ..
            } => {
                let axes = axes.map(|a| Vec3::new(a[0], a[1], a[2]));
                let frame = Frame3::new_with_tolerance(
                    Point3::new(origin[0], origin[1], origin[2]),
                    axes,
                    tol,
                )
                .map_err(|e| geometry(e, id))?;
                vec![NurbsFrustumWorkflowComponent::Canonical(Arc::new(
                    NurbsFrustumSolid::new(frame, *radii, *height, tol)
                        .map_err(|e| geometry(e, id))?,
                ))]
            }
            NurbsFrustumWorkflowOperation::PosedFrustum {
                radii,
                height,
                origin,
                rotation_axis,
                angle,
                ..
            } => {
                // Rotate the local axes about the world direction, then place
                // the local origin at the supplied world point (R*p + origin).
                let rotation = Transform::rotation(
                    Vec3::new(rotation_axis[0], rotation_axis[1], rotation_axis[2]),
                    *angle,
                )
                .map_err(|e| geometry(e, id))?;
                let frame = Frame3::new_with_tolerance(
                    Point3::new(origin[0], origin[1], origin[2]),
                    rotation.axes(),
                    tol,
                )
                .map_err(|e| geometry(e, id))?;
                vec![NurbsFrustumWorkflowComponent::Canonical(Arc::new(
                    NurbsFrustumSolid::new(frame, *radii, *height, tol)
                        .map_err(|e| geometry(e, id))?,
                ))]
            }
            NurbsFrustumWorkflowOperation::StepStock { step, .. } => {
                vec![NurbsFrustumWorkflowComponent::Canonical(Arc::new(
                    import_step_nurbs_frustum_cardinal_mm(step, tol)
                        .map_err(|e| geometry(e, id))?,
                ))]
            }
            NurbsFrustumWorkflowOperation::Select {
                input, component, ..
            } => {
                let shape = inputs[input.as_str()];
                vec![shape.components().get(*component).cloned().ok_or_else(|| {
                    diagnostic(
                        "invalid_input",
                        "invalid_component",
                        Some(id),
                        "Selected component is unavailable.",
                    )
                })?]
            }
            NurbsFrustumWorkflowOperation::CoaxialBore { input, radius, .. } => {
                let shape = inputs[input.as_str()];
                if shape.components().len() != 1 {
                    return Err(diagnostic(
                        "unsupported",
                        "ambiguous_components",
                        Some(id),
                        "CoaxialBore requires one explicitly selected component.",
                    ));
                }
                let source = shape.components()[0].canonical().ok_or_else(|| diagnostic(
                    "unsupported", "canonical_required", Some(id),
                    "CoaxialBore requires a canonical frustum; bored or oblique components cannot be bored again."))?;
                vec![NurbsFrustumWorkflowComponent::Bored(Arc::new(
                    source
                        .bore_coaxial(*radius, tol)
                        .map_err(|e| geometry(e, id))?,
                ))]
            }
            NurbsFrustumWorkflowOperation::PlaneSplit {
                input,
                origin,
                normal,
                ..
            } => {
                let shape = inputs[input.as_str()];
                if shape.components().len() != 1 {
                    return Err(diagnostic(
                        "unsupported",
                        "ambiguous_components",
                        Some(id),
                        "PlaneSplit requires one explicitly selected component.",
                    ));
                }
                let normal = Vec3::new(normal[0], normal[1], normal[2])
                    .normalized()
                    .map_err(|e| geometry(e, id))?;
                let reference = if normal.x.abs() < 0.9 {
                    Vec3::new(1., 0., 0.)
                } else {
                    Vec3::new(0., 1., 0.)
                };
                let u = reference
                    .cross(normal)
                    .normalized()
                    .map_err(|e| geometry(e, id))?;
                let v = normal.cross(u).normalized().map_err(|e| geometry(e, id))?;
                let plane = Surface::Plane {
                    origin: Point3::new(origin[0], origin[1], origin[2]),
                    u,
                    v,
                };
                match &shape.components()[0] {
                    NurbsFrustumWorkflowComponent::Canonical(source) => {
                        let split = source
                            .split_by_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::Oblique(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::Oblique(Arc::new(split.upper)),
                        ]
                    }
                    NurbsFrustumWorkflowComponent::Bored(source) => {
                        let split = source
                            .split_by_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::BoredOblique(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::BoredOblique(Arc::new(split.upper)),
                        ]
                    }
                    NurbsFrustumWorkflowComponent::Oblique(source) => {
                        let split = source
                            .split_by_parallel_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::Slab(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::Slab(Arc::new(split.upper)),
                        ]
                    }
                    NurbsFrustumWorkflowComponent::Slab(source) => {
                        let split = source
                            .split_by_parallel_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::Slab(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::Slab(Arc::new(split.upper)),
                        ]
                    }
                    NurbsFrustumWorkflowComponent::BoredOblique(source) => {
                        let split = source
                            .split_by_parallel_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::BoredSlab(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::BoredSlab(Arc::new(split.upper)),
                        ]
                    }
                    NurbsFrustumWorkflowComponent::BoredSlab(source) => {
                        let split = source
                            .split_by_parallel_plane(&plane, tol)
                            .map_err(|e| geometry(e, id))?;
                        vec![
                            NurbsFrustumWorkflowComponent::BoredSlab(Arc::new(split.lower)),
                            NurbsFrustumWorkflowComponent::BoredSlab(Arc::new(split.upper)),
                        ]
                    }
                }
            }
            NurbsFrustumWorkflowOperation::Partition { input, cuts, .. } => {
                let shape = inputs[input.as_str()];
                if shape.components().len() != 1 {
                    return Err(diagnostic(
                        "unsupported",
                        "ambiguous_components",
                        Some(id),
                        "Partition requires one explicitly selected component.",
                    ));
                }
                match &shape.components()[0] {
                    NurbsFrustumWorkflowComponent::Canonical(body) => body.split_axial_many(cuts,tol)
                        .map_err(|e| geometry(e,id))?.parts.into_iter()
                        .map(|v| NurbsFrustumWorkflowComponent::Canonical(Arc::new(v))).collect(),
                    NurbsFrustumWorkflowComponent::Bored(body) => body.split_axial_many(cuts,tol)
                        .map_err(|e| geometry(e,id))?.parts.into_iter()
                        .map(|v| NurbsFrustumWorkflowComponent::Bored(Arc::new(v))).collect(),
                    NurbsFrustumWorkflowComponent::Oblique(_) | NurbsFrustumWorkflowComponent::BoredOblique(_) | NurbsFrustumWorkflowComponent::Slab(_) | NurbsFrustumWorkflowComponent::BoredSlab(_) => return Err(diagnostic(
                        "unsupported", "canonical_required", Some(id),
                        "Partition requires a canonical or bored frustum; oblique components cannot be repartitioned.")),
                }
            }
        };
        for b in &bodies {
            b.validate(tol).map_err(|e| geometry(e, id))?;
            b.volume(tol).map_err(|e| geometry(e, id))?;
        }
        Ok(NurbsFrustumWorkflowShape::Components(bodies))
    }
    pub fn rebuild(&self) -> WorkflowResult<NurbsFrustumWorkflowShape> {
        let tol = self.checked()?;
        let mut nodes = vec![];
        for i in 0..self.operations.len() {
            nodes.push(Arc::new(self.apply(i, &nodes, tol)?));
        }
        let index = self
            .operations
            .iter()
            .position(|o| o.id() == self.output)
            .ok_or_else(|| {
                diagnostic(
                    "invalid_input",
                    "invalid_output",
                    None,
                    "Output unavailable.",
                )
            })?;
        Ok(nodes[index].as_ref().clone())
    }
}

/// Accepted node snapshots share actual immutable B-reps. Candidate callbacks
/// must finish display/export validation before any cache/history state commits.
#[derive(Default)]
pub struct NurbsFrustumWorkflowSession {
    accepted: Option<NurbsFrustumWorkflowDocument>,
    nodes: Vec<Arc<NurbsFrustumWorkflowShape>>,
    undo: Vec<NurbsFrustumWorkflowDocument>,
    redo: Vec<NurbsFrustumWorkflowDocument>,
}
impl NurbsFrustumWorkflowSession {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn accepted_document(&self) -> Option<&NurbsFrustumWorkflowDocument> {
        self.accepted.as_ref()
    }
    pub fn prefix_snapshot(&self, index: usize) -> Option<&Arc<NurbsFrustumWorkflowShape>> {
        self.nodes.get(index)
    }
    pub fn accepted_shape(&self) -> Option<&Arc<NurbsFrustumWorkflowShape>> {
        let doc = self.accepted.as_ref()?;
        let index = doc.operations.iter().position(|op| op.id() == doc.output)?;
        self.nodes.get(index)
    }
    /// Query one explicitly indexed accepted output component without replay,
    /// display generation, cache replacement or history mutation.
    pub fn classify_point(&self, component: usize, point: Point3) -> WorkflowResult<PointLocation> {
        let doc = self.accepted.as_ref().ok_or_else(|| {
            diagnostic(
                "invalid_input",
                "no_accepted_document",
                None,
                "Accept a frustum workflow before querying a component.",
            )
        })?;
        if !point.finite() {
            return Err(diagnostic(
                "invalid_input",
                "invalid_query",
                Some(&doc.output),
                "Point query coordinates must be finite.",
            ));
        }
        let shape = self.accepted_shape().ok_or_else(|| {
            diagnostic(
                "validation_failed",
                "output_unavailable",
                Some(&doc.output),
                "Accepted output snapshot is unavailable.",
            )
        })?;
        let body = shape.components().get(component).ok_or_else(|| {
            diagnostic(
                "invalid_input",
                "invalid_component",
                Some(&doc.output),
                "Point query component index is unavailable in the accepted output.",
            )
        })?;
        body.classify_point(point, doc.geometry_tol()?)
            .map_err(|e| geometry(e, &doc.output))
    }
    fn prepare(
        &self,
        doc: &NurbsFrustumWorkflowDocument,
    ) -> WorkflowResult<(
        NurbsFrustumWorkflowRebuild,
        Vec<Arc<NurbsFrustumWorkflowShape>>,
    )> {
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
            NurbsFrustumWorkflowRebuild {
                shape: nodes[output].clone(),
                stats: NurbsFrustumWorkflowRebuildStats {
                    evaluated_nodes: doc.operations.len() - reused,
                    reused_nodes: reused,
                },
            },
            nodes,
        ))
    }
    pub fn rebuild_with<T, F>(
        &mut self,
        doc: &NurbsFrustumWorkflowDocument,
        validate: F,
    ) -> WorkflowResult<(NurbsFrustumWorkflowRebuild, T)>
    where
        F: FnOnce(&NurbsFrustumWorkflowShape, &NurbsFrustumWorkflowDocument) -> Result<T>,
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
    /// selected component, subject to an aggregate triangle budget.
    pub fn rebuild(
        &mut self,
        doc: &NurbsFrustumWorkflowDocument,
    ) -> WorkflowResult<NurbsFrustumWorkflowRebuild> {
        self.rebuild_with(doc, Self::validate_output)
            .map(|(report, ())| report)
    }
    fn validate_output(
        shape: &NurbsFrustumWorkflowShape,
        doc: &NurbsFrustumWorkflowDocument,
    ) -> Result<()> {
        let tol = GeometryTolerance::new(
            doc.tolerance.linear,
            doc.tolerance.angular,
            doc.tolerance.relative,
        )?;
        let mut triangles = 0usize;
        for body in shape.components() {
            let mesh = body.tessellate(doc.display_chord_tolerance, tol)?;
            triangles = triangles
                .checked_add(mesh.triangles.len())
                .ok_or(Error::Unsupported(
                    "frustum workflow aggregate display triangle budget exceeded",
                ))?;
            if triangles > NURBS_FRUSTUM_WORKFLOW_MAX_DISPLAY_TRIANGLES {
                return Err(Error::Unsupported(
                    "frustum workflow aggregate display triangle budget exceeded",
                ));
            }
            body.export_step_mm(tol)?;
        }
        Ok(())
    }
    pub fn undo_with<T, F>(
        &mut self,
        validate: F,
    ) -> WorkflowResult<(NurbsFrustumWorkflowRebuild, T)>
    where
        F: FnOnce(&NurbsFrustumWorkflowShape, &NurbsFrustumWorkflowDocument) -> Result<T>,
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
    pub fn redo_with<T, F>(
        &mut self,
        validate: F,
    ) -> WorkflowResult<(NurbsFrustumWorkflowRebuild, T)>
    where
        F: FnOnce(&NurbsFrustumWorkflowShape, &NurbsFrustumWorkflowDocument) -> Result<T>,
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
    pub fn undo(&mut self) -> WorkflowResult<NurbsFrustumWorkflowRebuild> {
        self.undo_with(Self::validate_output)
            .map(|(report, ())| report)
    }
    pub fn redo(&mut self) -> WorkflowResult<NurbsFrustumWorkflowRebuild> {
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
