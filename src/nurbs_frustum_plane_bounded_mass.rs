//! Checked common-local-frame moments of actual ordered plane bounds.
use crate::nurbs_frustum_bored_oblique_mass::{inertia_from_centered, subtract_centered};
use crate::nurbs_frustum_oblique_mass::CheckedCenteredMoments;
use crate::*;
fn moments(
    body: &NurbsFrustumPlaneBoundedSolid,
    policy: GeometryTolerance,
) -> Result<CheckedCenteredMoments> {
    body.validate(policy)?;
    crate::nurbs_frustum_slab_mass::section_moments(
        body.source(),
        body.lower_section(),
        body.upper_section(),
        policy,
        || body.volume(policy),
    )
}
fn bored_moments(
    body: &NurbsBoredFrustumPlaneBoundedSolid,
    policy: GeometryTolerance,
) -> Result<CheckedCenteredMoments> {
    body.validate(policy)?;
    subtract_centered(
        moments(body.outer(), policy)?,
        moments(body.inner(), policy)?,
        body.volume(policy)?,
        body.source().source().frame(),
        policy,
    )
}
impl NurbsFrustumPlaneBoundedSolid {
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = moments(self, policy)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        inertia_from_centered(moments(self, policy)?, self.source().frame())
    }
}
impl NurbsBoredFrustumPlaneBoundedSolid {
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = bored_moments(self, policy)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        inertia_from_centered(bored_moments(self, policy)?, self.source().source().frame())
    }
}
