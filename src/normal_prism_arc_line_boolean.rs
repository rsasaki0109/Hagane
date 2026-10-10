//! Scoped exact Boolean bodies from normal line/quarter-circle prism arrangements.
use crate::*;

#[derive(Clone, Debug)]
pub struct NormalPrismArcLineBoolean {
    difference: Vec<Solid>,
    intersection: Vec<Solid>,
    union: Vec<Solid>,
}
impl NormalPrismArcLineBoolean {
    pub fn difference(&self) -> &[Solid] {
        &self.difference
    }
    pub fn intersection(&self) -> &[Solid] {
        &self.intersection
    }
    pub fn union(&self) -> &[Solid] {
        &self.union
    }
    pub fn into_solids(self) -> (Vec<Solid>, Vec<Solid>, Vec<Solid>) {
        (self.difference, self.intersection, self.union)
    }
}
/// Return all exact difference, intersection and union components of certified
/// normal prisms with the same physical axis and cap interval. The source may
/// have at most sixteen holes; the simple tool has no holes. Input curves must
/// be finite lines or quarter/smaller arcs. Periodic circles, co-circle overlays,
/// contacts, tangencies, vertex hits and unresolved arrangements are unsupported.
pub fn boolean_normal_arc_line_prisms(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismArcLineBoolean> {
    let (difference, intersection, union) = crate::normal_prism_convex_partition::partition_engine(
        source, tool, axis, tolerance, true,
    )?;
    Ok(NormalPrismArcLineBoolean {
        difference,
        intersection,
        union,
    })
}
