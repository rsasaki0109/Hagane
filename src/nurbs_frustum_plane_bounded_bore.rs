//! Coaxial drilling and strict enlargement of actual plane-bounded bodies.
//! Removed bodies are independently closed, certified finite partitions; no
//! nominal-height cylinder or mesh Boolean stands in for retained geometry.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsFrustumPlaneBoundedBorePartition {
    pub kept: NurbsBoredFrustumPlaneBoundedSolid,
    pub removed: NurbsFrustumPlaneBoundedSolid,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumPlaneBoundedBoreEnlargement {
    pub kept: NurbsBoredFrustumPlaneBoundedSolid,
    pub removed: NurbsBoredFrustumPlaneBoundedSolid,
}
fn unresolved() -> Error {
    Error::Unsupported("plane-bounded coaxial bore clearance, removed annulus or partition precision is unresolved")
}
fn restrict_cylinder(
    outer: &NurbsFrustumPlaneBoundedSolid,
    cylinder: &NurbsFrustumSolid,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumPlaneBoundedSolid> {
    let lower = outer
        .lower_section()
        .map(|s| cylinder.section_by_plane(s.plane(), policy))
        .transpose()?;
    let upper = outer
        .upper_section()
        .map(|s| cylinder.section_by_plane(s.plane(), policy))
        .transpose()?;
    NurbsFrustumPlaneBoundedSolid::new(cylinder, lower, upper, policy)
}
fn bore(
    outer: &NurbsFrustumPlaneBoundedSolid,
    radius: f64,
    policy: GeometryTolerance,
) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
    outer.validate(policy)?;
    // Canonical admission uses the ORIGINAL full source dimensions and frame,
    // including the global minimum radius and original effective boundary band.
    let source = outer.source().bore_coaxial(radius, policy)?;
    let inner = restrict_cylinder(outer, source.tool(), policy)?;
    NurbsBoredFrustumPlaneBoundedSolid::new(&source, outer.clone(), inner, policy)
}
fn enlargement_clearance(
    body: &NurbsBoredFrustumPlaneBoundedSolid,
    radius: f64,
    policy: GeometryTolerance,
) -> Result<()> {
    body.validate(policy)?;
    enlargement_clearance_for_source(
        body.source().source(),
        body.source().radius(),
        radius,
        &[
            body.outer().lower_section().map(|s| s.plane()),
            body.outer().upper_section().map(|s| s.plane()),
        ],
        body.solid(),
        policy,
    )
}
pub(crate) fn enlargement_clearance_for_source(
    source: &NurbsFrustumSolid,
    old_radius: f64,
    radius: f64,
    planes: &[Option<&Surface>],
    solid: &Solid,
    policy: GeometryTolerance,
) -> Result<()> {
    if !radius.is_finite() || radius <= 0. {
        return Err(Error::InvalidInput(
            "enlarged coaxial bore radius must be positive and finite",
        ));
    }
    let [a, b] = source.radii();
    let extent = (2. * a.max(b)).hypot(source.height());
    let band = policy.length_at_scale(extent)?;
    let origin = source.frame().origin();
    let mut world = extent
        .max(origin.x.abs())
        .max(origin.y.abs())
        .max(origin.z.abs());
    for plane in planes.iter().flatten() {
        let Surface::Plane { origin, .. } = **plane else {
            return Err(unresolved());
        };
        world = world
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
    }
    for edge in &solid.edges {
        let Curve::Nurbs(curve) = &edge.curve else {
            continue;
        };
        for p in curve.control_points() {
            world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
    }
    let axes = source.frame().axes();
    let mut gram = 0f64;
    for (i, a) in axes.iter().enumerate() {
        for (j, b) in axes.iter().enumerate() {
            gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
        }
    }
    let arithmetic = 65536. * f64::EPSILON * world + 16. * gram * extent;
    let gap = radius - old_radius;
    if !arithmetic.is_finite()
        || arithmetic >= policy.linear() / 16.
        || !gap.is_finite()
        || gap <= 10. * band + arithmetic
    {
        return Err(Error::Unsupported("coaxial enlargement requires a strictly larger radius with resolved clearance in the original source tolerance"));
    }
    Ok(())
}
pub(crate) fn conservation(original: f64, kept: f64, removed: f64, full_source: f64) -> Result<()> {
    let sum = kept / original + removed / original;
    let allowance = 131072. * f64::EPSILON * full_source / original;
    if !original.is_normal()
        || original <= 0.
        || !kept.is_normal()
        || kept <= 0.
        || !removed.is_normal()
        || removed <= 0.
        || !sum.is_finite()
        || !allowance.is_finite()
        || (sum - 1.).abs() > allowance
    {
        return Err(unresolved());
    }
    Ok(())
}
impl NurbsFrustumPlaneBoundedSolid {
    /// Drill a coaxial through opening in the actual retained body. The full
    /// original source must admit the radius; caps and outer walls are retained.
    pub fn bore_coaxial(
        &self,
        radius: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
        bore(self, radius, policy)
    }
    /// Return closed kept and removed bodies, with the same exact cylindrical
    /// interface. Removed caps use their actual retained section planes.
    pub fn partition_coaxial_bore(
        &self,
        radius: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumPlaneBoundedBorePartition> {
        let kept = bore(self, radius, policy)?;
        let removed = kept.inner().clone();
        removed.validate(policy)?;
        conservation(
            self.volume(policy)?,
            kept.volume(policy)?,
            removed.volume(policy)?,
            self.source().volume(policy)?,
        )?;
        Ok(NurbsFrustumPlaneBoundedBorePartition { kept, removed })
    }
}
impl NurbsBoredFrustumPlaneBoundedSolid {
    /// Strictly enlarge the original coaxial opening. Equal, shrinking,
    /// contacting and unresolved-radius changes are unsupported.
    pub fn enlarge_coaxial_bore(&self, radius: f64, policy: GeometryTolerance) -> Result<Self> {
        enlargement_clearance(self, radius, policy)?;
        bore(self.outer(), radius, policy)
    }
    /// Return the closed kept body and the removed annular region. The original
    /// inner body is retained exactly as the removed region's inward boundary.
    pub fn partition_coaxial_bore_enlargement(
        &self,
        radius: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumPlaneBoundedBoreEnlargement> {
        let kept = self.enlarge_coaxial_bore(radius, policy)?;
        let removed_source = kept
            .inner()
            .source()
            .bore_coaxial(self.source().radius(), policy)?;
        let removed = Self::new(
            &removed_source,
            kept.inner().clone(),
            self.inner().clone(),
            policy,
        )?;
        conservation(
            self.volume(policy)?,
            kept.volume(policy)?,
            removed.volume(policy)?,
            self.source().source().volume(policy)?,
        )?;
        Ok(NurbsFrustumPlaneBoundedBoreEnlargement { kept, removed })
    }
}
