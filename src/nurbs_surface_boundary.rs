//! Exact, oriented rectangular boundary data for a standalone NURBS patch.
use crate::{NurbsCurve, NurbsSurface, PCurve, Result};

/// A surface boundary in increasing curve parameter, with a separate traversal
/// direction. This is not yet a topological edge shared by B-rep faces.
#[derive(Clone, Debug)]
pub struct NurbsBoundaryEdge {
    pub curve: NurbsCurve,
    /// Exact UV map evaluated at the same parameter as `curve`.
    pub pcurve: PCurve,
    /// Traversal follows the positive UV orientation (outer loop CCW).
    pub forward: bool,
}
impl NurbsSurface {
    /// Bottom, right, top, left edges of the complete rectangular parameter
    /// domain. Top and left traverse backward; curve parameters stay unchanged.
    pub fn boundary_edges(&self) -> Result<[NurbsBoundaryEdge; 4]> {
        let [[u0, u1], [v0, v1]] = self.domain();
        let make = |axis, parameter, forward| -> Result<NurbsBoundaryEdge> {
            Ok(NurbsBoundaryEdge {
                curve: self.isocurve(axis, parameter)?,
                pcurve: PCurve::Affine {
                    origin: if axis == 0 {
                        [parameter, 0.]
                    } else {
                        [0., parameter]
                    },
                    direction: if axis == 0 { [0., 1.] } else { [1., 0.] },
                },
                forward,
            })
        };
        Ok([
            make(1, v0, true)?,
            make(0, u1, true)?,
            make(1, v1, false)?,
            make(0, u0, false)?,
        ])
    }
}
