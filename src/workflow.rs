//! Versioned, deliberately scoped editable modeling intent, not B-rep interchange.
use crate::operations::{
    apply_checked_prism_bore, checked_polygon_region_prism_stock, checked_skew_prism_bore_tools,
    swept_polygon_region_bore_clearance,
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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<Vec<[f64; 2]>>,
        height: f64,
        #[serde(default, skip_serializing_if = "zero_offset")]
        offset: [f64; 2],
    },
    Box {
        id: String,
        size: [f64; 3],
    },
    ArcLineExtrusion {
        id: String,
        outer: Vec<WorkflowProfileSegment>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<Vec<WorkflowProfileSegment>>,
        height: f64,
    },
    RoundedBox {
        id: String,
        size: [f64; 3],
        corner_radius: f64,
    },
    PlaneSplit {
        id: String,
        input: String,
        offset: f64,
        normal_angle: f64,
        side: WorkflowSplitSide,
    },
    Bore {
        #[serde(default, skip_serializing_if = "WorkflowBoreEntry::is_top")]
        entry: WorkflowBoreEntry,
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
pub enum WorkflowSplitSide {
    Negative,
    Positive,
}
/// Exact profile intent in millimetres and radians. Signed arcs retain direction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkflowProfileSegment {
    Line {
        start: [f64; 2],
        end: [f64; 2],
    },
    Arc {
        center: [f64; 2],
        radius: f64,
        start_angle: f64,
        sweep: f64,
    },
}
impl WorkflowProfileSegment {
    fn segment(&self, tol: Tolerance) -> Result<PlanarSegment> {
        match self {
            Self::Line { start, end } if start.iter().chain(end).all(|v| v.is_finite()) => Ok(PlanarSegment::Line { a: *start, b: *end }),
            Self::Arc { center, radius, start_angle, sweep } if center.iter().all(|v| v.is_finite()) && radius.is_finite() && *radius > 10.*tol.linear && start_angle.is_finite() && sweep.is_finite() && sweep.abs()>0. && sweep.abs()<=std::f64::consts::FRAC_PI_2 => Ok(PlanarSegment::Arc { center:*center,radius:*radius,start_angle:*start_angle,sweep:*sweep }),
            _ => Err(Error::InvalidInput("line/arc profile needs finite coordinates and positive resolved radii, with nonzero signed sweeps of at most pi/2")),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBoreMode {
    Through,
    Blind,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBoreEntry {
    #[default]
    Top,
    Bottom,
}
impl WorkflowBoreEntry {
    fn is_top(&self) -> bool {
        *self == Self::Top
    }
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
fn zero_offset(offset: &[f64; 2]) -> bool {
    *offset == [0., 0.]
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
        for (i, step) in plan.steps.iter().enumerate() {
            plan.apply_step(&mut solid, step)
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
            | WorkflowOperation::RoundedBox { id, .. }
            | WorkflowOperation::ArcLineExtrusion { id, .. }
            | WorkflowOperation::Extrusion { id, .. }
            | WorkflowOperation::Bore { id, .. }
            | WorkflowOperation::PlaneSplit { id, .. } => id,
        }
    }
    fn normal_prism_operations(&self) -> bool {
        matches!(
            self.operations.first(),
            Some(WorkflowOperation::RoundedBox { .. } | WorkflowOperation::ArcLineExtrusion { .. })
        ) || self
            .operations
            .iter()
            .any(|op| matches!(op, WorkflowOperation::PlaneSplit { .. }))
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
                | WorkflowOperation::RoundedBox { id, .. }
                | WorkflowOperation::ArcLineExtrusion { id, .. }
                | WorkflowOperation::Extrusion { id, .. }
                | WorkflowOperation::Bore { id, .. }
                | WorkflowOperation::PlaneSplit { id, .. } => id,
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
        let rounded_radius = match &self.operations[0] {
            WorkflowOperation::RoundedBox { corner_radius, .. } => Some(*corner_radius),
            _ => None,
        };
        let bore_count = self
            .operations
            .iter()
            .filter(|op| matches!(op, WorkflowOperation::Bore { .. }))
            .count();
        let split_count = self
            .operations
            .iter()
            .filter(|op| matches!(op, WorkflowOperation::PlaneSplit { .. }))
            .count();
        if rounded_radius.is_some()
            && (bore_count > 16 || 8 + 4 * bore_count + 3 * split_count > 128)
        {
            let mut d = diagnostic(
                "unsupported_history",
                None,
                Some("operations"),
                "Rounded-box histories support at most 16 bores and 128 profile segments, reserving 8 for stock, 4 per bore and 3 per plane split.",
            );
            d.category = "unsupported";
            return Err(d);
        }
        let arc_profile = if let WorkflowOperation::ArcLineExtrusion {
            id,
            outer,
            holes,
            height,
        } = &self.operations[0]
        {
            let total = outer.len() + holes.iter().map(Vec::len).sum::<usize>();
            let bores = bore_count;
            if holes.len() > 16
                || total > 128
                || holes.len() + bores > 16
                || total + 4 * bores + 3 * split_count > 128
            {
                let mut d=diagnostic("unsupported_history",Some(id),Some("outer_or_holes_or_operations"),"Line/arc histories support at most 16 total holes and 128 profile segments, reserving 4 per added bore and 3 per plane split.");
                d.category = "unsupported";
                return Err(d);
            }
            if !height.is_finite()
                || *height <= 10. * t.linear
                || outer.is_empty()
                || holes.iter().any(Vec::is_empty)
            {
                return Err(diagnostic(
                    "profile_rejected",
                    Some(id),
                    Some("outer_or_holes_or_height"),
                    "Line/arc extrusion needs nonempty rings and finite resolved positive height.",
                ));
            }
            let convert = |ring: &Vec<WorkflowProfileSegment>| {
                ring.iter()
                    .map(|s| s.segment(t))
                    .collect::<Result<Vec<_>>>()
            };
            let outer = convert(outer).map_err(|e| geometry_error(e, id))?;
            let holes = holes
                .iter()
                .map(convert)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| geometry_error(e, id))?;
            if !outer
                .iter()
                .chain(holes.iter().flatten())
                .any(|s| matches!(s, PlanarSegment::Arc { .. }))
            {
                let mut d=diagnostic("profile_rejected",Some(id),Some("outer_or_holes"),"Arc-line extrusion requires at least one circular arc; use extrusion for line-only profiles.");
                d.category = "unsupported";
                return Err(d);
            }
            Some(ArcLineRegion {
                origin: Point3::new(0., 0., -*height / 2.),
                outer,
                holes,
            })
        } else {
            None
        };
        let curved = rounded_radius.is_some() || arc_profile.is_some();
        let (box_id, b, profile) = match &self.operations[0] {
            WorkflowOperation::Box { id, size }
            | WorkflowOperation::RoundedBox { id, size, .. } => {
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
                if let Some(radius) = rounded_radius {
                    let band = policy
                        .length_at_scale(size.x.hypot(size.y).hypot(size.z))
                        .map_err(|e| geometry_error(e, id))?;
                    if !radius.is_finite()
                        || radius <= 10. * band
                        || size.x - 2. * radius <= 10. * band
                        || size.y - 2. * radius <= 10. * band
                    {
                        return Err(diagnostic("invalid_corner_radius",Some(id),Some("corner_radius"),"Corner radius must be positive and resolved, leaving resolved straight sides in both XY dimensions."));
                    }
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
            WorkflowOperation::ArcLineExtrusion { id, height, .. } => {
                let region = arc_profile.as_ref().ok_or_else(|| {
                    diagnostic(
                        "profile_rejected",
                        Some(id),
                        None,
                        "Missing line/arc profile.",
                    )
                })?;
                let mut min = [f64::INFINITY; 2];
                let mut max = [f64::NEG_INFINITY; 2];
                for segment in region.outer.iter().chain(region.holes.iter().flatten()) {
                    let bounds = match segment {
                        PlanarSegment::Line { a, b } => [*a, *b],
                        PlanarSegment::Arc { center, radius, .. } => [
                            [center[0] - radius, center[1] - radius],
                            [center[0] + radius, center[1] + radius],
                        ],
                    };
                    for p in bounds {
                        for axis in 0..2 {
                            min[axis] = min[axis].min(p[axis]);
                            max[axis] = max[axis].max(p[axis]);
                        }
                    }
                }
                let size = Vec3::new(max[0] - min[0], max[1] - min[1], *height);
                if !size.finite()
                    || !min.iter().all(|v| v.is_finite())
                    || size.x <= 0.
                    || size.y <= 0.
                {
                    return Err(diagnostic(
                        "profile_rejected",
                        Some(id),
                        Some("outer_or_holes"),
                        "Profile bounds exceed finite resolved arithmetic.",
                    ));
                }
                (
                    id,
                    BoxSpec {
                        min: Point3::new(min[0], min[1], -*height / 2.),
                        size,
                    },
                    None,
                )
            }
            WorkflowOperation::Extrusion {
                id,
                outer,
                holes,
                height,
                offset,
            } => {
                if offset.iter().any(|v| !v.is_finite()) {
                    return Err(diagnostic(
                        "invalid_offset",
                        Some(id),
                        Some("offset"),
                        "Extrusion offset must contain finite XY displacements.",
                    ));
                }
                let b =
                    checked_polygon_region_prism_stock(outer, holes, *height, t).map_err(|e| {
                        let mut d = geometry_error(e, id);
                        d.code = "profile_rejected";
                        d.field = Some(if holes.is_empty() {
                            "outer_or_height"
                        } else {
                            "outer_holes_or_height"
                        });
                        d
                    })?;
                (
                    id,
                    b,
                    Some(PolygonProfile {
                        origin: Point3::new(0., 0., -*height / 2.),
                        outer: outer.clone(),
                        holes: holes.clone(),
                    }),
                )
            }
            _ => {
                return Err(diagnostic(
                    "invalid_history",
                    None,
                    Some("operations"),
                    "The first operation must create a box, rounded box, polygon extrusion or arc-line extrusion.",
                ))
            }
        };
        let size = b.size;
        let direction = match &self.operations[0] {
            WorkflowOperation::Extrusion { offset, .. } => Vec3::new(offset[0], offset[1], size.z),
            _ => Vec3::new(0., 0., size.z),
        };
        if split_count > 0 {
            if direction.x != 0. || direction.y != 0. {
                let mut d = diagnostic(
                    "plane_split_normal_stock_required",
                    self.operations.iter().find_map(|op| match op {
                        WorkflowOperation::PlaneSplit { id, .. } => Some(id.as_str()),
                        _ => None,
                    }),
                    Some("offset"),
                    "Plane-cut histories require normal Z extrusion with zero XY offset.",
                );
                d.category = "unsupported";
                return Err(d);
            }
            if !curved {
                let (segments, holes) = profile.as_ref().map_or((4, 0), |profile| {
                    (
                        profile.outer.len() + profile.holes.iter().map(Vec::len).sum::<usize>(),
                        profile.holes.len(),
                    )
                });
                if holes + bore_count > 16
                    || segments + 2 > 128
                    || segments + 4 * bore_count + 3 * split_count > 128
                {
                    let mut d = diagnostic(
                        "unsupported_history", None, Some("operations"),
                        "Normal plane-cut histories support 16 initial openings plus bores, 128 source faces and 128 reserved profile segments.",
                    );
                    d.category = "unsupported";
                    return Err(d);
                }
            }
            let mut ids = vec![box_id.as_str()];
            let mut steps = Vec::new();
            for op in &self.operations[1..] {
                let (id, input) = match op {
                    WorkflowOperation::Bore { id, input, .. }
                    | WorkflowOperation::PlaneSplit { id, input, .. } => (id, input),
                    _ => return Err(diagnostic(
                        "unsupported_history",
                        None,
                        Some("operations"),
                        "Only through bores and plane splits may appear in a plane-cut history.",
                    )),
                };
                if ids.contains(&id.as_str()) || Some(input.as_str()) != ids.last().copied() {
                    return Err(diagnostic("invalid_reference",Some(id),Some("input"),"Operation ID must be unique and input must reference the immediately preceding operation."));
                }
                let step = match op {
                    WorkflowOperation::PlaneSplit {
                        offset,
                        normal_angle,
                        side,
                        ..
                    } => {
                        if !offset.is_finite() || !normal_angle.is_finite() {
                            return Err(diagnostic(
                                "invalid_plane_split",
                                Some(id),
                                Some("offset_or_normal_angle"),
                                "Plane offset and normal angle must be finite.",
                            ));
                        }
                        let (sin, cos) = normal_angle.sin_cos();
                        let plane = Surface::Plane {
                            origin: Point3::new(cos * offset, sin * offset, 0.),
                            u: Vec3::new(0., 0., 1.),
                            v: Vec3::new(sin, -cos, 0.),
                        };
                        WorkflowStep::PlaneSplit { plane, side: *side }
                    }
                    WorkflowOperation::Bore {
                        entry,
                        mode,
                        center,
                        radius,
                        depth,
                        ..
                    } => {
                        if *mode == WorkflowBoreMode::Blind {
                            let mut d = diagnostic(
                                if rounded_radius.is_some() {
                                    "rounded_blind_bore_unsupported"
                                } else if arc_profile.is_some() {
                                    "arc_line_blind_bore_unsupported"
                                } else {
                                    "normal_split_blind_bore_unsupported"
                                },
                                Some(id),
                                Some("mode"),
                                "Normal plane-cut histories support through bores only.",
                            );
                            d.category = "unsupported";
                            return Err(d);
                        }
                        if *entry != WorkflowBoreEntry::Top {
                            return Err(diagnostic(
                                "unexpected_entry",
                                Some(id),
                                Some("entry"),
                                "Through bores must omit entry or use top.",
                            ));
                        }
                        if depth.is_some() {
                            return Err(diagnostic(
                                "unexpected_depth",
                                Some(id),
                                Some("depth"),
                                "Through bores must omit depth.",
                            ));
                        }
                        if center.iter().any(|v| !v.is_finite())
                            || !radius.is_finite()
                            || *radius <= 10. * t.linear
                        {
                            return Err(diagnostic(
                                "invalid_bore_parameters",
                                Some(id),
                                Some("radius_or_center"),
                                "Bore center and radius must be finite and resolved.",
                            ));
                        }
                        WorkflowStep::Bore {
                            bore: BoxBore {
                                center: *center,
                                radius: *radius,
                                depth: None,
                            },
                            entry: *entry,
                            tool: None,
                        }
                    }
                    _ => {
                        return Err(diagnostic(
                            "unsupported_history",
                            Some(id),
                            None,
                            "Unsupported operation in split history.",
                        ))
                    }
                };
                steps.push(step);
                ids.push(id.as_str());
            }
            return Ok(WorkflowPlan {
                stock: b,
                direction,
                profile,
                steps,
                tolerance: t,
                rounded_radius,
                arc_profile,
                policy,
            });
        }
        let mut bores: Vec<BoxBore> = Vec::new();
        let mut entries = Vec::new();
        let mut ids = vec![box_id.as_str()];
        for operation in &self.operations[1..] {
            let WorkflowOperation::Bore {
                id,
                input,
                entry,
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
            if *mode == WorkflowBoreMode::Through && *entry != WorkflowBoreEntry::Top {
                return Err(diagnostic(
                    "unexpected_entry",
                    Some(id),
                    Some("entry"),
                    "Through bores must omit entry or use top; entry only selects blind cuts.",
                ));
            }
            if curved
                && *mode == WorkflowBoreMode::Through
                && bores.iter().any(|b| b.depth.is_some())
            {
                let mut d = diagnostic(
                    "curved_through_after_blind_unsupported",
                    Some(id),
                    Some("mode"),
                    "Through bores after blind machining are unsupported on curved line/arc stock.",
                );
                d.category = "unsupported";
                return Err(d);
            }
            if *mode == WorkflowBoreMode::Blind {
                let valid_depth =
                    depth
                        .filter(|v| v.is_finite() && *v > 10. * t.linear)
                        .ok_or_else(|| {
                            diagnostic("invalid_depth",Some(id),Some("depth"),
                        "Blind bores require finite depth greater than ten linear tolerances.")
                        })?;
                if size.z - valid_depth <= 10. * t.linear {
                    let mut d = diagnostic(
                        "floor_thickness",
                        Some(id),
                        Some("depth"),
                        "The requested blind depth breaks through or leaves an unresolved floor.",
                    );
                    d.category = "unsupported";
                    d.measured_clearance = Some(size.z - valid_depth);
                    d.required_clearance = Some(10. * t.linear);
                    d.suggestion =
                        Some("Reduce depth or increase height to retain a resolved floor.".into());
                    return Err(d);
                }
            }
            if arc_profile.is_none() {
                let (clearance, profile_hole) = if let Some(profile) = &profile {
                    swept_polygon_region_bore_clearance(
                        &profile.outer,
                        &profile.holes,
                        *center,
                        *radius,
                        [direction.x, direction.y],
                        size.z,
                        if *mode == WorkflowBoreMode::Blind {
                            let fraction = depth.unwrap() / size.z;
                            if *entry == WorkflowBoreEntry::Bottom {
                                [0., fraction]
                            } else {
                                [1. - fraction, 1.]
                            }
                        } else {
                            [0., 1.]
                        },
                    )
                    .map_err(|e| geometry_error(e, id))?
                } else {
                    (
                        (size.x / 2. - center[0].abs()).min(size.y / 2. - center[1].abs()) - radius,
                        None,
                    )
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
                    if let Some(index) = profile_hole {
                        d.code = "profile_hole_clearance";
                        d.message=format!("Circular tool reaches or lies inside profile hole {index} (zero-based index).");
                    }
                    d.category = "unsupported";
                    d.measured_clearance = Some(clearance);
                    d.required_clearance = Some(required);
                    d.suggestion=Some("Reduce the radius, move the center inward, or enlarge the stock until clearance exceeds the required margin.".into());
                    if profile_hole.is_some() {
                        d.suggestion=Some("Move the center away from the profile opening, reduce radius, or edit the opening until clearance exceeds the required margin.".into());
                    }
                    return Err(d);
                }
            }
            let required = 10. * t.linear;
            for ((previous, previous_id), previous_entry) in
                bores.iter().zip(&ids[1..]).zip(&entries)
            {
                let radial_gap = (center[0] - previous.center[0])
                    .hypot(center[1] - previous.center[1])
                    - radius
                    - previous.radius;
                let opposite = !curved
                    && *mode == WorkflowBoreMode::Blind
                    && previous.depth.is_some()
                    && entry != previous_entry;
                let axial_gap = if opposite {
                    size.z - depth.unwrap() - previous.depth.unwrap()
                } else {
                    f64::NEG_INFINITY
                };
                let gap = radial_gap.max(axial_gap);
                if !radial_gap.is_finite() || gap <= required {
                    let mut d = diagnostic(
                        if opposite {
                            "bore_web_thickness"
                        } else {
                            "bore_clearance"
                        },
                        Some(id),
                        Some(if opposite {
                            "depth_or_center_or_radius"
                        } else {
                            "center_or_radius"
                        }),
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
                        if opposite { "Reduce opposing depths to retain a resolved web, or separate the XY footprints." }
                        else { "Move the center or reduce radii so the XY footprints are separated." }.into(),
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
                    bores.push(BoxBore {
                        center: *center,
                        radius: *radius,
                        depth: *depth,
                    });
                }
            }
            entries.push(*entry);
            ids.push(id.as_str());
        }
        // Legacy planar pairs use actual entry/depth intervals. Curved stock
        // instead requires disjoint XY footprints on both entry sides.
        let tools = if curved {
            vec![None; bores.len()]
        } else {
            bores
                .iter()
                .zip(&ids[1..])
                .map(|(bore, id)| {
                    checked_skew_prism_bore_tools(b, [direction.x, direction.y], &[*bore], t)
                        .map(|tools| Some(tools[0]))
                        .map_err(|e| geometry_error(e, id))
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(WorkflowPlan {
            stock: b,
            direction,
            profile,
            steps: bores
                .into_iter()
                .zip(entries)
                .zip(tools)
                .map(|((bore, entry), tool)| WorkflowStep::Bore { bore, entry, tool })
                .collect(),
            tolerance: t,
            rounded_radius,
            arc_profile,
            policy,
        })
    }
    fn candidate_segments(&self, failed: Option<&str>) -> Vec<[[f64; 3]; 2]> {
        let height = match self.operations.first() {
            Some(
                WorkflowOperation::Box { size, .. } | WorkflowOperation::RoundedBox { size, .. },
            ) => size[2],
            Some(
                WorkflowOperation::Extrusion { height, .. }
                | WorkflowOperation::ArcLineExtrusion { height, .. },
            ) => *height,
            _ => return vec![],
        };
        let size = [height; 3];
        let Some(WorkflowOperation::Bore {
            center,
            radius,
            depth,
            mode,
            entry,
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
        let upper = size[2] / 2.;
        let (bottom, top) = if *mode == WorkflowBoreMode::Blind {
            if *entry == WorkflowBoreEntry::Bottom {
                (-upper, -upper + depth.unwrap_or(size[2]))
            } else {
                (upper - depth.unwrap_or(size[2]), upper)
            }
        } else {
            (-upper, upper)
        };
        if !bottom.is_finite() || !top.is_finite() || bottom.abs() > 1e6 || top.abs() > 1e6 {
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

enum WorkflowStep {
    Bore {
        bore: BoxBore,
        entry: WorkflowBoreEntry,
        tool: Option<CylinderSpec>,
    },
    PlaneSplit {
        plane: Surface,
        side: WorkflowSplitSide,
    },
}
struct WorkflowPlan {
    stock: BoxSpec,
    direction: Vec3,
    profile: Option<PolygonProfile>,
    steps: Vec<WorkflowStep>,
    tolerance: Tolerance,
    rounded_radius: Option<f64>,
    arc_profile: Option<ArcLineRegion>,
    policy: GeometryTolerance,
}
impl WorkflowPlan {
    fn apply_step(&self, solid: &mut Solid, step: &WorkflowStep) -> Result<()> {
        let normal = self.rounded_radius.is_some()
            || self.arc_profile.is_some()
            || self
                .steps
                .iter()
                .any(|s| matches!(s, WorkflowStep::PlaneSplit { .. }));
        match step {
            WorkflowStep::PlaneSplit { plane, side } => {
                let (negative, positive) = split_normal_prism_by_plane_components(
                    solid,
                    plane,
                    Vec3::new(0., 0., 1.),
                    self.policy,
                )?
                .into_solids();
                let mut selected = match side {
                    WorkflowSplitSide::Negative => negative,
                    WorkflowSplitSide::Positive => positive,
                };
                if selected.len() != 1 {
                    return Err(Error::Unsupported(
                        "workflow plane split requires exactly one connected component on the selected side",
                    ));
                }
                *solid = selected.pop().ok_or(Error::InvalidTopology(
                    "workflow plane split lost its selected component",
                ))?;
                Ok(())
            }
            WorkflowStep::Bore { bore, entry, tool } => {
                if normal {
                    if let Some(depth) = bore.depth {
                        if self
                            .steps
                            .iter()
                            .any(|s| matches!(s, WorkflowStep::PlaneSplit { .. }))
                        {
                            return Err(Error::Unsupported(
                                "normal plane-cut histories support through bores only",
                            ));
                        }
                        let (z, side) = match entry {
                            WorkflowBoreEntry::Top => (
                                self.stock.min.z + self.stock.size.z,
                                NormalPrismBoreEntry::Positive,
                            ),
                            WorkflowBoreEntry::Bottom => {
                                (self.stock.min.z, NormalPrismBoreEntry::Negative)
                            }
                        };
                        *solid = append_blind_bores_normal_prism(
                            solid,
                            &[NormalPrismBlindBoreSpec {
                                center: Point3::new(bore.center[0], bore.center[1], z),
                                radius: bore.radius,
                                depth,
                                entry: side,
                            }],
                            Vec3::new(0., 0., 1.),
                            self.policy,
                        )?
                        .into_solids()
                        .0;
                        return Ok(());
                    }
                    *solid = bore_normal_prism(
                        solid,
                        Point3::new(bore.center[0], bore.center[1], self.stock.min.z),
                        bore.radius,
                        Vec3::new(0., 0., 1.),
                        self.policy,
                    )?
                    .into_solids()
                    .0;
                    Ok(())
                } else if *entry == WorkflowBoreEntry::Bottom {
                    crate::operations::append_bottom_blind_bore(solid, self.stock, *bore);
                    Ok(())
                } else {
                    apply_checked_prism_bore(
                        solid,
                        self.stock,
                        *bore,
                        tool.ok_or(Error::InvalidTopology("missing checked prism tool"))?,
                        self.tolerance,
                    )
                }
            }
        }
    }
    fn make_stock(&self) -> Result<Solid> {
        if let Some(radius) = self.rounded_radius {
            let stock = make_box(self.stock, self.tolerance)?;
            Ok(fillet_parallel_box_edges(
                &stock,
                &[8, 9, 10, 11].map(|i| (i, radius)),
                self.policy,
            )?
            .into_solid())
        } else if let Some(region) = &self.arc_profile {
            let stock = extrude_arc_line_region(region, self.direction.z, self.tolerance)?;
            let extent = stock.bounds().max - stock.bounds().min;
            let band = self
                .policy
                .length_at_scale(extent.x.hypot(extent.y).hypot(extent.z))?;
            if self.direction.z <= 10. * band {
                return Err(Error::InvalidInput(
                    "arc-line height is unresolved at effective tolerance",
                ));
            }
            let loops: Vec<_> = std::iter::once(&region.outer)
                .chain(&region.holes)
                .cloned()
                .collect();
            crate::mixed::validate_mixed_region(&loops, Tolerance::new(band)?)?;
            crate::arc_line_prism_validation::certify_validated_arc_line_prism(
                &stock,
                self.tolerance,
            )?;
            Ok(stock)
        } else if let Some(profile) = &self.profile {
            extrude_polygon(profile, self.direction, self.tolerance)
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
                let prefix = old
                    .operations
                    .iter()
                    .zip(&document.operations)
                    .take_while(|(a, b)| a == b)
                    .count();
                // A first/last cut switches legacy full-circle bore construction
                // to/from certified quarter arcs. Only the stock is shared
                // across these operation domains, despite identical bore intent.
                if old.normal_prism_operations() != document.normal_prism_operations() {
                    prefix.min(1)
                } else {
                    prefix
                }
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
            plan.apply_step(&mut solid, &plan.steps[index - 1])
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
