//! Exact finite line/arc material-region Boolean operations on normal prisms.
use crate::*;

#[derive(Clone, Debug)]
pub struct NormalPrismRegionBoolean {
    difference: Vec<Solid>,
    intersection: Vec<Solid>,
    union: Vec<Solid>,
}
impl NormalPrismRegionBoolean {
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
/// Return every exact material component of difference, intersection and union.
/// Both certified normal prisms may have up to sixteen openings and must share
/// their physical axis and cap interval. Profiles contain finite lines and
/// quarter/smaller arcs. Contacts, tangencies, coincident circle supports,
/// vertex hits, periodic circles and unresolved arrangements are unsupported.
pub fn boolean_normal_prism_regions(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismRegionBoolean> {
    let (difference, intersection, union) =
        crate::normal_prism_convex_partition::region_partition_engine(
            source, tool, axis, tolerance, true, true,
        )?;
    Ok(NormalPrismRegionBoolean {
        difference,
        intersection,
        union,
    })
}
