//! Shape-preserving tensor-product refinement and exact isoparametric curves.
use crate::nurbs::{de_boor, knot_span, project, weighted_controls};
use crate::{Error, KnotSide, NurbsCurve, NurbsSurface, Result};

impl NurbsSurface {
    /// Contract the fixed axis: axis 0 fixes U and returns a curve in V;
    /// axis 1 fixes V and returns a curve in U. Original variable knots remain.
    pub fn isocurve(&self, axis: usize, parameter: f64) -> Result<NurbsCurve> {
        let knots = self.knots(axis)?;
        let degree = self.degrees()[axis];
        let counts = self.control_counts();
        let span = knot_span(degree, knots, counts[axis], parameter, KnotSide::Right)?;
        let h = weighted_controls(self.control_points(), self.weights())?;
        let varying = 1 - axis;
        let mut controls = Vec::with_capacity(counts[varying]);
        let mut weights = Vec::with_capacity(counts[varying]);
        for j in 0..counts[varying] {
            let row = (span - degree..=span)
                .map(|i| h[index(axis, i, j, counts)])
                .collect();
            let value = de_boor(degree, knots, span, parameter, row)?;
            controls.push(project(value)?);
            weights.push(value[3]);
        }
        NurbsCurve::new(
            self.degrees()[varying],
            self.knots(varying)?.to_vec(),
            controls,
            weights,
        )
    }
    /// Insert knots on one tensor axis without changing the mathematical surface.
    /// All rows use one common homogeneous weight scale, preserving relative
    /// row weights. Endpoint insertion and multiplicity above degree are errors.
    pub fn insert_knot(&self, axis: usize, parameter: f64, times: usize) -> Result<Self> {
        let source_knots = self.knots(axis)?;
        let degree = self.degrees()[axis];
        let mut counts = self.control_counts();
        knot_span(
            degree,
            source_knots,
            counts[axis],
            parameter,
            KnotSide::Right,
        )?;
        if times == 0 {
            return Ok(self.clone());
        }
        let [a, b] = self.domain()[axis];
        if parameter == a || parameter == b {
            return Err(Error::Unsupported(
                "clamped surface endpoint knots cannot be inserted",
            ));
        }
        let multiplicity = source_knots.partition_point(|v| *v <= parameter)
            - source_knots.partition_point(|v| *v < parameter);
        if times > degree - multiplicity {
            return Err(Error::InvalidInput(
                "surface knot insertion exceeds degree multiplicity",
            ));
        }
        let final_count = counts[axis]
            .checked_add(times)
            .and_then(|n| n.checked_mul(counts[1 - axis]))
            .ok_or(Error::InvalidInput(
                "surface refinement control count overflows",
            ))?;
        if final_count > 65536 {
            return Err(Error::Unsupported(
                "surface refinement exceeds 65536 controls",
            ));
        }
        if final_count.saturating_mul(times).saturating_mul(degree + 1) > 16_000_000 {
            return Err(Error::Unsupported("surface refinement exceeds work limit"));
        }
        let mut knots = [self.knots(0)?.to_vec(), self.knots(1)?.to_vec()];
        let mut h = weighted_controls(self.control_points(), self.weights())?;
        for inserted in 0..times {
            let span = knot_span(
                degree,
                &knots[axis],
                counts[axis],
                parameter,
                KnotSide::Right,
            )?;
            let s = multiplicity + inserted;
            let mut next_counts = counts;
            next_counts[axis] += 1;
            let mut q = vec![[0.; 4]; next_counts[0] * next_counts[1]];
            for row in 0..counts[1 - axis] {
                for i in 0..=span - degree {
                    q[index(axis, i, row, next_counts)] = h[index(axis, i, row, counts)];
                }
                for i in span - s..counts[axis] {
                    q[index(axis, i + 1, row, next_counts)] = h[index(axis, i, row, counts)];
                }
                for i in span - degree + 1..=span - s {
                    let denominator = knots[axis][i + degree] - knots[axis][i];
                    if denominator <= 0. || !denominator.is_finite() {
                        return Err(Error::InvalidInput(
                            "surface refinement knot interval is unusable",
                        ));
                    }
                    let alpha = (parameter - knots[axis][i]) / denominator;
                    let previous = h[index(axis, i - 1, row, counts)];
                    let current = h[index(axis, i, row, counts)];
                    for j in 0..4 {
                        q[index(axis, i, row, next_counts)][j] =
                            (1. - alpha) * previous[j] + alpha * current[j];
                    }
                }
            }
            knots[axis].insert(span + 1, parameter);
            counts = next_counts;
            h = q;
        }
        let points = h
            .iter()
            .map(|value| project(*value))
            .collect::<Result<Vec<_>>>()?;
        let weights = h.iter().map(|value| value[3]).collect();
        Self::new(self.degrees(), knots, counts, points, weights)
    }
}
fn index(axis: usize, fixed: usize, varying: usize, counts: [usize; 2]) -> usize {
    if axis == 0 {
        fixed * counts[1] + varying
    } else {
        varying * counts[1] + fixed
    }
}
