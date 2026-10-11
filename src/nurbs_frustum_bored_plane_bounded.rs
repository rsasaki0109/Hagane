//! Closed annular bodies bounded by strictly ordered interior planes.
use crate::nurbs_frustum_bored_slab::assemble_solids;
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumPlaneBoundedSolid {
    source: NurbsBoredFrustumSolid,
    outer: NurbsFrustumPlaneBoundedSolid,
    inner: NurbsFrustumPlaneBoundedSolid,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumInteriorPlaneSplit {
    pub lower: NurbsBoredFrustumPlaneBoundedSolid,
    pub upper: NurbsBoredFrustumPlaneBoundedSolid,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "bored plane-bounded geometry, annular cap correspondence or volume is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsBoredFrustumPlaneBoundedSolid {
    pub(crate) fn new(
        source: &NurbsBoredFrustumSolid,
        outer: NurbsFrustumPlaneBoundedSolid,
        inner: NurbsFrustumPlaneBoundedSolid,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let result = Self {
            solid: assemble_solids(outer.solid(), inner.solid())?,
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
    pub fn outer(&self) -> &NurbsFrustumPlaneBoundedSolid {
        &self.outer
    }
    pub fn inner(&self) -> &NurbsFrustumPlaneBoundedSolid {
        &self.inner
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        self.outer.validate(policy)?;
        self.inner.validate(policy)?;
        for (outer, inner) in [
            (self.outer.lower_section(), self.inner.lower_section()),
            (self.outer.upper_section(), self.inner.upper_section()),
        ] {
            match (outer, inner) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    let (
                        Surface::Plane {
                            origin: ao,
                            u: au,
                            v: av,
                        },
                        Surface::Plane {
                            origin: bo,
                            u: bu,
                            v: bv,
                        },
                    ) = (a.plane(), b.plane())
                    else {
                        return Err(unresolved());
                    };
                    if ao != bo || au != bu || av != bv {
                        return Err(Error::InvalidTopology(
                            "annular plane-bounded caps do not share the same caller plane",
                        ));
                    }
                }
                _ => return Err(unresolved()),
            }
        }
        if !crate::nurbs_frustum::same_body(
            self.outer.source().solid(),
            self.source.source().solid(),
        ) || !crate::nurbs_frustum::same_body(
            self.inner.source().solid(),
            self.source.tool().solid(),
        ) || !crate::nurbs_frustum::same_body(
            &self.solid,
            &assemble_solids(self.outer.solid(), self.inner.solid())?,
        ) {
            return Err(Error::InvalidTopology(
                "bored plane-bounded body differs from its complete annular certificate",
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
                "annular plane-bounded boundary is not closed and opposed",
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
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
        let outer = self.outer.split_by_interior_plane(plane, policy)?;
        let inner = self.inner.split_by_interior_plane(plane, policy)?;
        let result = NurbsBoredFrustumInteriorPlaneSplit {
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
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
        let outer = self.outer().split_by_interior_plane(plane, policy)?;
        let inner = self.inner().split_by_interior_plane(plane, policy)?;
        let result = NurbsBoredFrustumInteriorPlaneSplit {
            lower: NurbsBoredFrustumPlaneBoundedSolid::new(
                self.source(),
                outer.lower,
                inner.lower,
                policy,
            )?,
            upper: NurbsBoredFrustumPlaneBoundedSolid::new(
                self.source(),
                outer.upper,
                inner.upper,
                policy,
            )?,
        };
        let original = self.volume(policy)?;
        let sum = result.lower.volume(policy)? / original + result.upper.volume(policy)? / original;
        let allowance = 131072. * f64::EPSILON * self.source().source().volume(policy)? / original;
        if !sum.is_finite() || !allowance.is_finite() || (sum - 1.).abs() > allowance {
            return Err(unresolved());
        }
        Ok(result)
    }
}

impl NurbsBoredFrustumSlabSolid {
    /// Use a new interior plane without changing the exact-basis parallel API.
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
        let outer = NurbsFrustumPlaneBoundedSolid::new(
            self.outer().source(),
            self.outer().lower_section().cloned(),
            self.outer().upper_section().cloned(),
            policy,
        )?;
        let inner = NurbsFrustumPlaneBoundedSolid::new(
            self.inner().source(),
            self.inner().lower_section().cloned(),
            self.inner().upper_section().cloned(),
            policy,
        )?;
        NurbsBoredFrustumPlaneBoundedSolid::new(self.source(), outer, inner, policy)?
            .split_by_interior_plane(plane, policy)
    }
}
