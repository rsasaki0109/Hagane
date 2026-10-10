//! Transverse multiple bevel-plane intersections on convex planar solids.
use crate::*;

#[derive(Clone, Debug)]
pub struct PlanarEdgeChamfers {
    solid: Solid,
    removed: Vec<Solid>,
    bevels: Vec<PlanarFacePatch>,
    planes: Vec<Surface>,
    selections: Vec<(usize, f64)>,
}
impl PlanarEdgeChamfers {
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    /// Sequentially removed, mutually disjoint material interiors, in selection order.
    pub fn removed(&self) -> &[Solid] {
        &self.removed
    }
    /// Final retained bevel patches, in selection order, after all later cuts.
    pub fn bevels(&self) -> &[PlanarFacePatch] {
        &self.bevels
    }
    pub fn bevel_planes(&self) -> &[Surface] {
        &self.planes
    }
    /// Original input edge indices and physical setbacks.
    pub fn selections(&self) -> &[(usize, f64)] {
        &self.selections
    }
    pub fn into_solid(self) -> Solid {
        self.solid
    }
}
fn same_plane(a: &Surface, b: &Surface) -> bool {
    match (a, b) {
        (
            Surface::Plane {
                origin: a,
                u: au,
                v: av,
            },
            Surface::Plane {
                origin: b,
                u: bu,
                v: bv,
            },
        ) => a == b && au == bu && av == bv,
        _ => false,
    }
}
/// Apply original-edge equal-setback planes to successively retained material.
/// Each requested plane must individually isolate its input edge. Intersections
/// are supported when transverse to every intermediate vertex; equal adjacent
/// box setbacks, vertex contacts, vanished bevels and collapsing cuts are refused.
/// Selection order determines the decomposition of removed material, not the
/// mathematical retained intersection. No intermediate result is published on failure.
pub fn chamfer_straight_convex_edges(
    source: &Solid,
    selections: &[(usize, f64)],
    tol: GeometryTolerance,
) -> Result<PlanarEdgeChamfers> {
    chamfer_multiple(source, selections, tol, false)
}
/// Intersect bevel planes including guarded contacts with intermediate vertices.
/// Actual vertex positions are retained; shallow or ill-conditioned contacts are
/// rejected. This does not widen the existing transverse partition API.
pub fn chamfer_straight_convex_edges_with_vertex_contacts(
    source: &Solid,
    selections: &[(usize, f64)],
    tol: GeometryTolerance,
) -> Result<PlanarEdgeChamfers> {
    chamfer_multiple(source, selections, tol, true)
}
fn chamfer_multiple(
    source: &Solid,
    selections: &[(usize, f64)],
    tol: GeometryTolerance,
    contacts: bool,
) -> Result<PlanarEdgeChamfers> {
    if selections.is_empty() || selections.len() > 64 {
        return Err(Error::InvalidInput(
            "multi chamfer requires 1..64 original edge selections",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for &(edge, _) in selections {
        if !seen.insert(edge) {
            return Err(Error::InvalidInput(
                "multi chamfer edge selections must be unique",
            ));
        }
    }
    let mut planes = Vec::with_capacity(selections.len());
    for &(edge, setback) in selections {
        planes.push(
            chamfer_straight_convex_edge(source, edge, setback, tol)?
                .bevel_plane()
                .clone(),
        );
    }
    let mut solid = source.clone();
    let mut removed = Vec::with_capacity(planes.len());
    for plane in &planes {
        let split = if contacts {
            crate::convex_contact_split::split_convex_with_vertex_contacts(&solid, plane, tol)?
        } else {
            split_solid_by_plane(&solid, plane, tol)?
        };
        solid = split.negative;
        removed.push(split.positive);
    }
    let patches = planar_face_patches(&solid, tol.absolute())?;
    let mut bevels = Vec::with_capacity(planes.len());
    for plane in &planes {
        let matching: Vec<_> = patches
            .iter()
            .filter(|p| same_plane(&p.surface, plane))
            .collect();
        if matching.len() != 1 || matching[0].orientation != 1 || matching[0].rings.len() != 1 {
            return Err(Error::Unsupported(
                "multi chamfer must retain one simple final face for every bevel plane",
            ));
        }
        bevels.push(matching[0].clone());
    }
    let total = removed
        .iter()
        .try_fold(solid.volume()?, |sum, s| Ok::<_, Error>(sum + s.volume()?))?;
    let original = source.volume()?;
    if !total.is_finite() || (total - original).abs() > original.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "multi chamfer does not conserve source volume",
        ));
    }
    Ok(PlanarEdgeChamfers {
        solid,
        removed,
        bevels,
        planes,
        selections: selections.to_vec(),
    })
}
