//! Exact coaxial cylindrical openings in completely certified rational frusta.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumSolid {
    source: NurbsFrustumSolid,
    tool: NurbsFrustumSolid,
    solid: Solid,
    radius: f64,
}
fn assembly(source: &NurbsFrustumSolid, tool: &NurbsFrustumSolid) -> Solid {
    let mut solid = source.solid().clone();
    solid.vertices.extend(tool.solid().vertices.iter().cloned());
    solid
        .edges
        .extend(tool.solid().edges.iter().cloned().map(|mut e| {
            e.vertices = e.vertices.map(|v| v + 8);
            e
        }));
    for i in 0..2 {
        let mut wire = tool.solid().shell.faces[i].wires[0].clone();
        wire.coedges.reverse();
        for c in &mut wire.coedges {
            c.edge += 12;
            c.forward = !c.forward;
        }
        solid.shell.faces[i].wires.push(wire);
    }
    solid
        .shell
        .faces
        .extend(tool.solid().shell.faces[2..].iter().cloned().map(|mut f| {
            f.orientation = -f.orientation;
            for wire in &mut f.wires {
                for c in &mut wire.coedges {
                    c.edge += 12;
                }
            }
            f
        }));
    solid
}
impl NurbsFrustumSolid {
    pub fn bore_coaxial(
        &self,
        radius: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumSolid> {
        NurbsBoredFrustumSolid::new(self, radius, policy)
    }
}
impl NurbsBoredFrustumSolid {
    pub fn new(source: &NurbsFrustumSolid, radius: f64, policy: GeometryTolerance) -> Result<Self> {
        source.validate(policy)?;
        let tool = NurbsFrustumSolid::new(source.frame(), [radius; 2], source.height(), policy)?;
        let result = Self {
            solid: assembly(source, &tool),
            source: source.clone(),
            tool,
            radius,
        };
        result.validate(policy)?;
        Ok(result)
    }
    pub fn source(&self) -> &NurbsFrustumSolid {
        &self.source
    }
    pub fn tool(&self) -> &NurbsFrustumSolid {
        &self.tool
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        self.tool.validate(policy)?;
        let [a, b] = self.source.radii();
        let scale = (2. * a.max(b)).hypot(self.source.height());
        let band = policy.length_at_scale(scale)?;
        let origin = self.source.frame().origin();
        let world = scale
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        let arithmetic = 65536. * f64::EPSILON * world;
        if !self.radius.is_finite() || self.radius <= 0. {
            return Err(Error::InvalidInput(
                "coaxial bore radius must be positive and finite",
            ));
        }
        if arithmetic >= policy.linear() / 8.
            || self.radius <= 10. * band + arithmetic
            || a.min(b) - self.radius <= 10. * band + arithmetic
        {
            return Err(Error::Unsupported(
                "coaxial bore radius or remaining wall clearance is unresolved",
            ));
        }
        if self.tool.frame() != self.source.frame()
            || self.tool.radii() != [self.radius; 2]
            || self.tool.height() != self.source.height()
            || !crate::nurbs_frustum::same_body(&self.solid, &assembly(&self.source, &self.tool))
        {
            return Err(Error::InvalidTopology(
                "bored frustum differs from its complete canonical annular B-rep certificate",
            ));
        }
        let mut uses = [(0usize, 0i32); 24];
        for f in &self.solid.shell.faces {
            for wire in &f.wires {
                for c in &wire.coedges {
                    uses[c.edge].0 += 1;
                    uses[c.edge].1 += i32::from(f.orientation) * if c.forward { 1 } else { -1 };
                    // Cap rational pcurves and spatial rims have the same positive
                    // weights: control-point residual bounds the entire edge.
                    if let (Surface::Plane { .. }, PCurve::Nurbs(uv), Curve::Nurbs(curve)) =
                        (&f.surface, &c.pcurve, &self.solid.edges[c.edge].curve)
                    {
                        if uv.weights() != curve.weights()
                            || uv.knots() != curve.knots()
                            || uv.degree() != curve.degree()
                        {
                            return Err(Error::InvalidTopology(
                                "annular cap parameter basis differs from its actual rim",
                            ));
                        }
                        for (p, q) in uv.control_points().iter().zip(curve.control_points()) {
                            let d = f.surface.try_evaluate(p.x, p.y)? - *q;
                            if !d.finite()
                                || d.x.hypot(d.y).hypot(d.z) + arithmetic >= policy.linear() / 4.
                            {
                                return Err(Error::Unsupported(
                                    "annular cap continuous edge precision is unresolved",
                                ));
                            }
                        }
                    }
                    for t in [0., 0.25, 0.5, 0.75, 1.] {
                        let uv = c.pcurve.try_evaluate(t)?;
                        let d = f.surface.try_evaluate(uv[0], uv[1])?
                            - self.solid.edges[c.edge].curve.try_evaluate(t)?;
                        if !d.finite()
                            || d.x.hypot(d.y).hypot(d.z) + arithmetic >= policy.linear() / 4.
                        {
                            return Err(Error::Unsupported(
                                "bored frustum same-parameter precision is unresolved",
                            ));
                        }
                    }
                }
            }
        }
        if uses.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "bored frustum boundary is not closed and oriented",
            ));
        }
        Ok(())
    }
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        self.source.bounds(policy)
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
}
