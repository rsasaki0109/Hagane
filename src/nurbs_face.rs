//! Rectangular open NURBS face using the kernel's existing B-rep entities.
use crate::*;

/// One open face with four shared corner vertices and oriented coedges.
/// Boundary validation does not certify surface regularity or a closed solid.
#[derive(Clone, Debug)]
pub struct NurbsFace {
    pub vertices: [Vertex; 4],
    pub edges: [Edge; 4],
    pub face: Face,
}
impl NurbsFace {
    /// Retain the exact surface and its complete rectangular parameter domain.
    pub fn new(surface: NurbsSurface, orientation: i8, tol: Tolerance) -> Result<Self> {
        let boundary = surface.boundary_edges()?;
        let endpoint = |i: usize| -> Result<Vertex> {
            let curve = &boundary[i].curve;
            let parameter = curve.domain()[usize::from(!boundary[i].forward)];
            Ok(Vertex {
                point: curve.evaluate(parameter)?,
            })
        };
        let vertices = [endpoint(0)?, endpoint(1)?, endpoint(2)?, endpoint(3)?];
        let edges = std::array::from_fn(|i| Edge {
            vertices: if boundary[i].forward {
                [i, (i + 1) % 4]
            } else {
                [(i + 1) % 4, i]
            },
            curve: Curve::Nurbs(Box::new(boundary[i].curve.clone())),
        });
        let face = Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires: vec![Wire {
                coedges: boundary
                    .iter()
                    .enumerate()
                    .map(|(i, b)| Coedge {
                        edge: i,
                        forward: b.forward,
                        pcurve: b.pcurve.clone(),
                    })
                    .collect(),
            }],
            orientation,
        };
        let result = Self {
            vertices,
            edges,
            face,
        };
        result.validate_boundary(tol)?;
        Ok(result)
    }
    /// Validate the canonical rectangular boundary, reference indices, endpoint
    /// sharing, UV maps and traversal. Global injectivity is not checked.
    pub fn validate_boundary(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        if !matches!(self.face.orientation, -1 | 1) {
            return Err(Error::InvalidTopology(
                "NURBS face orientation must be +1 or -1",
            ));
        }
        let Surface::Nurbs(surface) = &self.face.surface else {
            return Err(Error::Unsupported("NURBS face requires a rational surface"));
        };
        if self.face.wires.len() != 1 || self.face.wires[0].coedges.len() != 4 {
            return Err(Error::Unsupported(
                "NURBS face supports only its complete rectangular domain",
            ));
        }
        let expected = surface.boundary_edges()?;
        for (i, boundary) in expected.iter().enumerate() {
            let edge = &self.edges[i];
            let coedge = &self.face.wires[0].coedges[i];
            if coedge.edge != i || coedge.forward != boundary.forward {
                return Err(Error::InvalidTopology(
                    "NURBS boundary coedge reference or direction differs",
                ));
            }
            let ids = if boundary.forward {
                [i, (i + 1) % 4]
            } else {
                [(i + 1) % 4, i]
            };
            if edge.vertices != ids {
                return Err(Error::InvalidTopology(
                    "NURBS boundary corner sharing differs",
                ));
            }
            let (
                PCurve::Affine { origin, direction },
                PCurve::Affine {
                    origin: eo,
                    direction: ed,
                },
            ) = (&coedge.pcurve, &boundary.pcurve)
            else {
                return Err(Error::Unsupported(
                    "NURBS rectangular boundary requires affine UV maps",
                ));
            };
            if origin != eo || direction != ed {
                return Err(Error::InvalidTopology(
                    "NURBS boundary UV map differs from its surface domain",
                ));
            }
            let Curve::Nurbs(curve) = &edge.curve else {
                return Err(Error::Unsupported(
                    "NURBS boundary requires canonical rational curves",
                ));
            };
            // Require canonical homogeneous weights rather than accepting a
            // small absolute weight error that could hide a large shape change.
            if curve.degree() != boundary.curve.degree()
                || curve.knots() != boundary.curve.knots()
                || curve.weights() != boundary.curve.weights()
                || curve.control_points().len() != boundary.curve.control_points().len()
                || curve
                    .control_points()
                    .iter()
                    .zip(boundary.curve.control_points())
                    .any(|(a, b)| !tol.coincident(*a, *b))
            {
                return Err(Error::InvalidTopology(
                    "NURBS boundary geometry differs from canonical surface isocurve",
                ));
            }
            for (id, parameter) in edge.vertices.into_iter().zip(curve.domain()) {
                let point = self.vertices[id].point;
                if !point.finite() || !tol.coincident(point, curve.evaluate(parameter)?) {
                    return Err(Error::InvalidTopology(
                        "NURBS boundary vertex disagrees with curve",
                    ));
                }
            }
        }
        Ok(())
    }
    /// Display the retained B-rep surface after boundary validation. This is
    /// uniform sampling, with no whole-surface approximation error guarantee.
    pub fn sample_grid(&self, cells: [usize; 2], tol: Tolerance) -> Result<Mesh> {
        self.validate_boundary(tol)?;
        let Surface::Nurbs(surface) = &self.face.surface else {
            unreachable!()
        };
        let mut mesh = surface.sample_grid(cells)?;
        if self.face.orientation < 0 {
            for triangle in &mut mesh.triangles {
                triangle.swap(1, 2);
            }
            for normal in &mut mesh.normals {
                *normal = *normal * -1.;
            }
        }
        Ok(mesh)
    }
    /// Place the exact surface and regenerate its canonical boundary topology.
    pub fn transformed(&self, transform: Transform, tol: Tolerance) -> Result<Self> {
        self.validate_boundary(tol)?;
        let Surface::Nurbs(surface) = self.face.surface.transformed(transform)? else {
            unreachable!()
        };
        Self::new(*surface, self.face.orientation, tol)
    }
}
