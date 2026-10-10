//! Display retained two-cap rational slabs without rebuilding their geometry.
use crate::*;
impl NurbsFrustumSlabSolid {
    pub fn tessellate(&self, chord: f64, policy: GeometryTolerance) -> Result<Mesh> {
        self.validate(policy)?;
        let mut weights = [[0.; 2]; 4];
        for (q, pair) in weights.iter_mut().enumerate() {
            let Surface::Nurbs(surface) = &self.solid().shell.faces[2 + q].surface else {
                return Err(Error::Unsupported(
                    "slab display requires actual rational ruled walls",
                ));
            };
            *pair = [surface.weights()[0], surface.weights()[1]];
        }
        crate::nurbs_frustum_display::tessellate_ruled_four_quarters(
            self.solid(),
            chord,
            policy,
            Some(weights),
        )
    }
}
impl NurbsBoredFrustumSlabSolid {
    pub fn tessellate(&self, chord: f64, policy: GeometryTolerance) -> Result<Mesh> {
        crate::nurbs_frustum_bored_oblique_display::tessellate_annular_ruled(
            self.solid(),
            self.source().source().frame(),
            self.source().radius(),
            chord,
            || self.validate(policy),
        )
    }
}
