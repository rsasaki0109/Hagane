//! Closed regular ring torus represented by sixteen exact rational tensor patches.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusSolid {
    solid: Solid,
    frame: Frame3,
    radii: [f64; 2],
}
const Q: [[[f64; 2]; 3]; 4] = [
    [[1., 0.], [1., 1.], [0., 1.]],
    [[0., 1.], [-1., 1.], [-1., 0.]],
    [[-1., 0.], [-1., -1.], [0., -1.]],
    [[0., -1.], [1., -1.], [1., 0.]],
];
fn knots() -> Vec<f64> {
    vec![0., 0., 0., 1., 1., 1.]
}
fn parameters(frame: Frame3, radii: [f64; 2], p: GeometryTolerance) -> Result<f64> {
    let checked = Frame3::new_with_tolerance(frame.origin(), frame.axes(), p)?;
    let [major, minor] = radii;
    if radii.into_iter().any(|r| !r.is_normal() || r <= 0.) || major <= minor {
        return Err(Error::InvalidInput(
            "torus requires positive resolved radii with major radius greater than minor radius",
        ));
    }
    let scale = 2. * (major + minor).hypot(minor);
    let o = frame.origin();
    let world = o.x.abs().max(o.y.abs()).max(o.z.abs()) + 2. * (major + minor);
    let arithmetic = 32768. * f64::EPSILON * world;
    let drift = frame
        .axes()
        .into_iter()
        .zip(checked.axes())
        .map(|(a, b)| (a - b).norm())
        .fold(0., f64::max)
        * scale;
    if !scale.is_finite()
        || !arithmetic.is_finite()
        || !drift.is_finite()
        || arithmetic + drift >= p.linear() / 8.
    {
        return Err(Error::Unsupported(
            "torus frame or world-coordinate precision is unresolved",
        ));
    }
    let band = p.length_at_scale(scale)?;
    if minor <= 10. * band + arithmetic || major - minor <= 10. * band + arithmetic {
        return Err(Error::Unsupported(
            "torus tube or ring clearance is unresolved; horn and spindle tori are unsupported",
        ));
    }
    Ok(arithmetic + drift)
}
fn build(frame: Frame3, [major, minor]: [f64; 2]) -> Result<Solid> {
    let w = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
    let pole = |u: [f64; 2], v: [f64; 2]| {
        frame.point(Point3::new(
            (major + minor * v[0]) * u[0],
            (major + minor * v[0]) * u[1],
            minor * v[1],
        ))
    };
    let mut s = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    for u in Q {
        for v in Q {
            s.vertices.push(Vertex {
                point: pole(u[0], v[0]),
            });
        }
    }
    for (i, qi) in Q.iter().enumerate() {
        for (j, qj) in Q.iter().enumerate() {
            s.edges.push(Edge {
                vertices: [i * 4 + j, ((i + 1) % 4) * 4 + j],
                curve: Curve::Nurbs(Box::new(NurbsCurve::new(
                    2,
                    knots(),
                    qi.map(|u| pole(u, qj[0])).to_vec(),
                    w.to_vec(),
                )?)),
            });
        }
    }
    for (i, qi) in Q.iter().enumerate() {
        for (j, qj) in Q.iter().enumerate() {
            s.edges.push(Edge {
                vertices: [i * 4 + j, i * 4 + (j + 1) % 4],
                curve: Curve::Nurbs(Box::new(NurbsCurve::new(
                    2,
                    knots(),
                    qj.map(|v| pole(qi[0], v)).to_vec(),
                    w.to_vec(),
                )?)),
            });
        }
    }
    for (i, qi) in Q.iter().enumerate() {
        for (j, qj) in Q.iter().enumerate() {
            let mut points = Vec::new();
            let mut weights = Vec::new();
            for (a, u) in qi.iter().copied().enumerate() {
                for (b, v) in qj.iter().copied().enumerate() {
                    points.push(pole(u, v));
                    weights.push(w[a] * w[b]);
                }
            }
            let c = |edge, forward, origin, direction| Coedge {
                edge,
                forward,
                pcurve: PCurve::Affine { origin, direction },
            };
            s.shell.faces.push(Face {
                surface: Surface::Nurbs(Box::new(NurbsSurface::new(
                    [2, 2],
                    [knots(), knots()],
                    [3, 3],
                    points,
                    weights,
                )?)),
                orientation: 1,
                wires: vec![Wire {
                    coedges: vec![
                        c(i * 4 + j, true, [0., 0.], [1., 0.]),
                        c(16 + ((i + 1) % 4) * 4 + j, true, [1., 0.], [0., 1.]),
                        c(i * 4 + (j + 1) % 4, false, [0., 1.], [1., 0.]),
                        c(16 + i * 4 + j, false, [0., 0.], [0., 1.]),
                    ],
                }],
            });
        }
    }
    Ok(s)
}
fn product(factors: &[f64]) -> Result<f64> {
    let mut mantissa = 1.;
    let mut exponent = 0i32;
    for &value in factors {
        if !value.is_finite() || value <= 0. {
            return Err(Error::Unsupported(
                "torus volume arithmetic overflows or underflows",
            ));
        }
        let (value, adjustment) = if value < f64::MIN_POSITIVE {
            (value * 4503599627370496., -52)
        } else {
            (value, 0)
        };
        let bits = value.to_bits();
        exponent += ((bits >> 52) & 2047) as i32 - 1023 + adjustment;
        mantissa *= f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
        if mantissa >= 2. {
            mantissa *= 0.5;
            exponent += 1;
        }
    }
    if !(-1022..=1023).contains(&exponent) {
        return Err(Error::Unsupported(
            "torus volume arithmetic overflows or underflows",
        ));
    }
    let result = mantissa * f64::from_bits(((exponent + 1023) as u64) << 52);
    if !result.is_finite() || result < f64::MIN_POSITIVE {
        return Err(Error::Unsupported(
            "torus volume arithmetic overflows or underflows",
        ));
    }
    Ok(result)
}

impl NurbsTorusSolid {
    pub fn new(frame: Frame3, radii: [f64; 2], policy: GeometryTolerance) -> Result<Self> {
        parameters(frame, radii, policy)?;
        Self::from_brep(build(frame, radii)?, frame, radii, policy)
    }
    /// Retain supplied geometry only if the complete canonical certificate matches.
    pub fn from_brep(
        solid: Solid,
        frame: Frame3,
        radii: [f64; 2],
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let body = Self {
            solid,
            frame,
            radii,
        };
        body.validate(policy)?;
        Ok(body)
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn frame(&self) -> Frame3 {
        self.frame
    }
    pub fn radii(&self) -> [f64; 2] {
        self.radii
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        let arithmetic = parameters(self.frame, self.radii, policy)?;
        if !crate::nurbs_frustum::same_body(&self.solid, &build(self.frame, self.radii)?) {
            return Err(Error::InvalidTopology(
                "torus differs from its complete canonical rational B-rep certificate",
            ));
        }
        let mut uses = [(0usize, 0i32); 32];
        for f in &self.solid.shell.faces {
            for c in &f.wires[0].coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += if c.forward { 1 } else { -1 };
                for t in [0., 0.25, 0.5, 0.75, 1.] {
                    let uv = c.pcurve.try_evaluate(t)?;
                    let delta = f.surface.try_evaluate(uv[0], uv[1])?
                        - self.solid.edges[c.edge].curve.try_evaluate(t)?;
                    if !delta.finite()
                        || delta.x.hypot(delta.y).hypot(delta.z) + arithmetic
                            >= policy.linear() / 4.
                    {
                        return Err(Error::Unsupported(
                            "torus same-parameter world precision is unresolved",
                        ));
                    }
                }
            }
        }
        if uses.iter().any(|&(count, sign)| count != 2 || sign != 0) {
            return Err(Error::InvalidTopology(
                "torus boundary is not closed and oriented",
            ));
        }
        Ok(())
    }
    /// Export the validated retained rational geometry, with no import-domain claim.
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, Tolerance::new(policy.linear())?)
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let [major, minor] = self.radii;
        let value = product(&[2. * std::f64::consts::PI.powi(2), major, minor, minor])?;
        Ok(value)
    }
}
