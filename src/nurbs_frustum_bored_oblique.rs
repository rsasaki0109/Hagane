//! Exact annular oblique pieces composed from independently certified cuts.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredObliqueFrustumSolid {
    source: NurbsBoredFrustumSolid,
    outer: NurbsObliqueFrustumSolid,
    inner: NurbsObliqueFrustumSolid,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumPlaneSplit {
    pub lower: NurbsBoredObliqueFrustumSolid,
    pub upper: NurbsBoredObliqueFrustumSolid,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "bored oblique split geometry, cap correspondence or positive volume is unresolved",
    )
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
fn assemble(outer: &NurbsObliqueFrustumSolid, inner: &NurbsObliqueFrustumSolid) -> Result<Solid> {
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
fn retained_volume(
    outer: &NurbsObliqueFrustumSolid,
    inner: &NurbsObliqueFrustumSolid,
    policy: GeometryTolerance,
) -> Result<f64> {
    let a = outer.volume(policy)?;
    let b = inner.volume(policy)?;
    let v = a - b;
    if !v.is_normal() || v <= 32768. * f64::EPSILON * a.max(b) {
        return Err(unresolved());
    }
    Ok(v)
}
impl NurbsBoredFrustumSolid {
    pub fn split_by_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumPlaneSplit> {
        self.validate(policy)?;
        let outer = self.source().split_by_plane(plane, policy)?;
        let inner = self.tool().split_by_plane(plane, policy)?;
        let make = |outer: NurbsObliqueFrustumSolid,
                    inner: NurbsObliqueFrustumSolid|
         -> Result<NurbsBoredObliqueFrustumSolid> {
            let result = NurbsBoredObliqueFrustumSolid {
                solid: assemble(&outer, &inner)?,
                source: self.clone(),
                outer,
                inner,
            };
            result.validate(policy)?;
            retained_volume(&result.outer, &result.inner, policy)?;
            Ok(result)
        };
        let result = NurbsBoredFrustumPlaneSplit {
            lower: make(outer.lower, inner.lower)?,
            upper: make(outer.upper, inner.upper)?,
        };
        let original = self.volume(policy)?;
        let sum = result.lower.volume(policy)? / original + result.upper.volume(policy)? / original;
        let allowance = 65536. * f64::EPSILON * self.source().volume(policy)? / original;
        if !sum.is_finite() || !allowance.is_finite() || (sum - 1.).abs() > allowance {
            return Err(unresolved());
        }
        Ok(result)
    }
}
impl NurbsBoredObliqueFrustumSolid {
    pub fn source(&self) -> &NurbsBoredFrustumSolid {
        &self.source
    }
    pub fn outer(&self) -> &NurbsObliqueFrustumSolid {
        &self.outer
    }
    pub fn inner(&self) -> &NurbsObliqueFrustumSolid {
        &self.inner
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn is_lower(&self) -> bool {
        self.outer.is_lower()
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        self.outer.validate(policy)?;
        self.inner.validate(policy)?;
        let same_plane = match (self.outer.section().plane(), self.inner.section().plane()) {
            (
                Surface::Plane {
                    origin: a,
                    u: b,
                    v: c,
                },
                Surface::Plane {
                    origin: d,
                    u: e,
                    v: f,
                },
            ) => a == d && b == e && c == f,
            _ => false,
        };
        if !same_plane
            || self.outer.is_lower() != self.inner.is_lower()
            || !crate::nurbs_frustum::same_body(
                self.outer.source().solid(),
                self.source.source().solid(),
            )
            || !crate::nurbs_frustum::same_body(
                self.inner.source().solid(),
                self.source.tool().solid(),
            )
            || !crate::nurbs_frustum::same_body(&self.solid, &assemble(&self.outer, &self.inner)?)
        {
            return Err(Error::InvalidTopology(
                "bored oblique body differs from its complete annular construction certificate",
            ));
        }
        let mut counts = [(0usize, 0i32); 24];
        for face in &self.solid.shell.faces {
            for wire in &face.wires {
                for c in &wire.coedges {
                    counts[c.edge].0 += 1;
                    counts[c.edge].1 +=
                        i32::from(face.orientation) * if c.forward { 1 } else { -1 };
                    if matches!(face.surface, Surface::Plane { .. }) {
                        let (PCurve::Nurbs(uv), Curve::Nurbs(curve)) =
                            (&c.pcurve, &self.solid.edges[c.edge].curve)
                        else {
                            return Err(unresolved());
                        };
                        if uv.degree() != curve.degree()
                            || uv.weights() != curve.weights()
                            || uv.knots() != curve.knots()
                        {
                            return Err(unresolved());
                        }
                        for (p, q) in uv.control_points().iter().zip(curve.control_points()) {
                            let d = face.surface.try_evaluate(p.x, p.y)? - *q;
                            if !d.finite() || norm(d) >= policy.linear() / 16. {
                                return Err(unresolved());
                            }
                        }
                    }
                }
            }
        }
        if counts.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "bored oblique annular boundary is not closed with opposed uses",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        retained_volume(&self.outer, &self.inner, policy)
    }
    /// Conservative outer positive-weight control-hull enclosure.
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        self.outer.bounds(policy)
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
}
