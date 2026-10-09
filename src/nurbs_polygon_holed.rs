//! Convex rational outer boundary with exact separated rectangular inner wires.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsPolygonHoledFace {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub face: Face,
    outer: NurbsPolygonFace,
    inner: Vec<NurbsSurfaceWire>,
    holes: Vec<[[f64; 2]; 2]>,
}
impl NurbsPolygonHoledFace {
    pub fn new(
        surface: NurbsSurface,
        outer: Vec<[f64; 2]>,
        holes: Vec<[[f64; 2]; 2]>,
        orientation: i8,
        tol: Tolerance,
    ) -> Result<Self> {
        let outer = NurbsPolygonFace::new(surface.clone(), outer, orientation, tol)?;
        check_holes(&surface, outer.boundary.uv_corners(), &holes)?;
        let mut vertices = outer.boundary.vertices.clone();
        let mut edges = outer.boundary.edges.clone();
        let mut wires = outer.face.wires.clone();
        let mut inner = Vec::with_capacity(holes.len());
        for &[[u0, u1], [v0, v1]] in &holes {
            // Clockwise UV traversal leaves retained material on the left.
            let boundary = NurbsSurfaceWire::new(
                surface.clone(),
                vec![[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
                tol,
            )?;
            let vertex_offset = vertices.len();
            let edge_offset = edges.len();
            vertices.extend(boundary.vertices.clone());
            edges.extend(boundary.edges.iter().cloned().map(|mut edge| {
                edge.vertices = edge.vertices.map(|i| i + vertex_offset);
                edge
            }));
            let mut wire = boundary.wire.clone();
            for coedge in &mut wire.coedges {
                coedge.edge += edge_offset;
            }
            wires.push(wire);
            inner.push(boundary);
        }
        let face = Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires,
            orientation,
        };
        let result = Self {
            vertices,
            edges,
            face,
            outer,
            inner,
            holes,
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub fn holes(&self) -> &[[[f64; 2]; 2]] {
        &self.holes
    }
    pub fn outer_corners(&self) -> &[[f64; 2]] {
        self.outer.boundary.uv_corners()
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        let n = self.outer.boundary.vertices.len();
        let expected = n + 4 * self.inner.len();
        if self.vertices.len() != expected
            || self.edges.len() != expected
            || self.face.wires.len() != 1 + self.inner.len()
        {
            return Err(Error::InvalidTopology(
                "holed polygon entity counts disagree",
            ));
        }
        let mut outer = self.outer.clone();
        outer.boundary.vertices = self.vertices[..n].to_vec();
        outer.boundary.edges = self.edges[..n].to_vec();
        outer.boundary.wire = self.face.wires[0].clone();
        outer.face.surface = self.face.surface.clone();
        outer.face.wires = vec![self.face.wires[0].clone()];
        outer.face.orientation = self.face.orientation;
        outer.validate(tol)?;
        for (index, original) in self.inner.iter().enumerate() {
            let offset = n + 4 * index;
            let mut boundary = original.clone();
            boundary.vertices = self.vertices[offset..offset + 4].to_vec();
            boundary.edges = self.edges[offset..offset + 4].to_vec();
            boundary.wire = self.face.wires[index + 1].clone();
            if boundary.wire.coedges.len() != 4 {
                return Err(Error::InvalidTopology(
                    "holed polygon requires four inner coedges",
                ));
            }
            for i in 0..4 {
                if boundary.edges[i].vertices != [offset + i, offset + (i + 1) % 4]
                    || boundary.wire.coedges[i].edge != offset + i
                {
                    return Err(Error::InvalidTopology(
                        "holed polygon inner references disagree",
                    ));
                }
                boundary.edges[i].vertices = boundary.edges[i].vertices.map(|v| v - offset);
                boundary.wire.coedges[i].edge -= offset;
            }
            boundary.validate(tol)?;
            if boundary.orientation()? != Orientation::Clockwise {
                return Err(Error::InvalidTopology(
                    "holed polygon inner UV wire must be clockwise",
                ));
            }
        }
        Ok(())
    }
    /// Display the B-rep material domain, splitting C0 normals and hole lines.
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsPolygonMesh> {
        self.validate(tol)?;
        let mut outer = self.outer.clone();
        outer.face.orientation = self.face.orientation;
        outer.tessellate_holed_bounded(error, max_triangles, tol, &self.holes)
    }
}
fn check_holes(surface: &NurbsSurface, outer: &[[f64; 2]], holes: &[[[f64; 2]; 2]]) -> Result<()> {
    if holes.len() > 16 {
        return Err(Error::Unsupported(
            "holed polygon supports at most 16 rectangular UV openings",
        ));
    }
    let domains = surface.domain();
    let widths = domains.map(|d| d[1] - d[0]);
    let guards = domains.map(|d| {
        128. * f64::EPSILON
            * d[0]
                .abs()
                .max(d[1].abs())
                .max((d[1] - d[0]).abs())
                .max(f64::MIN_POSITIVE)
    });
    let normalized_guard = guards[0] / widths[0] + guards[1] / widths[1];
    if !normalized_guard.is_finite() {
        return Err(Error::Unsupported(
            "polygon opening parameter precision exceeds numerical range",
        ));
    }
    for (index, hole) in holes.iter().enumerate() {
        for axis in 0..2 {
            let [lo, hi] = hole[axis];
            if !lo.is_finite()
                || !hi.is_finite()
                || lo >= hi
                || lo < domains[axis][0]
                || hi > domains[axis][1]
                || hi - lo <= 2. * guards[axis]
            {
                return Err(Error::InvalidInput(
                    "polygon openings need finite ordered resolved UV ranges",
                ));
            }
        }
        let [[u0, u1], [v0, v1]] = *hole;
        for corner in [[u0, v0], [u0, v1], [u1, v1], [u1, v0]] {
            for i in 0..outer.len() {
                let a = outer[i];
                let b = outer[(i + 1) % outer.len()];
                if orient2d(a, b, corner)? != Orientation::CounterClockwise {
                    return Err(Error::Unsupported(
                        "polygon opening must lie strictly inside outer UV boundary",
                    ));
                }
                let edge = [(b[0] - a[0]) / widths[0], (b[1] - a[1]) / widths[1]];
                let offset = [
                    (corner[0] - a[0]) / widths[0],
                    (corner[1] - a[1]) / widths[1],
                ];
                let cross = edge[0] * offset[1] - edge[1] * offset[0];
                let clearance = edge[0].hypot(edge[1]) * normalized_guard;
                if !cross.is_finite() || !clearance.is_finite() || cross <= clearance {
                    return Err(Error::Unsupported(
                        "polygon opening has unresolved UV boundary clearance",
                    ));
                }
            }
        }
        for other in &holes[..index] {
            if !(0..2).any(|axis| {
                hole[axis][0] - other[axis][1] > guards[axis]
                    || other[axis][0] - hole[axis][1] > guards[axis]
            }) {
                return Err(Error::Unsupported(
                    "polygon openings overlap or have unresolved UV separation",
                ));
            }
        }
    }
    Ok(())
}
