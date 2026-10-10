//! Checked axial partitions retaining the complete coaxial cylindrical opening.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumPartitions {
    pub parts: Vec<NurbsBoredFrustumSolid>,
}
impl NurbsBoredFrustumSolid {
    /// Partition at 1–16 increasing source-local axial distances. The original
    /// source tolerance band governs every cut; all actual child holes remain.
    pub fn split_axial_many(
        &self,
        cuts: &[f64],
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumPartitions> {
        self.validate(policy)?;
        let source = self.source().split_axial_many(cuts, policy)?;
        let tool = self.tool().split_axial_many(cuts, policy)?;
        let mut parts = Vec::with_capacity(source.parts.len());
        for (outer, inner) in source.parts.iter().zip(&tool.parts) {
            let child = outer.bore_coaxial(self.radius(), policy)?;
            // Both restrictions use the same source frame and axial cuts.
            // Preserve the independently certified cylindrical restriction,
            // rather than substituting a filled outer child.
            // Independent cylindrical interpolation may round a constant
            // radius by one ULP. Compare the entire positive-weight function
            // coefficients within the reserved physical budget, never fit or
            // overwrite either certified body's actual geometry.
            let budget = policy.linear() / 32.;
            let check = |a: Point3, b: Point3| -> Result<()> {
                let d = a - b;
                if !d.finite() || d.x.hypot(d.y).hypot(d.z) >= budget {
                    Err(Error::Unsupported(
                        "bored partition inner restriction correspondence is unresolved",
                    ))
                } else {
                    Ok(())
                }
            };
            for (a, b) in child
                .tool()
                .solid()
                .vertices
                .iter()
                .zip(&inner.solid().vertices)
            {
                check(a.point, b.point)?;
            }
            for (a, b) in child.tool().solid().edges.iter().zip(&inner.solid().edges) {
                match (&a.curve, &b.curve) {
                    (Curve::Nurbs(a), Curve::Nurbs(b))
                        if a.degree() == b.degree()
                            && a.knots() == b.knots()
                            && a.weights() == b.weights() =>
                    {
                        for (a, b) in a.control_points().iter().zip(b.control_points()) {
                            check(*a, *b)?;
                        }
                    }
                    (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => {
                        check(*a, *c)?;
                        check(*b, *d)?;
                    }
                    _ => {
                        return Err(Error::Unsupported(
                            "bored partition inner basis correspondence is unresolved",
                        ))
                    }
                }
            }
            for (a, b) in child.tool().solid().shell.faces[2..]
                .iter()
                .zip(&inner.solid().shell.faces[2..])
            {
                let (Surface::Nurbs(a), Surface::Nurbs(b)) = (&a.surface, &b.surface) else {
                    return Err(Error::Unsupported(
                        "bored partition inner patch basis is unresolved",
                    ));
                };
                if a.degrees() != b.degrees()
                    || a.control_counts() != b.control_counts()
                    || a.weights() != b.weights()
                    || [0, 1]
                        .into_iter()
                        .any(|i| a.knots(i).ok() != b.knots(i).ok())
                {
                    return Err(Error::Unsupported(
                        "bored partition inner patch basis is unresolved",
                    ));
                }
                for (a, b) in a.control_points().iter().zip(b.control_points()) {
                    check(*a, *b)?;
                }
            }
            child.validate(policy)?;
            parts.push(child);
        }
        let original = self.volume(policy)?;
        let mut sum = 0.;
        let mut compensation = 0.;
        for child in &parts {
            let term = child.volume(policy)? / original;
            let adjusted = term - compensation;
            let next = sum + adjusted;
            compensation = (next - sum) - adjusted;
            sum = next;
        }
        let [a, b] = self.source().radii();
        let condition = a.max(b) / (a.min(b) - self.radius());
        let allowance = 32768. * f64::EPSILON * parts.len() as f64 * condition;
        if !sum.is_finite() || !allowance.is_finite() || (sum - 1.).abs() > allowance {
            return Err(Error::Unsupported(
                "positive bored partition volume conservation is numerically unresolved",
            ));
        }
        Ok(NurbsBoredFrustumPartitions { parts })
    }
}
