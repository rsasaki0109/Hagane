//! One exact blind circular cavity grafted into an unchanged certified normal prism.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalPrismBoreEntry {
    /// Entry cap whose outward normal follows the signed supplied extrusion axis.
    Positive,
    /// Entry cap whose outward normal opposes the signed supplied extrusion axis.
    Negative,
}
#[derive(Clone, Debug)]
pub struct NormalPrismBlindBore {
    kept: Solid,
    removed: Solid,
    hole_faces: [usize; 4],
    floor_face: usize,
    direct_removed_volume: f64,
}
impl NormalPrismBlindBore {
    pub fn kept(&self) -> &Solid {
        &self.kept
    }
    pub fn removed(&self) -> &Solid {
        &self.removed
    }
    pub fn hole_faces(&self) -> &[usize; 4] {
        &self.hole_faces
    }
    pub fn floor_face(&self) -> usize {
        self.floor_face
    }
    pub fn direct_removed_volume(&self) -> f64 {
        self.direct_removed_volume
    }
    pub fn into_solids(self) -> (Solid, Solid) {
        (self.kept, self.removed)
    }
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn unresolved() -> Error {
    Error::Unsupported("normal prism blind bore arithmetic or cap identity is unresolved")
}
fn axis_direction(axis: Vec3) -> Result<Vec3> {
    let scale = axis.x.abs().max(axis.y.abs()).max(axis.z.abs());
    if !axis.finite() || !scale.is_finite() || scale == 0. {
        return Err(Error::InvalidInput(
            "blind bore axis must be finite and nonzero",
        ));
    }
    Vec3::new(axis.x / scale, axis.y / scale, axis.z / scale).normalized()
}
/// Create one normal blind bore from the cap outward toward `entry` along the
/// signed supplied axis. `center` must lie on that cap within the reserved
/// physical precision budget. Existing disjoint through openings are preserved.
/// Full-circle edges are outside the certified line/quarter-arc source domain.
/// The result is deliberately outside the normal-prism operation/import domain.
pub fn blind_bore_normal_prism(
    source: &Solid,
    center: Point3,
    radius: f64,
    depth: f64,
    extrusion_axis: Vec3,
    entry: NormalPrismBoreEntry,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismBlindBore> {
    let tol = tolerance.absolute();
    source.validate(tol)?;
    if !center.finite() || !radius.is_finite() || radius <= 0. || !depth.is_finite() || depth <= 0.
    {
        return Err(Error::InvalidInput(
            "blind bore center, positive radius and depth must be finite",
        ));
    }
    let axis = axis_direction(extrusion_axis)?;
    let desired = axis
        * if entry == NormalPrismBoreEntry::Positive {
            1.
        } else {
            -1.
        };
    let certified =
        crate::arc_line_prism_validation::recognize_validated_normal_prism(source, axis, tol)?;
    let scale = length(source.bounds().max - source.bounds().min);
    let band = tolerance.length_at_scale(scale)?;
    let reserve = tolerance.linear() / 32.;
    let mut world = scale
        .max(radius)
        .max(depth)
        .max(center.x.abs())
        .max(center.y.abs())
        .max(center.z.abs());
    for vertex in &source.vertices {
        world = world
            .max(vertex.point.x.abs())
            .max(vertex.point.y.abs())
            .max(vertex.point.z.abs());
    }
    for face in &source.shell.faces {
        let origin = match face.surface {
            Surface::Plane { origin, .. } => origin,
            Surface::FramedCylinder { frame, .. } => frame.origin(),
            _ => {
                return Err(Error::Unsupported(
                    "blind bore supports certified planar/cylindrical prism faces",
                ))
            }
        };
        world = world
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
    }
    let mut arithmetic = 4096. * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= tolerance.linear() / 256. {
        return Err(unresolved());
    }
    let margin = 10. * band + arithmetic;
    if radius <= margin || depth <= margin || certified.height - depth <= margin {
        return Err(Error::InvalidInput(
            "blind bore radius, depth and remaining floor thickness must be resolved",
        ));
    }
    let mut caps = Vec::new();
    for (index, face) in source.shell.faces.iter().enumerate() {
        if let Surface::Plane { origin, u, v } = face.surface {
            let normal = u.cross(v).normalized()? * f64::from(face.orientation);
            if length(normal - desired) * scale < reserve {
                let local = certified.frame.local_point(origin);
                if local.z.abs().min((local.z - certified.height).abs()) >= reserve {
                    return Err(unresolved());
                }
                caps.push(index);
            }
        }
    }
    if caps.len() != 1 {
        return Err(unresolved());
    }
    let cap_index = caps[0];
    let cap = &source.shell.faces[cap_index];
    let Surface::Plane { u, v, .. } = cap.surface else {
        unreachable!()
    };
    if (length(u) - 1.).abs() * scale >= reserve
        || (length(v) - 1.).abs() * scale >= reserve
        || u.dot(v).abs() * scale >= reserve
    {
        return Err(Error::Unsupported(
            "blind bore requires resolved orthonormal cap parameter frames",
        ));
    }
    let uv = cap.surface.try_parameters(center)?;
    let projected = cap.surface.try_evaluate(uv[0], uv[1])?;
    let floor_point = projected - desired * depth;
    if !projected.finite() || !floor_point.finite() {
        return Err(unresolved());
    }
    for point in [projected, floor_point] {
        world = world
            .max(point.x.abs())
            .max(point.y.abs())
            .max(point.z.abs());
    }
    arithmetic = 4096. * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= tolerance.linear() / 256. {
        return Err(unresolved());
    }
    let margin = 10. * band + arithmetic;
    if radius <= margin || depth <= margin || certified.height - depth <= margin {
        return Err(unresolved());
    }
    if length(projected - center) + arithmetic >= reserve {
        return Err(Error::InvalidInput(
            "blind bore center must lie on the selected entry cap",
        ));
    }
    // Reuse the stricter actual-region certificate and disk-clearance operation.
    // Its reconstructed result is never substituted for the original source.
    let _clearance = bore_normal_prism(source, projected, radius, axis, tolerance)?;
    let frame = Frame3::new(projected, [u, v, u.cross(v)], tol)?;
    if length(frame.axes()[0] - u) * radius + length(frame.axes()[1] - v) * radius + arithmetic
        >= reserve
    {
        return Err(unresolved());
    }
    let circle = (0..4)
        .map(|i| PlanarSegment::Arc {
            center: [0., 0.],
            radius,
            start_angle: f64::from(i) * std::f64::consts::FRAC_PI_2,
            sweep: std::f64::consts::FRAC_PI_2,
        })
        .collect();
    let tool = extrude_arc_line_region_in_frame(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: circle,
            holes: vec![],
        },
        desired * (-depth),
        frame,
        tol,
    )?;
    tool.validate(tol)?;
    let mut tool_entries = Vec::new();
    for (index, face) in tool.shell.faces.iter().enumerate() {
        if let Surface::Plane { origin, u, v } = face.surface {
            let outward = u.cross(v).normalized()? * f64::from(face.orientation);
            if length(outward - desired) * scale < reserve && length(origin - projected) < reserve {
                tool_entries.push(index);
            }
        }
    }
    if tool_entries.len() != 1 {
        return Err(unresolved());
    }
    let tool_entry = tool_entries[0];
    if tool.shell.faces[tool_entry].wires.len() != 1 {
        return Err(unresolved());
    }
    let mut kept = source.clone();
    let vertex_offset = kept.vertices.len();
    let edge_offset = kept.edges.len();
    kept.vertices.extend(tool.vertices.iter().cloned());
    kept.edges.extend(tool.edges.iter().cloned().map(|mut e| {
        e.vertices = e.vertices.map(|i| i + vertex_offset);
        e
    }));
    let mut inner = Vec::new();
    for coedge in tool.shell.faces[tool_entry].wires[0].coedges.iter().rev() {
        let edge = &tool.edges[coedge.edge];
        let Curve::Arc {
            frame,
            radius,
            sweep,
        } = edge.curve
        else {
            return Err(unresolved());
        };
        let c = cap.surface.try_parameters(frame.origin())?;
        let start = cap.surface.try_parameters(edge.curve.try_evaluate(0.)?)?;
        let phase = (start[1] - c[1]).atan2(start[0] - c[0]);
        let pcurve = PCurve::Arc {
            center: c,
            radius,
            start_angle: phase,
            sweep,
        };
        // Circle coefficient identity bounds the entire parameter interval,
        // rather than relying on finitely sampled witnesses.
        let cosine = u * (radius * phase.cos()) + v * (radius * phase.sin());
        let sine = u * (-radius * phase.sin()) + v * (radius * phase.cos());
        let identity_bound = length(cap.surface.try_evaluate(c[0], c[1])? - frame.origin())
            + length(cosine - frame.axes()[0] * radius)
            + length(sine - frame.axes()[1] * radius)
            + arithmetic;
        if !identity_bound.is_finite() || identity_bound >= reserve {
            return Err(unresolved());
        }
        for j in 0..=16 {
            let t = sweep * f64::from(j) / 16.;
            let uv = pcurve.try_evaluate(t)?;
            if length(cap.surface.try_evaluate(uv[0], uv[1])? - edge.curve.try_evaluate(t)?)
                + arithmetic
                >= reserve
            {
                return Err(unresolved());
            }
        }
        inner.push(Coedge {
            edge: coedge.edge + edge_offset,
            forward: !coedge.forward,
            pcurve,
        });
    }
    kept.shell.faces[cap_index]
        .wires
        .push(Wire { coedges: inner });
    let mut walls = Vec::new();
    let mut floor = None;
    for (index, face) in tool.shell.faces.iter().enumerate() {
        if index == tool_entry {
            continue;
        }
        let mut face = face.clone();
        face.orientation = -face.orientation;
        for wire in &mut face.wires {
            for coedge in &mut wire.coedges {
                coedge.edge += edge_offset;
            }
        }
        let id = kept.shell.faces.len();
        match face.surface {
            Surface::FramedCylinder { .. } => walls.push(id),
            Surface::Plane { .. } => {
                if floor.replace(id).is_some() {
                    return Err(unresolved());
                }
            }
            _ => return Err(unresolved()),
        }
        kept.shell.faces.push(face);
    }
    let hole_faces: [usize; 4] = walls.try_into().map_err(|_| unresolved())?;
    let floor_face = floor.ok_or_else(unresolved)?;
    kept.validate(tol)?;
    let direct_removed_volume = std::f64::consts::PI * radius * radius * depth;
    let before = source.volume()?;
    let after = kept.volume()?;
    let removed = tool.volume()?;
    let own_budget = 8192. * f64::EPSILON * direct_removed_volume.abs().max(removed.abs());
    let sum_budget = 8192. * f64::EPSILON * before.abs().max(after.abs()).max(removed.abs());
    if !direct_removed_volume.is_finite()
        || direct_removed_volume < f64::MIN_POSITIVE
        || !before.is_finite()
        || !after.is_finite()
        || !removed.is_finite()
        || after <= 0.
        || removed <= 0.
        || !own_budget.is_finite()
        || !sum_budget.is_finite()
        || (removed - direct_removed_volume).abs() > own_budget
        || (after + removed - before).abs() > sum_budget
    {
        return Err(unresolved());
    }
    Ok(NormalPrismBlindBore {
        kept,
        removed: tool,
        hole_faces,
        floor_face,
        direct_removed_volume,
    })
}
