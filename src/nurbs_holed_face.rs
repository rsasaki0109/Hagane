//! Exact rectangular NURBS face domains with rectangular UV openings.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsHoledFace {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub face: Face,
    outer: [[f64; 2]; 2],
    holes: Vec<[[f64; 2]; 2]>,
}
impl NurbsHoledFace {
    pub fn new(
        surface: NurbsSurface,
        outer: [[f64; 2]; 2],
        holes: Vec<[[f64; 2]; 2]>,
        orientation: i8,
        tol: Tolerance,
    ) -> Result<Self> {
        Tolerance::new(tol.linear)?;
        validate_regions(outer, &holes)?;
        let mut surface = surface.restricted(outer)?;
        let mut insertions: [Vec<(f64, usize)>; 2] = [Vec::new(), Vec::new()];
        let mut counts = surface.control_counts();
        for axis in 0..2 {
            let mut parameters = holes.iter().flat_map(|hole| hole[axis]).collect::<Vec<_>>();
            parameters.sort_by(f64::total_cmp);
            parameters.dedup();
            for parameter in parameters {
                let knots = surface.knots(axis)?;
                let multiplicity = knots.partition_point(|v| *v <= parameter)
                    - knots.partition_point(|v| *v < parameter);
                let times = surface.degrees()[axis] - multiplicity;
                if times > 0 {
                    insertions[axis].push((parameter, times));
                    counts[axis] = counts[axis].saturating_add(times);
                }
            }
        }
        if counts[0].saturating_mul(counts[1]) > 65536 {
            return Err(Error::Unsupported(
                "NURBS hole refinement exceeds control limit",
            ));
        }
        let mut current = surface.control_counts();
        let mut work = 0usize;
        for axis in 0..2 {
            for (_, times) in &insertions[axis] {
                current[axis] += times;
                work = work.saturating_add(
                    current[0]
                        .saturating_mul(current[1])
                        .saturating_mul(*times)
                        .saturating_mul(surface.degrees()[axis] + 1),
                );
            }
        }
        if work > 16_000_000 {
            return Err(Error::Unsupported(
                "NURBS hole refinement exceeds work limit",
            ));
        }
        for (axis, insertions) in insertions.into_iter().enumerate() {
            for (parameter, times) in insertions {
                surface = surface.insert_knot(axis, parameter, times)?;
            }
        }
        let result = Self::assemble(surface, outer, holes, orientation, tol)?;
        result.validate_boundary(tol)?;
        Ok(result)
    }
    pub fn outer(&self) -> [[f64; 2]; 2] {
        self.outer
    }
    pub fn holes(&self) -> &[[[f64; 2]; 2]] {
        &self.holes
    }
    fn assemble(
        surface: NurbsSurface,
        outer: [[f64; 2]; 2],
        holes: Vec<[[f64; 2]; 2]>,
        orientation: i8,
        tol: Tolerance,
    ) -> Result<Self> {
        let base = NurbsFace::new(surface.clone(), orientation, tol)?;
        let mut vertices = base.vertices.to_vec();
        let mut edges = base.edges.to_vec();
        let mut face = base.face;
        for hole in &holes {
            let patch = NurbsFace::new(surface.restricted(*hole)?, 1, tol)?;
            let offset = vertices.len();
            vertices.extend(patch.vertices);
            for mut edge in patch.edges {
                for id in &mut edge.vertices {
                    *id += offset;
                }
                edges.push(edge);
            }
            let wire = Wire {
                coedges: patch.face.wires[0]
                    .coedges
                    .iter()
                    .rev()
                    .map(|c| Coedge {
                        edge: c.edge + offset,
                        forward: !c.forward,
                        pcurve: c.pcurve.clone(),
                    })
                    .collect(),
            };
            face.wires.push(wire);
        }
        Ok(Self {
            vertices,
            edges,
            face,
            outer,
            holes,
        })
    }
    pub fn validate_boundary(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        validate_regions(self.outer, &self.holes)?;
        let Surface::Nurbs(surface) = &self.face.surface else {
            return Err(Error::Unsupported(
                "holed rational face requires a NURBS surface",
            ));
        };
        if surface.domain() != self.outer {
            return Err(Error::InvalidTopology(
                "holed surface domain disagrees with outer rectangle",
            ));
        }
        for hole in &self.holes {
            for (axis, range) in hole.iter().enumerate() {
                for parameter in *range {
                    let knots = surface.knots(axis)?;
                    let count = knots.partition_point(|v| *v <= parameter)
                        - knots.partition_point(|v| *v < parameter);
                    if count != surface.degrees()[axis] {
                        return Err(Error::InvalidTopology("NURBS hole boundaries must be retained as full-multiplicity knot lines"));
                    }
                }
            }
        }
        let expected = Self::assemble(
            (**surface).clone(),
            self.outer,
            self.holes.clone(),
            self.face.orientation,
            tol,
        )?;
        if self.vertices.len() != expected.vertices.len()
            || self.edges.len() != expected.edges.len()
            || self.face.wires.len() != expected.face.wires.len()
        {
            return Err(Error::InvalidTopology(
                "holed face boundary entity counts differ",
            ));
        }
        for (a, b) in self.vertices.iter().zip(&expected.vertices) {
            if !a.point.finite() || !tol.coincident(a.point, b.point) {
                return Err(Error::InvalidTopology(
                    "holed face vertex differs from canonical boundary",
                ));
            }
        }
        for (a, b) in self.edges.iter().zip(&expected.edges) {
            let (Curve::Nurbs(a_curve), Curve::Nurbs(b_curve)) = (&a.curve, &b.curve) else {
                return Err(Error::Unsupported(
                    "holed face requires canonical rational boundaries",
                ));
            };
            if a.vertices != b.vertices
                || a_curve.degree() != b_curve.degree()
                || a_curve.knots() != b_curve.knots()
                || a_curve.weights() != b_curve.weights()
                || a_curve.control_points().len() != b_curve.control_points().len()
                || a_curve
                    .control_points()
                    .iter()
                    .zip(b_curve.control_points())
                    .any(|(a, b)| !tol.coincident(*a, *b))
            {
                return Err(Error::InvalidTopology(
                    "holed face edge differs from canonical boundary",
                ));
            }
        }
        for edge in &self.edges {
            for (id, parameter) in edge.vertices.into_iter().zip(edge.curve.range()) {
                if !tol.coincident(self.vertices[id].point, edge.curve.try_evaluate(parameter)?) {
                    return Err(Error::InvalidTopology(
                        "holed face actual vertex disagrees with actual curve endpoint",
                    ));
                }
            }
        }
        for (a, b) in self.face.wires.iter().zip(&expected.face.wires) {
            if a.coedges.len() != b.coedges.len() {
                return Err(Error::InvalidTopology(
                    "holed face wire cardinality differs",
                ));
            }
            for (a, b) in a.coedges.iter().zip(&b.coedges) {
                let (
                    PCurve::Affine {
                        origin: ao,
                        direction: ad,
                    },
                    PCurve::Affine {
                        origin: bo,
                        direction: bd,
                    },
                ) = (&a.pcurve, &b.pcurve)
                else {
                    return Err(Error::Unsupported(
                        "holed face needs affine boundary pcurves",
                    ));
                };
                if a.edge != b.edge || a.forward != b.forward || ao != bo || ad != bd {
                    return Err(Error::InvalidTopology(
                        "holed face coedge reference, traversal or UV map differs",
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_cells: usize,
        tol: Tolerance,
    ) -> Result<NurbsSurfaceMesh> {
        self.validate_boundary(tol)?;
        let Surface::Nurbs(surface) = &self.face.surface else {
            unreachable!()
        };
        let mut output = surface.tessellate_bounded_excluding(error, max_cells, &self.holes)?;
        if self.face.orientation < 0 {
            for triangle in &mut output.mesh.triangles {
                triangle.swap(1, 2);
            }
            for normal in &mut output.mesh.normals {
                *normal = *normal * -1.;
            }
        }
        Ok(output)
    }
}
fn validate_regions(outer: [[f64; 2]; 2], holes: &[[[f64; 2]; 2]]) -> Result<()> {
    if holes.len() > 16 {
        return Err(Error::Unsupported(
            "at most 16 rectangular NURBS holes are supported",
        ));
    }
    let mut guards = [0f64; 2];
    for axis in 0..2 {
        let [a, b] = outer[axis];
        if !a.is_finite() || !b.is_finite() || a >= b || !(b - a).is_finite() {
            return Err(Error::InvalidInput(
                "NURBS outer UV rectangle must have positive finite spans",
            ));
        }
        guards[axis] = 128. * f64::EPSILON * a.abs().max(b.abs()).max(b - a).max(f64::MIN_POSITIVE);
    }
    for hole in holes {
        for axis in 0..2 {
            let [a, b] = hole[axis];
            if !a.is_finite()
                || !b.is_finite()
                || a >= b
                || !(b - a).is_finite()
                || b - a <= guards[axis]
                || a - outer[axis][0] <= guards[axis]
                || outer[axis][1] - b <= guards[axis]
            {
                return Err(Error::Unsupported(
                    "NURBS holes must be resolved rectangles strictly inside the outer UV domain",
                ));
            }
        }
    }
    for i in 0..holes.len() {
        for j in 0..i {
            if !(0..2).any(|axis| {
                (holes[i][axis][0] - holes[j][axis][1]).max(holes[j][axis][0] - holes[i][axis][1])
                    > guards[axis]
            }) {
                return Err(Error::Unsupported(
                    "NURBS holes overlap, touch or have unresolved UV separation",
                ));
            }
        }
    }
    Ok(())
}
