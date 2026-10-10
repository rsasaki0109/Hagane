//! Axial partition of the completely certified canonical NURBS frustum family.
use crate::*;

#[derive(Clone, Debug)]
pub struct NurbsFrustumSplit {
    pub lower: NurbsFrustumSolid,
    pub upper: NurbsFrustumSolid,
    /// Four actual rational boundary curves of the lower child's top cap.
    pub section: Vec<Curve>,
    /// Source-local physical axial distance from the bottom cap.
    pub cut_height: f64,
    pub radius: f64,
}

/// Ordered, closed parts of one source, with actual section curves between them.
#[derive(Clone, Debug)]
pub struct NurbsFrustumPartitions {
    pub parts: Vec<NurbsFrustumSolid>,
    pub sections: Vec<Vec<Curve>>,
    /// Strictly increasing source-local axial distances.
    pub cut_heights: Vec<f64>,
}
fn unresolved() -> Error {
    Error::Unsupported("frustum axial partition geometry or arithmetic is unresolved")
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn check(value: f64, budget: f64) -> Result<()> {
    if value.is_finite() && value < budget {
        Ok(())
    } else {
        Err(unresolved())
    }
}
fn same_basis(a: &NurbsSurface, b: &NurbsSurface) -> bool {
    a.degrees() == [2, 1]
        && b.degrees() == [2, 1]
        && a.control_counts() == [3, 2]
        && b.control_counts() == [3, 2]
        && a.weights() == b.weights()
        && [0, 1]
            .into_iter()
            .all(|axis| a.knots(axis).ok() == b.knots(axis).ok())
}
fn check_restriction(source: &Solid, child: &Solid, range: [f64; 2], budget: f64) -> Result<()> {
    for face in 2..6 {
        let (Surface::Nurbs(a), Surface::Nurbs(b)) = (
            &source.shell.faces[face].surface,
            &child.shell.faces[face].surface,
        ) else {
            return Err(unresolved());
        };
        if !same_basis(a, b) {
            return Err(unresolved());
        }
        for row in 0..3 {
            let index = 2 * row;
            // Equal endpoint weights in V make homogeneous restriction exactly
            // the same affine combination of Euclidean controls. U weights are
            // positive and identical, so the maximum control discrepancy bounds
            // the entire rational patch, not merely a grid of evaluations.
            if a.weights()[index] != a.weights()[index + 1] {
                return Err(unresolved());
            }
            for (column, t) in range.into_iter().enumerate() {
                let expected =
                    a.control_points()[index] * (1. - t) + a.control_points()[index + 1] * t;
                check(
                    length(expected - b.control_points()[index + column]),
                    budget,
                )?;
            }
        }
    }
    Ok(())
}
fn check_section(lower: &Solid, upper: &Solid, budget: f64, scale: f64) -> Result<Vec<Curve>> {
    let mut curves = Vec::with_capacity(4);
    for i in 0..4 {
        let (Curve::Nurbs(a), Curve::Nurbs(b)) = (&lower.edges[4 + i].curve, &upper.edges[i].curve)
        else {
            return Err(unresolved());
        };
        if a.degree() != b.degree()
            || a.knots() != b.knots()
            || a.weights() != b.weights()
            || a.control_points().len() != b.control_points().len()
        {
            return Err(unresolved());
        }
        for (a, b) in a.control_points().iter().zip(b.control_points()) {
            check(length(*a - *b), budget)?;
        }
        curves.push(lower.edges[4 + i].curve.clone());
    }
    let lower_cap = &lower.shell.faces[1];
    let upper_cap = &upper.shell.faces[0];
    let n0 = lower_cap.surface.normal(0.) * f64::from(lower_cap.orientation);
    let n1 = upper_cap.surface.normal(0.) * f64::from(upper_cap.orientation);
    check(length(n0 + n1) * scale, budget)?;
    Ok(curves)
}
fn centroid_fraction(radii: [f64; 2]) -> f64 {
    let scale = radii[0].max(radii[1]);
    let [a, b] = radii.map(|r| r / scale);
    (a * a + 2. * a * b + 3. * b * b) / (4. * (a * a + a * b + b * b))
}
impl NurbsFrustumSolid {
    /// Partition at 1–16 strictly increasing source-local axial heights.
    /// Every interval must exceed ten times the FULL source tolerance band.
    /// Returns no partial result on failure and never modifies the source.
    pub fn split_axial_many(
        &self,
        cuts: &[f64],
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumPartitions> {
        self.validate(policy)?;
        if cuts.is_empty() || cuts.len() > 16 {
            return Err(Error::InvalidInput("frustum partition requires 1–16 cuts"));
        }
        let height = self.height();
        let frame = self.frame();
        let scale = (2. * self.radii()[0].max(self.radii()[1])).hypot(height);
        let world = scale
            .max(frame.origin().x.abs())
            .max(frame.origin().y.abs())
            .max(frame.origin().z.abs());
        let arithmetic = 32768. * f64::EPSILON * world;
        let minimum = 10. * policy.length_at_scale(scale)? + arithmetic;
        let budget = policy.linear() / 32. - arithmetic;
        if budget <= 0. || !budget.is_finite() {
            return Err(unresolved());
        }
        let mut previous = 0.;
        for &cut in cuts {
            if !cut.is_finite() || cut <= previous || cut >= height {
                return Err(Error::InvalidInput(
                    "cuts must be finite, increasing and strictly interior",
                ));
            }
            if cut - previous <= minimum {
                return Err(Error::Unsupported(
                    "frustum partition interval is unresolved at source tolerance",
                ));
            }
            previous = cut;
        }
        if height - previous <= minimum {
            return Err(Error::Unsupported(
                "frustum final partition interval is unresolved at source tolerance",
            ));
        }
        let mut remaining = self.clone();
        let mut parts = Vec::with_capacity(cuts.len() + 1);
        previous = 0.;
        for &cut in cuts {
            let split = remaining.split_axial(cut - previous, policy)?;
            parts.push(split.lower);
            remaining = split.upper;
            previous = cut;
        }
        parts.push(remaining);
        // Repeated frame arithmetic must still certify EVERY returned patch
        // against the original source, rather than accumulate unchecked drift.
        let mut start = 0.;
        for (i, part) in parts.iter().enumerate() {
            let end = cuts.get(i).copied().unwrap_or(height);
            check_restriction(
                self.solid(),
                part.solid(),
                [start / height, end / height],
                budget,
            )?;
            start = end;
        }
        let sections = parts
            .windows(2)
            .map(|pair| check_section(pair[0].solid(), pair[1].solid(), budget, scale))
            .collect::<Result<Vec<_>>>()?;
        let source_volume = self.volume(policy)?;
        let mut fraction = 0.;
        let mut moment = 0.;
        start = 0.;
        for (i, part) in parts.iter().enumerate() {
            let end = cuts.get(i).copied().unwrap_or(height);
            let weight = part.volume(policy)? / source_volume;
            fraction += weight;
            moment += weight
                * (start / height + (end - start) / height * centroid_fraction(part.radii()));
            start = end;
        }
        check((fraction - 1.).abs(), 32768. * f64::EPSILON)?;
        check(
            (moment - centroid_fraction(self.radii())).abs(),
            32768. * f64::EPSILON,
        )?;
        Ok(NurbsFrustumPartitions {
            parts,
            sections,
            cut_heights: cuts.to_vec(),
        })
    }
    /// Split by the source-local plane Z=`cut_height`, returning both closed
    /// canonical rational bodies and their actual circular section curves.
    /// The source remains unchanged. Oblique cuts and general NURBS solids are
    /// outside this operation. Entity IDs and bit-identical section coordinates
    /// across independently evaluated world frames are not promised. Analytic
    /// volume and centroid metrics must be representable for conservation admission.
    pub fn split_axial(
        &self,
        cut_height: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumSplit> {
        self.validate(policy)?;
        if !cut_height.is_finite() {
            return Err(Error::InvalidInput("frustum cut height must be finite"));
        }
        let height = self.height();
        let radii = self.radii();
        let frame = self.frame();
        let scale = (2. * radii[0].max(radii[1])).hypot(height);
        let world = scale
            .max(frame.origin().x.abs())
            .max(frame.origin().y.abs())
            .max(frame.origin().z.abs());
        let arithmetic = 32768. * f64::EPSILON * world;
        let band = policy.length_at_scale(scale)?;
        let upper_height = height - cut_height;
        if cut_height <= 10. * band + arithmetic || upper_height <= 10. * band + arithmetic {
            return Err(Error::Unsupported(
                "frustum axial cut must leave two resolved strictly interior heights",
            ));
        }
        let budget = policy.linear() / 32. - arithmetic;
        if !budget.is_finite() || budget <= 0. {
            return Err(unresolved());
        }
        let t = cut_height / height;
        if !t.is_finite() || !(0. ..1.).contains(&t) {
            return Err(unresolved());
        }
        let radius = radii[0] * (1. - t) + radii[1] * t;
        let lower = Self::new(frame, [radii[0], radius], cut_height, policy)?;
        let upper_frame = Frame3::new_with_tolerance(
            frame.point(Vec3::new(0., 0., cut_height)),
            frame.axes(),
            policy,
        )?;
        let upper = Self::new(upper_frame, [radius, radii[1]], upper_height, policy)?;
        lower.validate(policy)?;
        upper.validate(policy)?;
        check_restriction(self.solid(), lower.solid(), [0., t], budget)?;
        check_restriction(self.solid(), upper.solid(), [t, 1.], budget)?;
        let section = check_section(lower.solid(), upper.solid(), budget, scale)?;
        // Require representable analytic metrics for conservation admission.
        // Normalize moments locally, avoiding large-world centroid subtraction
        // and overflowing volume-times-length products. Metric errors propagate.
        let source = self.mass_properties(policy)?;
        let lo = lower.mass_properties(policy)?;
        let hi = upper.mass_properties(policy)?;
        let a = lo.volume / source.volume;
        let b = hi.volume / source.volume;
        check((a + b - 1.).abs(), 32768. * f64::EPSILON)?;
        let moment = a * (cut_height / height) * centroid_fraction(lower.radii())
            + b * (cut_height / height
                + (upper_height / height) * centroid_fraction(upper.radii()));
        check(
            (moment - centroid_fraction(radii)).abs(),
            32768. * f64::EPSILON,
        )?;
        Ok(NurbsFrustumSplit {
            lower,
            upper,
            section,
            cut_height,
            radius,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axial_partition_repeated_and_swapped_radii_preserves_analytic_mass() {
        let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        for radii in [[2., 4.], [4., 2.], [3., 3.]] {
            let body = NurbsFrustumSolid::new(Frame3::IDENTITY, radii, 10., policy).unwrap();
            let before = format!("{:?}", body.solid());
            let split = body.split_axial(3., policy).unwrap();
            assert_eq!(split.section.len(), 4);
            assert!((split.radius - (0.7 * radii[0] + 0.3 * radii[1])).abs() < 1e-14);
            assert!(
                (split.lower.volume(policy).unwrap() + split.upper.volume(policy).unwrap()
                    - body.volume(policy).unwrap())
                .abs()
                    < 1e-10
            );
            let next = split.upper.split_axial(2., policy).unwrap();
            assert!(
                (next.lower.volume(policy).unwrap() + next.upper.volume(policy).unwrap()
                    - split.upper.volume(policy).unwrap())
                .abs()
                    < 1e-10
            );
            assert_eq!(before, format!("{:?}", body.solid()));
        }
    }
    #[test]
    fn placed_and_scaled_restrictions_match_source_dense_points() {
        let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.7).unwrap();
        for scale in [1e-60, 1., 1e60] {
            let policy = GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap();
            let frame = Frame3::new(
                Point3::new(7. * scale, -3. * scale, 9. * scale),
                rotation.axes(),
                policy.absolute(),
            )
            .unwrap();
            let body = NurbsFrustumSolid::new(frame, [2. * scale, 4. * scale], 10. * scale, policy)
                .unwrap();
            let split = body.split_axial(3. * scale, policy).unwrap();
            for face in 2..6 {
                for i in 0..=8 {
                    for j in 0..=8 {
                        let u = i as f64 / 8.;
                        let v = j as f64 / 8.;
                        for (child, start, width) in
                            [(&split.lower, 0., 0.3), (&split.upper, 0.3, 0.7)]
                        {
                            let a = body.solid().shell.faces[face]
                                .surface
                                .try_evaluate(u, start + width * v)
                                .unwrap();
                            let b = child.solid().shell.faces[face]
                                .surface
                                .try_evaluate(u, v)
                                .unwrap();
                            assert!(length(a - b) < 1e-12 * scale);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn contact_endpoint_nonfinite_and_full_source_relative_band_reject() {
        let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [2., 4.], 10., policy).unwrap();
        for cut in [0., 10., -1., 11., f64::NAN, f64::INFINITY, 5e-6] {
            assert!(body.split_axial(cut, policy).is_err());
        }
        let relative = GeometryTolerance::new(1e-6, 1e-10, 1e-3).unwrap();
        assert!(body.split_axial(0.01, relative).is_err());
        assert!(body.split_axial(3., relative).is_ok());
    }
}
