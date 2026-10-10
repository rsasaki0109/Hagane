//! Analytic local centered moments of parallel sections; no world raw subtraction.
use crate::nurbs_frustum_bored_oblique_mass::{inertia_from_centered, subtract_centered};
use crate::nurbs_frustum_oblique_mass::{checked_centered_moments, CheckedCenteredMoments};
use crate::*;

fn slab_moments(
    body: &NurbsFrustumSlabSolid,
    policy: GeometryTolerance,
) -> Result<CheckedCenteredMoments> {
    body.validate(policy)?;
    let moments = match (body.lower_section(), body.upper_section()) {
        (None, Some(upper)) => {
            let pieces = body.source().split_by_plane(upper.plane(), policy)?;
            checked_centered_moments(&pieces.lower, policy)?
        }
        (Some(lower), None) => {
            let pieces = body.source().split_by_plane(lower.plane(), policy)?;
            checked_centered_moments(&pieces.upper, policy)?
        }
        (Some(lower), Some(upper)) => {
            let upper_pieces = body.source().split_by_plane(upper.plane(), policy)?;
            let lower_pieces = body.source().split_by_plane(lower.plane(), policy)?;
            subtract_centered(
                checked_centered_moments(&upper_pieces.lower, policy)?,
                checked_centered_moments(&lower_pieces.lower, policy)?,
                body.volume(policy)?,
                body.source().frame(),
                policy,
            )?
        }
        (None, None) => {
            return Err(Error::Unsupported(
                "slab analytic moments require a retained section boundary",
            ))
        }
    };
    // The geometry's independently checked stable volume remains authoritative.
    let volume = body.volume(policy)?;
    if (moments.volume / volume - 1.).abs() > 32768. * f64::EPSILON {
        return Err(Error::Unsupported(
            "slab cumulative moment volume correspondence is unresolved",
        ));
    }
    Ok(CheckedCenteredMoments { volume, ..moments })
}
fn bored_moments(
    body: &NurbsBoredFrustumSlabSolid,
    policy: GeometryTolerance,
) -> Result<CheckedCenteredMoments> {
    body.validate(policy)?;
    subtract_centered(
        slab_moments(body.outer(), policy)?,
        slab_moments(body.inner(), policy)?,
        body.volume(policy)?,
        body.source().source().frame(),
        policy,
    )
}
impl NurbsFrustumSlabSolid {
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = slab_moments(self, policy)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        inertia_from_centered(slab_moments(self, policy)?, self.source().frame())
    }
}
impl NurbsBoredFrustumSlabSolid {
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
