//! Ordered interior plane sections of an original rational frustum.
use crate::nurbs_frustum_slab::build_between;
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsFrustumPlaneBoundedSolid {
    source: NurbsFrustumSolid,
    lower: Option<NurbsFrustumPlaneSection>,
    upper: Option<NurbsFrustumPlaneSection>,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumInteriorPlaneSplit {
    pub lower: NurbsFrustumPlaneBoundedSolid,
    pub upper: NurbsFrustumPlaneBoundedSolid,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "plane-bounded frustum geometry, whole-generator clearance or arithmetic is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsFrustumPlaneBoundedSolid {
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
    pub(crate) fn from_retained(
        source: &NurbsFrustumSolid,
        lower: Option<NurbsFrustumPlaneSection>,
        upper: Option<NurbsFrustumPlaneSection>,
        solid: Solid,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let result = Self {
            source: source.clone(),
            lower,
            upper,
            solid,
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
            // Check association to THIS original source, rather than only the
            // private source stored inside the section's own certificate.
            let expected = self.source.section_by_plane(section.plane(), policy)?;
            for (actual, expected) in section.edges().iter().zip(expected.edges()) {
                if actual.vertices != expected.vertices
                    || !crate::nurbs_graph_step_import::same_curve(&actual.curve, &expected.curve)
                {
                    return Err(Error::InvalidTopology(
                        "plane-bounded section does not belong to its original source",
                    ));
                }
            }
            for (actual, expected) in section.uses().iter().zip(expected.uses()) {
                let (PCurve::Nurbs(a), PCurve::Nurbs(b)) = (&actual.pcurve, &expected.pcurve)
                else {
                    return Err(unresolved());
                };
                if actual.face_id != expected.face_id
                    || a.degree() != b.degree()
                    || a.knots() != b.knots()
                    || a.weights() != b.weights()
                    || a.control_points() != b.control_points()
                {
                    return Err(Error::InvalidTopology(
                        "plane-bounded section source parameters changed",
                    ));
                }
            }
        }
        if self.lower.is_none() && self.upper.is_none() {
            return Err(unresolved());
        }
        let [a, b] = self.source.radii();
        let scale = (2. * a.max(b)).hypot(self.source.height());
        let origin = self.source.frame().origin();
        let world = scale
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        let arithmetic = 65536. * f64::EPSILON * world;
        if arithmetic >= policy.linear() / 16. {
            return Err(unresolved());
        }
        certify_ordering(
            &self.source,
            self.lower.as_ref(),
            self.upper.as_ref(),
            policy,
        )?;
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
                "plane-bounded body differs from its complete retained boundary certificate",
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
                "plane-bounded body is not closed with opposed boundary uses",
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
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
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
        Ok(NurbsFrustumInteriorPlaneSplit { lower, upper })
    }
}
impl NurbsObliqueFrustumSolid {
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
        let (lower, upper) = if self.is_lower() {
            (None, Some(self.section().clone()))
        } else {
            (Some(self.section().clone()), None)
        };
        NurbsFrustumPlaneBoundedSolid::new(self.source(), lower, upper, policy)?
            .split_by_interior_plane(plane, policy)
    }
}

impl NurbsFrustumSlabSolid {
    /// Split by a separately certified interior plane; the parallel API remains strict.
    pub fn split_by_interior_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumInteriorPlaneSplit> {
        self.validate(policy)?;
        NurbsFrustumPlaneBoundedSolid::new(
            self.source(),
            self.lower_section().cloned(),
            self.upper_section().cloned(),
            policy,
        )?
        .split_by_interior_plane(plane, policy)
    }
}

/// The plane height over the source's generator disk is
/// z(q)=(c-r0*v.q)/(1+s*v.q). Positive denominators and axis radius
/// R=r0+s*c are required for the whole disk, not merely at sample points.
fn certify_plane_disk(
    source: &NurbsFrustumSolid,
    section: &NurbsFrustumPlaneSection,
    arithmetic: f64,
) -> Result<()> {
    let Surface::Plane { origin, u, v } = *section.plane() else {
        return Err(unresolved());
    };
    let normal = u.cross(v).normalized()?;
    let n = source.frame().local_vector(normal);
    let delta = origin - source.frame().origin();
    if !delta.finite() || !n.finite() || !n.z.is_normal() {
        return Err(unresolved());
    }
    let c = normal.dot(delta) / n.z;
    let slope = (source.radii()[1] - source.radii()[0]) / source.height();
    let l = (n.x / n.z).hypot(n.y / n.z);
    let denominator = 1. - slope.abs() * l;
    let rounding = 32768. * f64::EPSILON * (1. + slope.abs() * l) * (1. + 1. / n.z.abs());
    let radius = source.radii()[0] + slope * c;
    let radius_error = arithmetic * (1. + slope.abs()) / n.z.abs()
        + 32768.
            * f64::EPSILON
            * (source.radii()[0].abs() + (slope * c).abs())
            * (1. + 1. / n.z.abs());
    if !c.is_finite()
        || !slope.is_finite()
        || !l.is_finite()
        || !denominator.is_normal()
        || denominator <= rounding
        || !rounding.is_finite()
        || !radius.is_normal()
        || !radius_error.is_finite()
        || radius <= radius_error
    {
        return Err(unresolved());
    }
    Ok(())
}
fn certify_ordering(
    source: &NurbsFrustumSolid,
    lower: Option<&NurbsFrustumPlaneSection>,
    upper: Option<&NurbsFrustumPlaneSection>,
    policy: GeometryTolerance,
) -> Result<()> {
    let [r0, r1] = source.radii();
    let scale = r0.max(r1).max(source.height());
    let extent = (2. * r0.max(r1)).hypot(source.height());
    let band = policy.length_at_scale(extent)?;
    let origin = source.frame().origin();
    let mut world = extent
        .max(origin.x.abs())
        .max(origin.y.abs())
        .max(origin.z.abs());
    for section in [lower, upper].into_iter().flatten() {
        let Surface::Plane { origin, .. } = *section.plane() else {
            return Err(unresolved());
        };
        world = world
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        for edge in section.edges() {
            let Curve::Nurbs(curve) = &edge.curve else {
                return Err(unresolved());
            };
            for p in curve.control_points() {
                world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
            }
        }
    }
    let axes = source.frame().axes();
    let mut gram = 0f64;
    for (i, a) in axes.iter().enumerate() {
        for (j, b) in axes.iter().enumerate() {
            gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
        }
    }
    let arithmetic = 65536. * f64::EPSILON * world + 16. * gram * extent;
    if !scale.is_normal() || !arithmetic.is_finite() || arithmetic >= policy.linear() / 32. {
        return Err(unresolved());
    }
    for section in [lower, upper].into_iter().flatten() {
        certify_plane_disk(source, section, arithmetic)?;
    }
    let margin = 10. * band / scale;
    if !margin.is_finite() {
        return Err(unresolved());
    }
    for q in 0..4 {
        let lo = if let Some(s) = lower {
            &s.edges()[q].curve
        } else {
            &source.solid().edges[q].curve
        };
        let hi = if let Some(s) = upper {
            &s.edges()[q].curve
        } else {
            &source.solid().edges[4 + q].curve
        };
        let (Curve::Nurbs(lo), Curve::Nurbs(hi)) = (lo, hi) else {
            return Err(unresolved());
        };
        if lo.degree() != 2
            || hi.degree() != 2
            || lo.control_points().len() != 3
            || hi.control_points().len() != 3
        {
            return Err(unresolved());
        }
        let normalize = |curve: &NurbsCurve| -> Result<([f64; 3], [f64; 3])> {
            let max = curve.weights().iter().copied().fold(0f64, f64::max);
            if !max.is_normal() {
                return Err(unresolved());
            }
            let weights = std::array::from_fn(|i| curve.weights()[i] / max);
            let heights =
                std::array::from_fn(|i| axes[2].dot(curve.control_points()[i] - origin) / scale);
            if weights.iter().any(|w| !w.is_normal() || *w <= 0.)
                || heights.iter().any(|z| !z.is_finite())
            {
                return Err(unresolved());
            }
            Ok((weights, heights))
        };
        // Scaling weights here is proof-only. Actual curves/nets retain every
        // original raw coefficient and weight; nothing is fitted or normalized.
        let (wl, zl) = normalize(lo)?;
        let (wu, zu) = normalize(hi)?;
        let minimum = wl.into_iter().fold(f64::INFINITY, f64::min)
            * wu.into_iter().fold(f64::INFINITY, f64::min);
        if !minimum.is_normal() {
            return Err(unresolved());
        }
        let c2 = [1., 2., 1.];
        let c4 = [1., 4., 6., 4., 1.];
        for (k, &divisor) in c4.iter().enumerate() {
            let mut numerator = 0.;
            let mut denominator = 0.;
            let mut operands = 0.;
            for i in 0..3 {
                if k < i || k - i > 2 {
                    continue;
                }
                let j = k - i;
                let factor = c2[i] * c2[j] / divisor;
                let weight = factor * wu[i] * wl[j];
                numerator += weight * (zu[i] - zl[j] - margin);
                denominator += weight;
                operands += weight * (zu[i].abs() + zl[j].abs() + margin.abs());
            }
            // Degree-four Bernstein product: (Zu*Wl-Zl*Wu)-margin*Wu*Wl.
            // Termwise absolute operands cover signed subtraction and proof-only
            // normalization; the physical projection reserve enters both heights.
            let error = 2. * arithmetic / scale * denominator + 32768. * f64::EPSILON * operands;
            let physical_error = error / minimum * scale;
            if !numerator.is_finite()
                || !error.is_finite()
                || !physical_error.is_finite()
                || numerator <= error
                || physical_error >= policy.linear() / 8.
            {
                return Err(unresolved());
            }
        }
    }
    // Boundary clearance extends to the FULL generator disk, not just its rim.
    // For each plane, grad z=-R*v/(1+s*v.q)^2 with R>0 and denominator>0.
    // If the two v vectors are nonparallel, grad(z_upper-z_lower) cannot vanish:
    // its minimum on the compact disk lies on the circle. If parallel (including
    // an axial cap), the difference depends on one scalar whose entire disk range
    // is attained on the circle. Thus the strict Bernstein rim margin applies
    // throughout the disk and hence also any coaxial annulus. No angle fitting or
    // numerical parallel/nonparallel branch is needed for this mathematical fact.
    Ok(())
}
