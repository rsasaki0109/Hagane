//! Closed annular slabs bounded by retained exact parallel sections.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumSlabSolid {
    source: NurbsBoredFrustumSolid,
    outer: NurbsFrustumSlabSolid,
    inner: NurbsFrustumSlabSolid,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumParallelSplit {
    pub lower: NurbsBoredFrustumSlabSolid,
    pub upper: NurbsBoredFrustumSlabSolid,
}
fn unresolved() -> Error {
    Error::Unsupported("bored parallel slab geometry, cap correspondence or volume is unresolved")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn cap_curve(surface: &Surface, curve: &Curve) -> Result<PCurve> {
    let (Surface::Plane { origin, u, v }, Curve::Nurbs(curve)) = (surface, curve) else {
        return Err(unresolved());
    };
    let aa = u.dot(*u);
    let ab = u.dot(*v);
    let bb = v.dot(*v);
    let determinant = aa * bb - ab * ab;
    if !determinant.is_normal() || determinant <= 0. {
        return Err(unresolved());
    }
    let points = curve
        .control_points()
        .iter()
        .map(|p| {
            let d = *p - *origin;
            Point3::new(
                (bb * u.dot(d) - ab * v.dot(d)) / determinant,
                (aa * v.dot(d) - ab * u.dot(d)) / determinant,
                0.,
            )
        })
        .collect();
    PCurve::nurbs(NurbsCurve::new(
        curve.degree(),
        curve.knots().to_vec(),
        points,
        curve.weights().to_vec(),
    )?)
}
fn assemble(outer: &NurbsFrustumSlabSolid, inner: &NurbsFrustumSlabSolid) -> Result<Solid> {
    let mut body = outer.solid().clone();
    body.vertices.extend(inner.solid().vertices.iter().cloned());
    body.edges
        .extend(inner.solid().edges.iter().cloned().map(|mut e| {
            e.vertices = e.vertices.map(|v| v + 8);
            e
        }));
    for i in 0..2 {
        let mut wire = inner.solid().shell.faces[i].wires[0].clone();
        wire.coedges.reverse();
        for c in &mut wire.coedges {
            c.pcurve = cap_curve(
                &body.shell.faces[i].surface,
                &inner.solid().edges[c.edge].curve,
            )?;
            c.edge += 12;
            c.forward = !c.forward;
        }
        body.shell.faces[i].wires.push(wire);
    }
    body.shell
        .faces
        .extend(inner.solid().shell.faces[2..].iter().cloned().map(|mut f| {
            f.orientation = -f.orientation;
            for w in &mut f.wires {
                for c in &mut w.coedges {
                    c.edge += 12;
                }
            }
            f
        }));
    Ok(body)
}
impl NurbsBoredFrustumSlabSolid {
    fn new(
        source: &NurbsBoredFrustumSolid,
        outer: NurbsFrustumSlabSolid,
        inner: NurbsFrustumSlabSolid,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let result = Self {
            solid: assemble(&outer, &inner)?,
            source: source.clone(),
            outer,
            inner,
        };
        result.validate(policy)?;
        result.volume(policy)?;
        Ok(result)
    }
    pub fn source(&self) -> &NurbsBoredFrustumSolid {
        &self.source
    }
    pub fn outer(&self) -> &NurbsFrustumSlabSolid {
        &self.outer
    }
    pub fn inner(&self) -> &NurbsFrustumSlabSolid {
        &self.inner
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        self.outer.validate(policy)?;
        self.inner.validate(policy)?;
        if !crate::nurbs_frustum::same_body(
            self.outer.source().solid(),
            self.source.source().solid(),
        ) || !crate::nurbs_frustum::same_body(
            self.inner.source().solid(),
            self.source.tool().solid(),
        ) || !crate::nurbs_frustum::same_body(&self.solid, &assemble(&self.outer, &self.inner)?)
        {
            return Err(Error::InvalidTopology(
                "bored slab differs from its complete annular certificate",
            ));
        }
        let mut uses = [(0usize, 0i32); 24];
        for f in &self.solid.shell.faces {
            for w in &f.wires {
                for c in &w.coedges {
                    uses[c.edge].0 += 1;
                    uses[c.edge].1 += i32::from(f.orientation) * if c.forward { 1 } else { -1 };
                    if matches!(f.surface, Surface::Plane { .. }) {
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
                            if norm(f.surface.try_evaluate(p.x, p.y)? - *q) >= policy.linear() / 16.
                            {
                                return Err(unresolved());
                            }
                        }
                    }
                }
            }
        }
        if uses.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "annular slab boundary is not closed and opposed",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let a = self.outer.volume(policy)?;
        let b = self.inner.volume(policy)?;
        let v = a - b;
        if !v.is_normal() || v <= 32768. * f64::EPSILON * a.max(b) {
            return Err(unresolved());
        }
        Ok(v)
    }
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        self.outer.bounds(policy)
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
    pub fn split_by_parallel_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumParallelSplit> {
        self.validate(policy)?;
        let outer = self.outer.split_by_parallel_plane(plane, policy)?;
        let inner = self.inner.split_by_parallel_plane(plane, policy)?;
        let result = NurbsBoredFrustumParallelSplit {
            lower: Self::new(&self.source, outer.lower, inner.lower, policy)?,
            upper: Self::new(&self.source, outer.upper, inner.upper, policy)?,
        };
        let original = self.volume(policy)?;
        let sum = result.lower.volume(policy)? / original + result.upper.volume(policy)? / original;
        let allowance = 131072. * f64::EPSILON * self.source.source().volume(policy)? / original;
        if !sum.is_finite() || !allowance.is_finite() || (sum - 1.).abs() > allowance {
            return Err(unresolved());
        }
        Ok(result)
    }
}
impl NurbsBoredObliqueFrustumSolid {
    pub fn split_by_parallel_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumParallelSplit> {
        self.validate(policy)?;
        let outer = self.outer().split_by_parallel_plane(plane, policy)?;
        let inner = self.inner().split_by_parallel_plane(plane, policy)?;
        Ok(NurbsBoredFrustumParallelSplit {
            lower: NurbsBoredFrustumSlabSolid::new(
                self.source(),
                outer.lower,
                inner.lower,
                policy,
            )?,
            upper: NurbsBoredFrustumSlabSolid::new(
                self.source(),
                outer.upper,
                inner.upper,
                policy,
            )?,
        })
    }
}
