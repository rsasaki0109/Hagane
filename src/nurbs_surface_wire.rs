//! Closed strictly convex UV boundaries with exact lifted rational edges.
use crate::*;

/// A boundary only: neither a trimmed face nor a regularity certificate.
#[derive(Clone, Debug)]
pub struct NurbsSurfaceWire {
    pub surface: NurbsSurface,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub wire: Wire,
    uv: Vec<[f64; 2]>,
}
impl NurbsSurfaceWire {
    /// Preserve input traversal; accept either clockwise or counterclockwise.
    /// Supply each corner once (the closing edge is implicit).
    pub fn new(surface: NurbsSurface, uv: Vec<[f64; 2]>, tol: Tolerance) -> Result<Self> {
        check_polygon(&uv)?;
        let mut vertices = Vec::with_capacity(uv.len());
        let mut edges = Vec::with_capacity(uv.len());
        let mut coedges = Vec::with_capacity(uv.len());
        for i in 0..uv.len() {
            let lifted =
                NurbsSurfaceEdge::new(surface.clone(), uv[i], uv[(i + 1) % uv.len()], tol)?;
            vertices.push(lifted.vertices[0].clone());
            let mut edge = lifted.edge;
            edge.vertices = [i, (i + 1) % uv.len()];
            edges.push(edge);
            coedges.push(Coedge {
                edge: i,
                forward: true,
                pcurve: lifted.pcurve,
            });
        }
        let result = Self {
            surface,
            vertices,
            edges,
            wire: Wire { coedges },
            uv,
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub fn uv_corners(&self) -> &[[f64; 2]] {
        &self.uv
    }
    pub fn orientation(&self) -> Result<Orientation> {
        check_polygon(&self.uv)
    }
    /// Check canonical geometry, same-parameter pcurves and shared closure.
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        check_polygon(&self.uv)?;
        let n = self.uv.len();
        if self.vertices.len() != n || self.edges.len() != n || self.wire.coedges.len() != n {
            return Err(Error::InvalidTopology(
                "surface wire entity counts disagree",
            ));
        }
        for i in 0..n {
            let next = (i + 1) % n;
            let coedge = &self.wire.coedges[i];
            if self.edges[i].vertices != [i, next] || coedge.edge != i || !coedge.forward {
                return Err(Error::InvalidTopology(
                    "surface wire sharing or traversal disagrees",
                ));
            }
            let mut lifted =
                NurbsSurfaceEdge::new(self.surface.clone(), self.uv[i], self.uv[next], tol)?;
            lifted.vertices = [self.vertices[i].clone(), self.vertices[next].clone()];
            lifted.edge = self.edges[i].clone();
            lifted.edge.vertices = [0, 1];
            lifted.pcurve = coedge.pcurve.clone();
            lifted.validate(tol)?;
        }
        Ok(())
    }
    /// Separate bounded edge polylines, retaining each edge's T=0..1.
    pub fn tessellate_boundary(
        &self,
        error: f64,
        max_segments_per_edge: usize,
        tol: Tolerance,
    ) -> Result<Vec<NurbsPolyline>> {
        self.validate(tol)?;
        self.edges
            .iter()
            .map(|edge| match &edge.curve {
                Curve::Nurbs(curve) => curve.tessellate_bounded(error, max_segments_per_edge),
                _ => Err(Error::InvalidTopology(
                    "surface wire requires rational edges",
                )),
            })
            .collect()
    }
}
fn check_polygon(uv: &[[f64; 2]]) -> Result<Orientation> {
    if !(3..=64).contains(&uv.len()) {
        return Err(Error::Unsupported(
            "surface wire supports 3 through 64 convex UV corners",
        ));
    }
    let direction = orient2d(uv[0], uv[1], uv[2])?;
    if direction == Orientation::Collinear {
        return Err(Error::InvalidInput("surface wire has collinear UV corners"));
    }
    // Every nonincident corner must lie strictly on the same side of every
    // directed edge. Unlike a local turn test this also rejects star polygons.
    for i in 0..uv.len() {
        let next = (i + 1) % uv.len();
        for j in 0..uv.len() {
            if j != i && j != next && orient2d(uv[i], uv[next], uv[j])? != direction {
                return Err(Error::Unsupported(
                    "surface wire requires a strictly convex simple UV polygon",
                ));
            }
        }
    }
    Ok(direction)
}
