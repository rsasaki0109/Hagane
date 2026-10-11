//! Checked adapters preserve the actual legacy retained BRep before drilling.
use crate::nurbs_frustum_plane_bounded_bore::{conservation, enlargement_clearance_for_source};
use crate::*;

#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumBoreEnlargement {
    pub kept: NurbsBoredFrustumSolid,
    pub removed: NurbsBoredFrustumSolid,
}
fn oblique(
    body: &NurbsObliqueFrustumSolid,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumPlaneBoundedSolid> {
    body.validate(policy)?;
    let (lower, upper) = if body.is_lower() {
        (None, Some(body.section().clone()))
    } else {
        (Some(body.section().clone()), None)
    };
    NurbsFrustumPlaneBoundedSolid::from_retained(
        body.source(),
        lower,
        upper,
        body.solid().clone(),
        policy,
    )
}
fn slab(
    body: &NurbsFrustumSlabSolid,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumPlaneBoundedSolid> {
    body.validate(policy)?;
    NurbsFrustumPlaneBoundedSolid::from_retained(
        body.source(),
        body.lower_section().cloned(),
        body.upper_section().cloned(),
        body.solid().clone(),
        policy,
    )
}
fn bored_oblique(
    body: &NurbsBoredObliqueFrustumSolid,
    policy: GeometryTolerance,
) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
    body.validate(policy)?;
    NurbsBoredFrustumPlaneBoundedSolid::from_retained(
        body.source(),
        oblique(body.outer(), policy)?,
        oblique(body.inner(), policy)?,
        body.solid().clone(),
        policy,
    )
}
fn bored_slab(
    body: &NurbsBoredFrustumSlabSolid,
    policy: GeometryTolerance,
) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
    body.validate(policy)?;
    NurbsBoredFrustumPlaneBoundedSolid::from_retained(
        body.source(),
        slab(body.outer(), policy)?,
        slab(body.inner(), policy)?,
        body.solid().clone(),
        policy,
    )
}
macro_rules! plain_adapter {
    ($ty:ty,$convert:ident) => {
        impl $ty {
            /// Drill the actual retained body while preserving its outer boundary.
            pub fn bore_coaxial(
                &self,
                radius: f64,
                policy: GeometryTolerance,
            ) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
                $convert(self, policy)?.bore_coaxial(radius, policy)
            }
            /// Return independently closed kept and removed partitions.
            pub fn partition_coaxial_bore(
                &self,
                radius: f64,
                policy: GeometryTolerance,
            ) -> Result<NurbsFrustumPlaneBoundedBorePartition> {
                $convert(self, policy)?.partition_coaxial_bore(radius, policy)
            }
        }
    };
}
macro_rules! bored_adapter {
    ($ty:ty,$convert:ident) => {
        impl $ty {
            /// Strictly enlarge the actual opening using the original source tolerance.
            pub fn enlarge_coaxial_bore(
                &self,
                radius: f64,
                policy: GeometryTolerance,
            ) -> Result<NurbsBoredFrustumPlaneBoundedSolid> {
                $convert(self, policy)?.enlarge_coaxial_bore(radius, policy)
            }
            /// Return the kept body and the actual removed closed annulus.
            pub fn partition_coaxial_bore_enlargement(
                &self,
                radius: f64,
                policy: GeometryTolerance,
            ) -> Result<NurbsFrustumPlaneBoundedBoreEnlargement> {
                $convert(self, policy)?.partition_coaxial_bore_enlargement(radius, policy)
            }
        }
    };
}
plain_adapter!(NurbsObliqueFrustumSolid, oblique);
plain_adapter!(NurbsFrustumSlabSolid, slab);
bored_adapter!(NurbsBoredObliqueFrustumSolid, bored_oblique);
bored_adapter!(NurbsBoredFrustumSlabSolid, bored_slab);
impl NurbsBoredFrustumSolid {
    /// Strict enlargement; equal, shrinking and unresolved changes are refused.
    pub fn enlarge_coaxial_bore(&self, radius: f64, policy: GeometryTolerance) -> Result<Self> {
        self.validate(policy)?;
        enlargement_clearance_for_source(
            self.source(),
            self.radius(),
            radius,
            &[],
            self.solid(),
            policy,
        )?;
        self.source().bore_coaxial(radius, policy)
    }
    /// Partition into a kept body and an independently certified removed annulus.
    pub fn partition_coaxial_bore_enlargement(
        &self,
        radius: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumBoreEnlargement> {
        let kept = self.enlarge_coaxial_bore(radius, policy)?;
        let removed = kept.tool().bore_coaxial(self.radius(), policy)?;
        conservation(
            self.volume(policy)?,
            kept.volume(policy)?,
            removed.volume(policy)?,
            self.source().volume(policy)?,
        )?;
        Ok(NurbsBoredFrustumBoreEnlargement { kept, removed })
    }
}
