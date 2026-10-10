//! Certified display of actual rational ruled children of an oblique cut.
use crate::*;
impl NurbsObliqueFrustumSolid {
    /// Approximate retained NURBS patches with a conforming common dyadic grid.
    /// Unequal layer weights make generator V nonuniform in physical length;
    /// shared line samples use the exact endpoint-weight rational mapping.
    pub fn tessellate(&self, chord_error: f64, policy: GeometryTolerance) -> Result<Mesh> {
        self.validate(policy)?;
        let mut weights = [[0.; 2]; 4];
        for (q, pair) in weights.iter_mut().enumerate() {
            let Surface::Nurbs(surface) = &self.solid().shell.faces[q + 2].surface else {
                return Err(Error::Unsupported(
                    "oblique frustum display requires retained rational ruled faces",
                ));
            };
            if surface.degrees() != [2, 1]
                || surface.control_counts() != [3, 2]
                || surface.domain() != [[0., 1.], [0., 1.]]
            {
                return Err(Error::Unsupported(
                    "oblique frustum display requires one quadratic/linear span",
                ));
            }
            *pair = [surface.weights()[0], surface.weights()[1]];
        }
        crate::nurbs_frustum_display::tessellate_ruled_four_quarters(
            self.solid(),
            chord_error,
            policy,
            Some(weights),
        )
    }
}
