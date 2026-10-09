//! An exact affine-UV rational curve retained with existing B-rep edge entities.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsSurfaceEdge {
    pub surface: NurbsSurface,
    pub vertices: [Vertex; 2],
    pub edge: Edge,
    pub pcurve: PCurve,
    endpoints: [[f64; 2]; 2],
}
impl NurbsSurfaceEdge {
    pub fn new(
        surface: NurbsSurface,
        start: [f64; 2],
        end: [f64; 2],
        tol: Tolerance,
    ) -> Result<Self> {
        check_parameter_identity(&surface, [start, end], tol)?;
        let curve = surface.parameter_curve(start, end)?;
        let vertices = [
            Vertex {
                point: surface.evaluate(start[0], start[1])?,
            },
            Vertex {
                point: surface.evaluate(end[0], end[1])?,
            },
        ];
        let edge = Edge {
            vertices: [0, 1],
            curve: Curve::Nurbs(Box::new(curve)),
        };
        let pcurve = PCurve::Affine {
            origin: start,
            direction: [end[0] - start[0], end[1] - start[1]],
        };
        let result = Self {
            surface,
            vertices,
            edge,
            pcurve,
            endpoints: [start, end],
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub fn endpoints(&self) -> [[f64; 2]; 2] {
        self.endpoints
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        let identity_budget = check_parameter_identity(&self.surface, self.endpoints, tol)?;
        let [start, end] = self.endpoints;
        let expected = self.surface.parameter_curve(start, end)?;
        let Curve::Nurbs(curve) = &self.edge.curve else {
            return Err(Error::Unsupported(
                "surface edge requires a canonical rational curve",
            ));
        };
        let PCurve::Affine { origin, direction } = &self.pcurve else {
            return Err(Error::Unsupported(
                "surface edge requires an affine UV pcurve",
            ));
        };
        if self.edge.vertices != [0, 1]
            || *origin != start
            || *direction != [end[0] - start[0], end[1] - start[1]]
            || curve.degree() != expected.degree()
            || curve.knots() != expected.knots()
            || curve.weights() != expected.weights()
            || curve.control_points().len() != expected.control_points().len()
        {
            return Err(Error::InvalidTopology(
                "surface edge differs from its canonical affine-UV curve",
            ));
        }
        // Same rational basis and positive weights make the control displacement
        // a curve displacement bound. Reserve UV arithmetic before admitting it.
        let maximum_gap = curve
            .control_points()
            .iter()
            .zip(expected.control_points())
            .map(|(a, b)| (*a - *b).norm())
            .fold(0f64, f64::max);
        if !maximum_gap.is_finite() || maximum_gap + identity_budget > tol.linear {
            return Err(Error::InvalidTopology(
                "surface edge control displacement exhausts parameter identity tolerance",
            ));
        }
        for (i, uv) in self.endpoints.iter().enumerate() {
            let point = self.vertices[i].point;
            let mapped = self.pcurve.evaluate(i as f64);
            if !point.finite()
                || !tol.coincident(point, self.surface.evaluate(mapped[0], mapped[1])?)
                || !tol.coincident(point, self.surface.evaluate(uv[0], uv[1])?)
                || !tol.coincident(point, curve.evaluate(i as f64)?)
            {
                return Err(Error::InvalidTopology(
                    "surface edge vertex disagrees with curve or surface endpoint",
                ));
            }
        }
        Ok(())
    }
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_segments: usize,
        tol: Tolerance,
    ) -> Result<NurbsPolyline> {
        self.validate(tol)?;
        let Curve::Nurbs(curve) = &self.edge.curve else {
            unreachable!()
        };
        curve.tessellate_bounded(error, max_segments)
    }
}

// A floating affine pcurve has parameter roundoff even when its mathematical
// composition is exact. Bound the spatial impact with positive rational basis
// derivative envelopes; do not confuse UV units with physical length tolerance.
fn check_parameter_identity(
    surface: &NurbsSurface,
    endpoints: [[f64; 2]; 2],
    tol: Tolerance,
) -> Result<f64> {
    Tolerance::new(tol.linear)?;
    let [start, end] = endpoints;
    if start.into_iter().chain(end).any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "surface edge needs finite UV endpoints",
        ));
    }
    let first = surface.control_points()[0];
    let mut lo = first;
    let mut hi = first;
    for p in surface.control_points() {
        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        lo.z = lo.z.min(p.z);
        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
        hi.z = hi.z.max(p.z);
    }
    let diameter = (hi - lo).norm();
    let min_weight = surface
        .weights()
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let max_weight = surface.weights().iter().copied().fold(0., f64::max);
    let ratio = max_weight / min_weight;
    if !diameter.is_finite() || !ratio.is_finite() {
        return Err(Error::Unsupported(
            "surface edge parameter identity exceeds numerical range",
        ));
    }
    let world_scale = surface
        .control_points()
        .iter()
        .flat_map(|point| [point.x.abs(), point.y.abs(), point.z.abs()])
        .fold(0f64, f64::max)
        .max(f64::MIN_POSITIVE);
    // Homogeneous evaluation and control restoration also round in world
    // coordinates, even for fixed UV axes. Reserve this independently of UV.
    let mut budget = 4096.
        * f64::EPSILON
        * world_scale
        * ratio
        * (surface.degrees()[0] + surface.degrees()[1] + 2) as f64;

    for axis in 0..2 {
        let delta = end[axis] - start[axis];
        if delta == 0. {
            continue;
        }
        if !delta.is_finite() {
            return Err(Error::InvalidInput(
                "surface edge UV direction exceeds finite range",
            ));
        }
        let parameter_scale = start[axis]
            .abs()
            .max(end[axis].abs())
            .max(delta.abs())
            .max(f64::MIN_POSITIVE);
        let parameter_roundoff = 16. * f64::EPSILON * parameter_scale;
        let min_span = surface
            .knots(axis)?
            .windows(2)
            .filter(|pair| pair[0] < pair[1])
            .map(|pair| pair[1] - pair[0])
            .fold(f64::INFINITY, f64::min);
        let derivative_bound = 2. * surface.degrees()[axis] as f64 * (diameter / min_span) * ratio;
        budget += parameter_roundoff * derivative_bound;
    }
    if !budget.is_finite() || budget > tol.linear {
        return Err(Error::Unsupported(
            "affine UV roundoff cannot preserve surface edge identity within physical tolerance",
        ));
    }
    Ok(budget)
}
