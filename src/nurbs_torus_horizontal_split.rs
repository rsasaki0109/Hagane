//! Exact rectangular restrictions of a regular ring torus with annular cut caps.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusHorizontalSolid {
    source: NurbsTorusSolid,
    solid: Solid,
    cut: f64,
    lower: bool,
    strips: Vec<(usize, [f64; 2])>,
}
#[derive(Clone, Debug)]
pub struct NurbsTorusHorizontalSplit {
    pub lower: NurbsTorusHorizontalSolid,
    pub upper: NurbsTorusHorizontalSolid,
}
fn unresolved() -> Error {
    Error::Unsupported("horizontal torus cut, partial patch or cap precision is unresolved")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn inverse(x: f64, y: f64) -> f64 {
    let k = y / (1. + x);
    std::f64::consts::SQRT_2 * k / (1. + (std::f64::consts::SQRT_2 - 1.) * k)
}
pub(crate) fn ranges(c: f64, r: f64, lower: bool) -> Result<Vec<(usize, [f64; 2])>> {
    if c == 0. {
        return Ok(if lower {
            vec![(2, [0., 1.]), (3, [0., 1.])]
        } else {
            vec![(0, [0., 1.]), (1, [0., 1.])]
        });
    }
    let s = (c / r).abs();
    let x = ((1. - s) * (1. + s)).sqrt();
    let a = inverse(x, s);
    let b = inverse(s, x);
    if !a.is_finite() || !b.is_finite() || !(0. ..1.).contains(&a) || !(0. ..1.).contains(&b) {
        return Err(unresolved());
    }
    Ok(match (c > 0., lower) {
        (true, true) => vec![(1, [b, 1.]), (2, [0., 1.]), (3, [0., 1.]), (0, [0., a])],
        (true, false) => vec![(0, [a, 1.]), (1, [0., b])],
        (false, true) => vec![(2, [a, 1.]), (3, [0., b])],
        (false, false) => vec![(3, [b, 1.]), (0, [0., 1.]), (1, [0., 1.]), (2, [0., a])],
    })
}
pub(crate) fn parameters(source: &NurbsTorusSolid, c: f64, p: GeometryTolerance) -> Result<f64> {
    source.validate(p)?;
    let [major, minor] = source.radii();
    if !c.is_finite() || c.abs() >= minor {
        return Err(Error::InvalidInput(
            "horizontal torus cut must be finite and strictly inside the tube height",
        ));
    }
    let scale = 2. * (major + minor).hypot(minor);
    let band = p.length_at_scale(scale)?;
    let o = source.frame().origin();
    let error = 65536. * f64::EPSILON * (scale + o.x.abs().max(o.y.abs()).max(o.z.abs()));
    if !error.is_finite()
        || error >= p.linear() / 16.
        || minor - c.abs() <= 10. * band + error
        || (c != 0. && c.abs() <= 10. * band + error)
    {
        return Err(unresolved());
    }
    let a = minor * ((1. - c / minor) * (1. + c / minor)).sqrt();
    if !a.is_finite() || 2. * a <= 10. * band + error {
        return Err(unresolved());
    }
    for lower in [true, false] {
        for (q, [lo, hi]) in ranges(c, minor, lower)? {
            let surface = &source.solid().shell.faces[q].surface;
            let chord = norm(surface.try_evaluate(0., hi)? - surface.try_evaluate(0., lo)?);
            if !chord.is_finite() || chord <= 10. * band + error {
                return Err(unresolved());
            }
        }
    }
    Ok(error)
}
fn surface(source: &NurbsTorusSolid, i: usize, q: usize) -> Result<&NurbsSurface> {
    match &source.solid().shell.faces[i * 4 + q].surface {
        Surface::Nurbs(s) => Ok(s),
        _ => Err(unresolved()),
    }
}
pub(crate) fn cap_curve(curve: &NurbsCurve, plane: &Surface) -> Result<NurbsCurve> {
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(unresolved());
    };
    let a = u.dot(u);
    let b = u.dot(v);
    let d = v.dot(v);
    let determinant = a * d - b * b;
    if !determinant.is_normal() || determinant <= 0. {
        return Err(unresolved());
    }
    let points = curve
        .control_points()
        .iter()
        .map(|p| {
            let delta = *p - origin;
            let x = delta.dot(u);
            let y = delta.dot(v);
            Point3::new(
                (d * x - b * y) / determinant,
                (a * y - b * x) / determinant,
                0.,
            )
        })
        .collect();
    NurbsCurve::new(
        curve.degree(),
        curve.knots().to_vec(),
        points,
        curve.weights().to_vec(),
    )
}
pub(crate) fn build_strip_walls(
    source: &NurbsTorusSolid,
    strips: &[(usize, [f64; 2])],
) -> Result<Solid> {
    let n = strips.len();
    let mut s = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    for i in 0..4 {
        for j in 0..=n {
            let (q, v) = if j < n {
                (strips[j].0, strips[j].1[0])
            } else {
                (strips[n - 1].0, strips[n - 1].1[1])
            };
            let curve = surface(source, i, q)?.parameter_curve([0., v], [1., v])?;
            s.vertices.push(Vertex {
                point: curve.control_points()[0],
            });
            s.edges.push(Edge {
                vertices: [i * (n + 1) + j, ((i + 1) % 4) * (n + 1) + j],
                curve: Curve::Nurbs(Box::new(curve)),
            });
        }
    }
    let offset = 4 * (n + 1);
    for i in 0..4 {
        for (k, (q, [a, b])) in strips.iter().enumerate() {
            s.edges.push(Edge {
                vertices: [i * (n + 1) + k, i * (n + 1) + k + 1],
                curve: Curve::Nurbs(Box::new(
                    surface(source, i, *q)?.parameter_curve([0., *a], [0., *b])?,
                )),
            });
        }
    }
    for i in 0..4 {
        for (k, (q, [a, b])) in strips.iter().enumerate() {
            let c = |edge, forward, origin, direction| Coedge {
                edge,
                forward,
                pcurve: PCurve::Affine { origin, direction },
            };
            s.shell.faces.push(Face {
                surface: Surface::Nurbs(Box::new(
                    surface(source, i, *q)?.restricted([[0., 1.], [*a, *b]])?,
                )),
                orientation: 1,
                wires: vec![Wire {
                    coedges: vec![
                        c(i * (n + 1) + k, true, [0., *a], [1., 0.]),
                        c(offset + ((i + 1) % 4) * n + k, true, [1., *a], [0., b - a]),
                        c(i * (n + 1) + k + 1, false, [0., *b], [1., 0.]),
                        c(offset + i * n + k, false, [0., *a], [0., b - a]),
                    ],
                }],
            });
        }
    }
    Ok(s)
}
fn build(
    source: &NurbsTorusSolid,
    c: f64,
    lower: bool,
    strips: &[(usize, [f64; 2])],
) -> Result<Solid> {
    let n = strips.len();
    let mut s = build_strip_walls(source, strips)?;
    let frame = source.frame();
    let plane = Surface::Plane {
        origin: frame.point(Point3::new(0., 0., c)),
        u: frame.axes()[0],
        v: frame.axes()[1],
    };
    let mut wires = Vec::new();
    for outer in [true, false] {
        let j = if outer == lower { n } else { 0 };
        let indices: Vec<usize> = if outer {
            (0..4).collect()
        } else {
            (0..4).rev().collect()
        };
        let mut coedges = Vec::new();
        for i in indices {
            let edge = i * (n + 1) + j;
            let Curve::Nurbs(curve) = &s.edges[edge].curve else {
                return Err(unresolved());
            };
            coedges.push(Coedge {
                edge,
                forward: outer,
                pcurve: PCurve::Nurbs(Box::new(cap_curve(curve, &plane)?)),
            });
        }
        wires.push(Wire { coedges });
    }
    s.shell.faces.push(Face {
        surface: plane,
        orientation: if lower { 1 } else { -1 },
        wires,
    });
    Ok(s)
}
impl NurbsTorusSolid {
    pub fn split_by_local_z(
        &self,
        c: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusHorizontalSplit> {
        parameters(self, c, policy)?;
        let lower = NurbsTorusHorizontalSolid::new(self, c, true, policy)?;
        let upper = NurbsTorusHorizontalSolid::new(self, c, false, policy)?;
        let total = self.volume(policy)?;
        let sum = lower.volume(policy)? / total + upper.volume(policy)? / total;
        if !sum.is_finite() || (sum - 1.).abs() > 65536. * f64::EPSILON {
            return Err(unresolved());
        }
        Ok(NurbsTorusHorizontalSplit { lower, upper })
    }
}
impl NurbsTorusHorizontalSolid {
    fn new(
        source: &NurbsTorusSolid,
        c: f64,
        lower: bool,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let strips = ranges(c, source.radii()[1], lower)?;
        let solid = build(source, c, lower, &strips)?;
        let body = Self {
            source: source.clone(),
            solid,
            cut: c,
            lower,
            strips,
        };
        body.validate(policy)?;
        body.volume(policy)?;
        Ok(body)
    }
    pub fn source(&self) -> &NurbsTorusSolid {
        &self.source
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn cut_height(&self) -> f64 {
        self.cut
    }
    pub fn is_lower(&self) -> bool {
        self.lower
    }
    pub(crate) fn strip_ranges(&self) -> &[(usize, [f64; 2])] {
        &self.strips
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        let error = parameters(&self.source, self.cut, policy)?;
        if self.strips != ranges(self.cut, self.source.radii()[1], self.lower)?
            || !crate::nurbs_frustum::same_body(
                &self.solid,
                &build(&self.source, self.cut, self.lower, &self.strips)?,
            )
        {
            return Err(Error::InvalidTopology("horizontal torus differs from complete source-restriction and annular-cap certificate"));
        }
        let mut uses = vec![(0usize, 0i32); self.solid.edges.len()];
        for f in &self.solid.shell.faces {
            for wire in &f.wires {
                for coedge in &wire.coedges {
                    uses[coedge.edge].0 += 1;
                    uses[coedge.edge].1 +=
                        i32::from(f.orientation) * if coedge.forward { 1 } else { -1 };
                    if let (Surface::Plane { .. }, PCurve::Nurbs(uv), Curve::Nurbs(edge)) = (
                        &f.surface,
                        &coedge.pcurve,
                        &self.solid.edges[coedge.edge].curve,
                    ) {
                        if uv.weights() != edge.weights() || uv.knots() != edge.knots() {
                            return Err(unresolved());
                        }
                        for (uv, p) in uv.control_points().iter().zip(edge.control_points()) {
                            if norm(f.surface.try_evaluate(uv.x, uv.y)? - *p) + error
                                >= policy.linear() / 8.
                            {
                                return Err(unresolved());
                            }
                        }
                    }
                    for t in [0., 0.25, 0.5, 0.75, 1.] {
                        let uv = coedge.pcurve.try_evaluate(t)?;
                        if norm(
                            f.surface.try_evaluate(uv[0], uv[1])?
                                - self.solid.edges[coedge.edge].curve.try_evaluate(t)?,
                        ) + error
                            >= policy.linear() / 4.
                        {
                            return Err(unresolved());
                        }
                    }
                }
            }
        }
        if uses.iter().any(|&(n, sign)| n != 2 || sign != 0) {
            return Err(Error::InvalidTopology(
                "horizontal torus shell is not closed and oriented",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let [major, minor] = self.source.radii();
        let s = self.cut / minor;
        let x = ((1. - s) * (1. + s)).sqrt();
        let theta = x.atan2(if self.lower { -s } else { s });
        let area = if theta < 0.05 {
            let t = theta * theta;
            theta
                * theta
                * theta
                * (2. / 3.
                    + t * (-2. / 15. + t * (4. / 315. + t * (-2. / 2835. + t * (4. / 155925.)))))
        } else {
            theta - theta.sin() * theta.cos()
        };
        let v = product(&[2. * std::f64::consts::PI, major, minor, minor, area])?;
        Ok(v)
    }
    /// Conservative hull bound of the actual positive-weight retained patches.
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for face in &self.solid.shell.faces {
            if let Surface::Nurbs(s) = &face.surface {
                for p in s.control_points() {
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    min.z = min.z.min(p.z);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                    max.z = max.z.max(p.z);
                }
            }
        }
        Ok(Bounds { min, max })
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, Tolerance::new(policy.linear())?)
    }
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
