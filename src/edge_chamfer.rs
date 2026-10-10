//! Isolated equal-setback chamfers on strictly convex planar straight-edge solids.
use crate::*;

#[derive(Clone, Debug)]
pub struct PlanarEdgeChamfer {
    solid: Solid,
    removed: Solid,
    bevel: PlanarFacePatch,
    bevel_plane: Surface,
    source_edge: usize,
    setback: f64,
}
impl PlanarEdgeChamfer {
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn removed(&self) -> &Solid {
        &self.removed
    }
    pub fn bevel(&self) -> &PlanarFacePatch {
        &self.bevel
    }
    pub fn bevel_plane(&self) -> &Surface {
        &self.bevel_plane
    }
    /// Index in the input solid, not a persistent identifier in the result.
    pub fn source_edge(&self) -> usize {
        self.source_edge
    }
    pub fn setback(&self) -> f64 {
        self.setback
    }
    pub fn into_solid(self) -> Solid {
        self.solid
    }
}

/// Cut an isolated convex edge by a plane at equal physical setback on its two faces.
/// Curved faces, inner wires, coplanar subdivisions and interacting cuts are unsupported.
/// Floating-point precision guards are conservative engineering checks, not interval proofs.
pub fn chamfer_straight_convex_edge(
    source: &Solid,
    edge_index: usize,
    setback: f64,
    tol: GeometryTolerance,
) -> Result<PlanarEdgeChamfer> {
    planar_face_patches(source, tol.absolute())?;
    if !setback.is_finite() || setback <= 0.0 {
        return Err(Error::InvalidInput(
            "chamfer setback must be finite and positive",
        ));
    }
    let edge = source
        .edges
        .get(edge_index)
        .ok_or(Error::InvalidInput("chamfer edge index is out of range"))?;
    let bounds = source.bounds();
    let extent = bounds.max - bounds.min;
    let scale = extent.x.hypot(extent.y).hypot(extent.z);
    let band = tol.length_at_scale(scale)?;
    let world = source.vertices.iter().fold(scale.max(setback), |s, v| {
        s.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let world = source.shell.faces.iter().fold(world, |s, face| {
        if let Surface::Plane { origin, .. } = face.surface {
            s.max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs())
        } else {
            s
        }
    });
    let arithmetic = 4096.0 * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= tol.linear() / 4.0 {
        return Err(Error::Unsupported(
            "chamfer world-coordinate precision is unresolved",
        ));
    }
    let margin = 10.0 * band + arithmetic;
    let mut adjacent = Vec::new();
    let mut normals = Vec::new();
    for (fi, face) in source.shell.faces.iter().enumerate() {
        if face.wires.len() != 1 {
            return Err(Error::Unsupported(
                "chamfer requires faces without inner wires",
            ));
        }
        let Surface::Plane { origin, u, v } = face.surface else {
            return Err(Error::Unsupported("chamfer requires planar faces"));
        };
        let normal = u.cross(v).normalized()? * f64::from(face.orientation);
        let mut own = std::collections::BTreeSet::new();
        for c in &face.wires[0].coedges {
            let e = &source.edges[c.edge];
            own.extend(e.vertices);
            if c.edge == edge_index {
                adjacent.push(fi);
            }
        }
        for (vi, vertex) in source.vertices.iter().enumerate() {
            let d = (vertex.point - origin).dot(normal);
            if !d.is_finite() || (own.contains(&vi) && d.abs() > tol.linear() / 4.0) {
                return Err(Error::Unsupported(
                    "chamfer supporting plane precision is unresolved",
                ));
            }
            if !own.contains(&vi) && d >= -margin {
                return Err(Error::Unsupported("chamfer requires strict convex supporting planes without coplanar subdivisions"));
            }
        }
        normals.push(normal);
    }
    if adjacent.len() != 2 || adjacent[0] == adjacent[1] {
        return Err(Error::InvalidTopology(
            "chamfer edge must have two distinct incident faces",
        ));
    }
    let n1 = normals[adjacent[0]];
    let n2 = normals[adjacent[1]];
    let sum = n1 + n2;
    let sum_length = sum.x.hypot(sum.y).hypot(sum.z);
    let cross = n1.cross(n2);
    let sine = cross.x.hypot(cross.y).hypot(cross.z);
    if !sum_length.is_finite() || sum_length <= tol.angular().sin() || sine <= tol.angular().sin() {
        return Err(Error::Unsupported("chamfer dihedral angle is unresolved"));
    }
    let normal = sum.normalized()?;
    // sin(theta/2) = sin(theta) / |n1+n2| avoids subtracting nearly equal cosines.
    let offset = setback * (sine / sum_length);
    if !offset.is_finite() || offset <= margin {
        return Err(Error::Unsupported(
            "chamfer setback is unresolved at the requested tolerance",
        ));
    }
    let a = source.vertices[edge.vertices[0]].point;
    let b = source.vertices[edge.vertices[1]].point;
    let axis = (b - a).normalized()?;
    if axis.dot(normal).abs() > tol.angular().sin() {
        return Err(Error::Unsupported(
            "chamfer incident planes disagree with the selected edge",
        ));
    }
    let origin = a - normal * offset;
    let v = normal.cross(axis).normalized()?;
    let plane = Surface::Plane { origin, u: axis, v };
    Frame3::new(origin, [axis, v, axis.cross(v)], tol.absolute())?;
    for (vi, vertex) in source.vertices.iter().enumerate() {
        let d = (vertex.point - origin).dot(normal);
        let selected = edge.vertices.contains(&vi);
        if !d.is_finite() || (selected && d <= margin) || (!selected && d >= -margin) {
            return Err(Error::Unsupported(
                "chamfer would touch, collapse or interfere with another feature",
            ));
        }
    }
    let mut split = split_solid_by_plane(source, &plane, tol)?;
    if split.section.len() != 1 || split.section[0].rings.len() != 1 {
        return Err(Error::Unsupported(
            "chamfer section must be one simple face",
        ));
    }
    let bevel = split.section.pop().unwrap();
    // Each cut vertex lies on one of the incident faces and has distance d from the edge.
    for p in &bevel.rings[0] {
        let delta = *p - a;
        let transverse = delta - axis * delta.dot(axis);
        let distance = transverse.x.hypot(transverse.y).hypot(transverse.z);
        if !distance.is_finite() || (distance - setback).abs() > tol.linear() {
            return Err(Error::Unsupported(
                "chamfer equal setback loses physical precision",
            ));
        }
    }
    Ok(PlanarEdgeChamfer {
        solid: split.negative,
        removed: split.positive,
        bevel,
        bevel_plane: plane,
        source_edge: edge_index,
        setback,
    })
}
