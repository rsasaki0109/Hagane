//! Repeated exact-basis parallel sections of an original rational frustum.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsFrustumSlabSolid {
    source: NurbsFrustumSolid,
    lower: Option<NurbsFrustumPlaneSection>,
    upper: Option<NurbsFrustumPlaneSection>,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumParallelSplit {
    pub lower: NurbsFrustumSlabSolid,
    pub upper: NurbsFrustumSlabSolid,
}
fn unresolved() -> Error {
    Error::Unsupported("parallel frustum slab geometry, clearance or arithmetic is unresolved")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
pub(crate) fn build_between(
    source: &NurbsFrustumSolid,
    lower: Option<&NurbsFrustumPlaneSection>,
    upper: Option<&NurbsFrustumPlaneSection>,
    policy: GeometryTolerance,
) -> Result<Solid> {
    let mut curves = vec![];
    for layer in 0..2 {
        for q in 0..4 {
            let boundary = if layer == 0 { lower } else { upper };
            let curve = if let Some(section) = boundary {
                &section.edges()[q].curve
            } else {
                &source.solid().edges[layer * 4 + q].curve
            };
            let Curve::Nurbs(c) = curve else {
                return Err(unresolved());
            };
            curves.push(c.as_ref().clone());
        }
    }
    let vertices = (0..8)
        .map(|i| Vertex {
            point: curves[i].control_points()[0],
        })
        .collect::<Vec<_>>();
    let mut edges = vec![];
    for (i, c) in curves.iter().enumerate() {
        let layer = i / 4;
        let q = i % 4;
        edges.push(Edge {
            vertices: [i, layer * 4 + (q + 1) % 4],
            curve: Curve::Nurbs(Box::new(c.clone())),
        });
    }
    for q in 0..4 {
        edges.push(Edge {
            vertices: [q, 4 + q],
            curve: Curve::Line {
                a: vertices[q].point,
                b: vertices[4 + q].point,
            },
        });
    }

    let mut faces = vec![];
    for layer in 0..2 {
        let boundary = if layer == 0 { lower } else { upper };
        let surface = if let Some(section) = boundary {
            crate::nurbs_frustum_oblique_split::cap(source, section, policy)?
        } else {
            source.solid().shell.faces[layer].surface.clone()
        };
        let Surface::Plane { origin, u, v } = surface else {
            return Err(unresolved());
        };
        let mut coedges = vec![];
        for q in 0..4 {
            if boundary.is_none() {
                coedges.push(source.solid().shell.faces[layer].wires[0].coedges[q].clone());
                continue;
            }
            let c = &curves[layer * 4 + q];
            let points = c
                .control_points()
                .iter()
                .map(|p| {
                    let d = *p - origin;
                    Point3::new(d.dot(u), d.dot(v), 0.)
                })
                .collect();
            let uv = NurbsCurve::new(2, c.knots().to_vec(), points, c.weights().to_vec())?;
            coedges.push(Coedge {
                edge: layer * 4 + q,
                forward: true,
                pcurve: PCurve::nurbs(uv)?,
            });
        }
        faces.push(Face {
            surface: Surface::Plane { origin, u, v },
            orientation: if layer == 0 { -1 } else { 1 },
            wires: vec![Wire { coedges }],
        });
    }
    for q in 0..4 {
        let a = &curves[q];
        let b = &curves[4 + q];
        let mut points = vec![];
        let mut weights = vec![];
        for i in 0..3 {
            points.extend([a.control_points()[i], b.control_points()[i]]);
            weights.extend([a.weights()[i], b.weights()[i]]);
        }
        let surface = NurbsSurface::new(
            [2, 1],
            [a.knots().to_vec(), vec![0., 0., 1., 1.]],
            [3, 2],
            points,
            weights,
        )?;
        let affine = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        let generator = |edge, forward, u, w0, w1| -> Result<Coedge> {
            Ok(Coedge {
                edge,
                forward,
                pcurve: PCurve::nurbs(NurbsCurve::new(
                    1,
                    vec![0., 0., 1., 1.],
                    vec![Point3::new(u, 0., 0.), Point3::new(u, 1., 0.)],
                    vec![w1, w0],
                )?)?,
            })
        };
        faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            orientation: 1,
            wires: vec![Wire {
                coedges: vec![
                    affine(q, true, [0., 0.], [1., 0.]),
                    generator(8 + (q + 1) % 4, true, 1., a.weights()[2], b.weights()[2])?,
                    affine(4 + q, false, [0., 1.], [1., 0.]),
                    generator(8 + q, false, 0., a.weights()[0], b.weights()[0])?,
                ],
            }],
        });
    }
    Ok(Solid {
        vertices,
        edges,
        shell: Shell { faces },
    })
}
impl NurbsFrustumSlabSolid {
    pub(crate) fn new(
        source: &NurbsFrustumSolid,
        lower: Option<NurbsFrustumPlaneSection>,
        upper: Option<NurbsFrustumPlaneSection>,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let result = Self {
            solid: build_between(source, lower.as_ref(), upper.as_ref(), policy)?,
            source: source.clone(),
            lower,
            upper,
        };
        result.validate(policy)?;
        result.volume(policy)?;
        Ok(result)
    }
    pub fn source(&self) -> &NurbsFrustumSolid {
        &self.source
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn lower_section(&self) -> Option<&NurbsFrustumPlaneSection> {
        self.lower.as_ref()
    }
    pub fn upper_section(&self) -> Option<&NurbsFrustumPlaneSection> {
        self.upper.as_ref()
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        for section in [self.lower.as_ref(), self.upper.as_ref()]
            .into_iter()
            .flatten()
        {
            section.validate(policy)?;
        }
        if let (Some(lower), Some(upper)) = (&self.lower, &self.upper) {
            let (Surface::Plane { u: a, v: b, .. }, Surface::Plane { u: c, v: d, .. }) =
                (lower.plane(), upper.plane())
            else {
                return Err(unresolved());
            };
            if a != c || b != d {
                return Err(unresolved());
            }
        }
        if self.lower.is_none() && self.upper.is_none() {
            return Err(unresolved());
        }
        let [a, b] = self.source.radii();
        let scale = (2. * a.max(b)).hypot(self.source.height());
        let band = policy.length_at_scale(scale)?;
        let origin = self.source.frame().origin();
        let world = scale
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        let arithmetic = 65536. * f64::EPSILON * world;
        if arithmetic >= policy.linear() / 16. {
            return Err(unresolved());
        }
        for q in 0..4 {
            let Curve::Nurbs(lo) = &self.solid.edges[q].curve else {
                return Err(unresolved());
            };
            let Curve::Nurbs(hi) = &self.solid.edges[4 + q].curve else {
                return Err(unresolved());
            };
            if self.lower.is_some() && self.upper.is_some() && lo.weights() != hi.weights() {
                return Err(unresolved());
            }
            for (a, b) in lo.control_points().iter().zip(hi.control_points()) {
                let gap = self.source.frame().axes()[2].dot(*b - *a);
                if !gap.is_finite() || gap <= 10. * band + arithmetic {
                    return Err(unresolved());
                }
            }
        }
        if !crate::nurbs_frustum::same_body(
            &self.solid,
            &build_between(
                &self.source,
                self.lower.as_ref(),
                self.upper.as_ref(),
                policy,
            )?,
        ) {
            return Err(Error::InvalidTopology(
                "slab differs from its complete retained boundary certificate",
            ));
        }
        let mut uses = [(0usize, 0i32); 12];
        for f in &self.solid.shell.faces {
            for c in &f.wires[0].coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += i32::from(f.orientation) * if c.forward { 1 } else { -1 };
                if let Surface::Plane { .. } = &f.surface {
                    let (PCurve::Nurbs(uv), Curve::Nurbs(curve)) =
                        (&c.pcurve, &self.solid.edges[c.edge].curve)
                    else {
                        return Err(unresolved());
                    };
                    if uv.weights() != curve.weights()
                        || uv.knots() != curve.knots()
                        || uv.degree() != curve.degree()
                    {
                        return Err(unresolved());
                    }
                    for (p, q) in uv.control_points().iter().zip(curve.control_points()) {
                        if norm(f.surface.try_evaluate(p.x, p.y)? - *q) + arithmetic
                            >= policy.linear() / 8.
                        {
                            return Err(unresolved());
                        }
                    }
                }
                for t in [0., 0.25, 0.5, 0.75, 1.] {
                    let uv = c.pcurve.try_evaluate(t)?;
                    if norm(
                        f.surface.try_evaluate(uv[0], uv[1])?
                            - self.solid.edges[c.edge].curve.try_evaluate(t)?,
                    ) + arithmetic
                        >= policy.linear() / 4.
                    {
                        return Err(unresolved());
                    }
                }
            }
        }
        if uses.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "slab is not closed with opposed boundary uses",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let lo = if let Some(s) = &self.lower {
            crate::nurbs_frustum_oblique_split::lower_volume(&self.source, s.plane(), policy)?
        } else {
            0.
        };
        let hi = if let Some(s) = &self.upper {
            crate::nurbs_frustum_oblique_split::lower_volume(&self.source, s.plane(), policy)?
        } else {
            self.source.volume(policy)?
        };
        let v = hi - lo;
        if !v.is_normal() || v <= 32768. * f64::EPSILON * hi.max(lo) {
            return Err(unresolved());
        }
        Ok(v)
    }
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for edge in &self.solid.edges {
            let points = match &edge.curve {
                Curve::Nurbs(c) => c.control_points().to_vec(),
                Curve::Line { a, b } => vec![*a, *b],
                _ => return Err(unresolved()),
            };
            for p in points {
                min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
                max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
            }
        }
        Ok(Bounds { min, max })
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
    pub fn split_by_parallel_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumParallelSplit> {
        self.validate(policy)?;
        let stored = self
            .lower
            .as_ref()
            .or(self.upper.as_ref())
            .ok_or_else(unresolved)?;
        let (Surface::Plane { u: a, v: b, .. }, Surface::Plane { u: c, v: d, .. }) =
            (stored.plane(), plane)
        else {
            return Err(unresolved());
        };
        if a != c || b != d {
            return Err(Error::Unsupported("parallel slab split requires the exact stored plane basis; near-parallel fitting is unsupported"));
        }
        let section = self.source.section_by_plane(plane, policy)?;
        let lower = Self::new(
            &self.source,
            self.lower.clone(),
            Some(section.clone()),
            policy,
        )?;
        let upper = Self::new(&self.source, Some(section), self.upper.clone(), policy)?;
        let original = self.volume(policy)?;
        let sum = lower.volume(policy)? / original + upper.volume(policy)? / original;
        let allowance = 65536. * f64::EPSILON * self.source.volume(policy)? / original;
        if !sum.is_finite() || !allowance.is_finite() || (sum - 1.).abs() > allowance {
            return Err(unresolved());
        }
        Ok(NurbsFrustumParallelSplit { lower, upper })
    }
}
impl NurbsObliqueFrustumSolid {
    pub fn split_by_parallel_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumParallelSplit> {
        self.validate(policy)?;
        let (lower, upper) = if self.is_lower() {
            (None, Some(self.section().clone()))
        } else {
            (Some(self.section().clone()), None)
        };
        NurbsFrustumSlabSolid::new(self.source(), lower, upper, policy)?
            .split_by_parallel_plane(plane, policy)
    }
}
