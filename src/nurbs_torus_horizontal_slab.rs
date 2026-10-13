//! Two annular caps and actual outer/inner source-patch chains bound a torus band.
use crate::nurbs_torus_horizontal_split::{build_strip_walls, cap_curve, parameters, ranges};
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusHorizontalSlab {
    source: NurbsTorusSolid,
    heights: [f64; 2],
    solid: Solid,
    outer: Vec<(usize, [f64; 2])>,
    inner: Vec<(usize, [f64; 2])>,
}
fn unresolved() -> Error {
    Error::Unsupported("horizontal torus slab height, patch, volume or precision is unresolved")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn boundary(c: f64, r: f64, outer: bool) -> Result<f64> {
    let rows = ranges(c, r, true)?;
    Ok(if outer {
        rows[rows.len() - 1].1[1]
    } else {
        rows[0].1[0]
    })
}
type Chains = (Vec<(usize, [f64; 2])>, Vec<(usize, [f64; 2])>);
fn chains([lo, hi]: [f64; 2], r: f64) -> Result<Chains> {
    let outer = if lo >= 0. {
        vec![(
            0,
            [
                if lo == 0. { 0. } else { boundary(lo, r, true)? },
                boundary(hi, r, true)?,
            ],
        )]
    } else if hi <= 0. {
        vec![(
            3,
            [
                boundary(lo, r, true)?,
                if hi == 0. { 1. } else { boundary(hi, r, true)? },
            ],
        )]
    } else {
        vec![
            (3, [boundary(lo, r, true)?, 1.]),
            (0, [0., boundary(hi, r, true)?]),
        ]
    };
    let inner = if lo >= 0. {
        vec![(
            1,
            [
                boundary(hi, r, false)?,
                if lo == 0. {
                    1.
                } else {
                    boundary(lo, r, false)?
                },
            ],
        )]
    } else if hi <= 0. {
        vec![(
            2,
            [
                if hi == 0. {
                    0.
                } else {
                    boundary(hi, r, false)?
                },
                boundary(lo, r, false)?,
            ],
        )]
    } else {
        vec![
            (1, [boundary(hi, r, false)?, 1.]),
            (2, [0., boundary(lo, r, false)?]),
        ]
    };
    Ok((outer, inner))
}
fn admission(source: &NurbsTorusSolid, heights: [f64; 2], p: GeometryTolerance) -> Result<f64> {
    source.validate(p)?;
    let [lo, hi] = heights;
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return Err(Error::InvalidInput(
            "torus slab heights must be finite and strictly increasing",
        ));
    }
    let error = parameters(source, lo, p)?.max(parameters(source, hi, p)?);
    let [major, minor] = source.radii();
    let band = p.length_at_scale(2. * (major + minor).hypot(minor))?;
    if !(hi - lo).is_finite() || hi - lo <= 10. * band + error {
        return Err(unresolved());
    }
    let (outer, inner) = chains(heights, minor)?;
    for (q, [a, b]) in outer.iter().chain(&inner) {
        let face = &source.solid().shell.faces[*q].surface;
        let chord = norm(face.try_evaluate(0., *b)? - face.try_evaluate(0., *a)?);
        if !chord.is_finite() || chord <= 10. * band + error {
            return Err(unresolved());
        }
    }
    Ok(error)
}
// NEW slab meridians retain both exact UV endpoints. The affine difference
// a+(b-a) can round one ULP outside a narrow restricted surface domain; a
// positive linear Bernstein pcurve encodes the closed interval without clamps.
fn closed_meridian_pcurves(solid: &mut Solid) -> Result<()> {
    for face in &mut solid.shell.faces {
        let Surface::Nurbs(surface) = &face.surface else {
            return Err(unresolved());
        };
        let [a, b] = surface.domain()[1];
        for wire in &mut face.wires {
            for c in &mut wire.coedges {
                if let PCurve::Affine { origin, direction } = &c.pcurve {
                    if direction[0] == 0. && direction[1] != 0. {
                        c.pcurve = PCurve::Nurbs(Box::new(NurbsCurve::new(
                            1,
                            vec![0., 0., 1., 1.],
                            vec![Point3::new(origin[0], a, 0.), Point3::new(origin[0], b, 0.)],
                            vec![1., 1.],
                        )?));
                    }
                }
            }
        }
    }
    Ok(())
}
fn build(
    source: &NurbsTorusSolid,
    heights: [f64; 2],
    outer: &[(usize, [f64; 2])],
    inner: &[(usize, [f64; 2])],
) -> Result<Solid> {
    let mut solid = build_strip_walls(source, outer)?;
    closed_meridian_pcurves(&mut solid)?;
    let no = outer.len();
    let ni = inner.len();
    let vo = solid.vertices.len();
    let eo = solid.edges.len();
    let mut other = build_strip_walls(source, inner)?;
    closed_meridian_pcurves(&mut other)?;
    solid.vertices.extend(other.vertices);
    for mut edge in other.edges {
        edge.vertices = edge.vertices.map(|i| i + vo);
        solid.edges.push(edge);
    }
    for mut face in other.shell.faces {
        for wire in &mut face.wires {
            for c in &mut wire.coedges {
                c.edge += eo;
            }
        }
        solid.shell.faces.push(face);
    }
    let frame = source.frame();
    for (cap, height) in heights.into_iter().enumerate() {
        let plane = Surface::Plane {
            origin: frame.point(Point3::new(0., 0., height)),
            u: frame.axes()[0],
            v: frame.axes()[1],
        };
        let mut wires = Vec::new();
        for ring in 0..2 {
            let mut coedges = Vec::new();
            let indices: Vec<_> = if ring == 0 {
                (0..4).collect()
            } else {
                (0..4).rev().collect()
            };
            for i in indices {
                let edge = if ring == 0 {
                    i * (no + 1) + if cap == 0 { 0 } else { no }
                } else {
                    eo + i * (ni + 1) + if cap == 0 { ni } else { 0 }
                };
                let Curve::Nurbs(curve) = &solid.edges[edge].curve else {
                    return Err(unresolved());
                };
                coedges.push(Coedge {
                    edge,
                    forward: ring == 0,
                    pcurve: PCurve::Nurbs(Box::new(cap_curve(curve, &plane)?)),
                });
            }
            wires.push(Wire { coedges });
        }
        solid.shell.faces.push(Face {
            surface: plane,
            orientation: if cap == 0 { -1 } else { 1 },
            wires,
        });
    }
    Ok(solid)
}
impl NurbsTorusSolid {
    pub fn slab_between_local_z(
        &self,
        heights: [f64; 2],
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusHorizontalSlab> {
        admission(self, heights, policy)?;
        let (outer, inner) = chains(heights, self.radii()[1])?;
        let solid = build(self, heights, &outer, &inner)?;
        let slab = NurbsTorusHorizontalSlab {
            source: self.clone(),
            heights,
            solid,
            outer,
            inner,
        };
        slab.validate(policy)?;
        slab.volume(policy)?;
        Ok(slab)
    }
}
impl NurbsTorusHorizontalSlab {
    pub fn source(&self) -> &NurbsTorusSolid {
        &self.source
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn heights(&self) -> [f64; 2] {
        self.heights
    }
    pub(crate) fn chain_ranges(&self) -> [&[(usize, [f64; 2])]; 2] {
        [&self.outer, &self.inner]
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        let error = admission(&self.source, self.heights, policy)?;
        let (outer, inner) = chains(self.heights, self.source.radii()[1])?;
        if self.outer != outer
            || self.inner != inner
            || !crate::nurbs_frustum::same_body(
                &self.solid,
                &build(&self.source, self.heights, &outer, &inner)?,
            )
        {
            return Err(Error::InvalidTopology(
                "torus slab differs from complete source-restriction and annular-cap certificate",
            ));
        }
        let mut uses = vec![(0usize, 0i32); self.solid.edges.len()];
        for face in &self.solid.shell.faces {
            for wire in &face.wires {
                for c in &wire.coedges {
                    uses[c.edge].0 += 1;
                    uses[c.edge].1 += i32::from(face.orientation) * if c.forward { 1 } else { -1 };
                    if let (Surface::Plane { .. }, PCurve::Nurbs(uv), Curve::Nurbs(edge)) =
                        (&face.surface, &c.pcurve, &self.solid.edges[c.edge].curve)
                    {
                        if uv.weights() != edge.weights() || uv.knots() != edge.knots() {
                            return Err(unresolved());
                        }
                        for (p, uv) in edge.control_points().iter().zip(uv.control_points()) {
                            if norm(face.surface.try_evaluate(uv.x, uv.y)? - *p) + error
                                >= policy.linear() / 8.
                            {
                                return Err(unresolved());
                            }
                        }
                    }
                    for t in [0., 0.25, 0.5, 0.75, 1.] {
                        let uv = c.pcurve.try_evaluate(t)?;
                        if norm(
                            face.surface.try_evaluate(uv[0], uv[1])?
                                - self.solid.edges[c.edge].curve.try_evaluate(t)?,
                        ) + error
                            >= policy.linear() / 4.
                        {
                            return Err(unresolved());
                        }
                    }
                }
            }
        }
        if uses.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "torus slab is not closed and oriented",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let [major, minor] = self.source.radii();
        let [lo, hi] = self.heights;
        let a = lo / minor;
        let b = hi / minor;
        let ca = (((minor - lo) / minor) * ((minor + lo) / minor)).sqrt();
        let cb = (((minor - hi) / minor) * ((minor + hi) / minor)).sqrt();
        let cosine = ca * cb + a * b;
        let sine = if a < 0. && b > 0. {
            b * ca - a * cb
        } else {
            ((hi - lo) / minor) * (1. + cosine) / (ca + cb)
        };
        let delta = sine.atan2(cosine);
        if !delta.is_finite() || delta <= 0. || !sine.is_finite() || sine <= 0. {
            return Err(unresolved());
        }
        let residual = if delta < 0.05 {
            let d = delta * delta;
            delta
                * delta
                * delta
                * (1. / 6.
                    + d * (-1. / 120. + d * (1. / 5040. + d * (-1. / 362880. + d / 39916800.))))
        } else {
            delta - delta.sin()
        };
        let positive = 0.5 * ((ca + cb) * (ca + cb) + (a - b) * (a - b)) * sine;
        let area = residual + positive;
        if !area.is_normal() || area <= 0. {
            return Err(unresolved());
        }
        product(&[2. * std::f64::consts::PI, major, minor, minor, area])
    }
    /// Conservative hull including actual walls, shared boundaries and
    /// plane-projected positive-weight cap pcurve controls.
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut include = |p: Point3| {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        };
        for vertex in &self.solid.vertices {
            include(vertex.point);
        }
        for edge in &self.solid.edges {
            if let Curve::Nurbs(curve) = &edge.curve {
                for &p in curve.control_points() {
                    include(p);
                }
            }
        }
        for face in &self.solid.shell.faces {
            if let Surface::Nurbs(surface) = &face.surface {
                for &p in surface.control_points() {
                    include(p);
                }
            } else if matches!(face.surface, Surface::Plane { .. }) {
                for coedge in face.wires.iter().flat_map(|wire| &wire.coedges) {
                    if let PCurve::Nurbs(curve) = &coedge.pcurve {
                        for p in curve.control_points() {
                            include(face.surface.try_evaluate(p.x, p.y)?);
                        }
                    }
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
