//! Exact rectangular restriction of positive rational tensor-product surfaces.
use crate::{Error, NurbsSurface, Result};
impl NurbsSurface {
    /// Restrict the original U/V domain by exact knot insertion and control
    /// slicing. No parameter rescaling or geometric approximation is performed.
    pub fn restricted(&self, ranges: [[f64; 2]; 2]) -> Result<Self> {
        let domain = self.domain();
        let degrees = self.degrees();
        let mut insertions: [Vec<(f64, usize)>; 2] = [Vec::new(), Vec::new()];
        let mut counts = self.control_counts();
        for axis in 0..2 {
            let [a, b] = ranges[axis];
            if !a.is_finite()
                || !b.is_finite()
                || a >= b
                || !(b - a).is_finite()
                || a < domain[axis][0]
                || b > domain[axis][1]
            {
                return Err(Error::InvalidInput(
                    "surface restriction requires ordered finite ranges inside original domain",
                ));
            }
            let knots = self.knots(axis)?;
            for parameter in [a, b] {
                if parameter == domain[axis][0] || parameter == domain[axis][1] {
                    continue;
                }
                let multiplicity = knots.partition_point(|v| *v <= parameter)
                    - knots.partition_point(|v| *v < parameter);
                let times = degrees[axis] - multiplicity;
                if times > 0 {
                    insertions[axis].push((parameter, times));
                    counts[axis] = counts[axis].saturating_add(times);
                }
            }
        }
        if counts[0].saturating_mul(counts[1]) > 65536 {
            return Err(Error::Unsupported(
                "surface restriction exceeds refinement control limit",
            ));
        }
        let mut current = self.control_counts();
        let mut work = 0usize;
        for axis in 0..2 {
            for (_, times) in &insertions[axis] {
                current[axis] += times;
                work = work.saturating_add(
                    current[0]
                        .saturating_mul(current[1])
                        .saturating_mul(*times)
                        .saturating_mul(degrees[axis] + 1),
                );
            }
        }
        if work > 16_000_000 {
            return Err(Error::Unsupported(
                "surface restriction exceeds refinement work limit",
            ));
        }
        let mut refined = self.clone();
        for (axis, insertions) in insertions.into_iter().enumerate() {
            for (parameter, times) in insertions {
                refined = refined.insert_knot(axis, parameter, times)?;
            }
        }
        let mut starts = [0usize; 2];
        let mut selected_counts = [0usize; 2];
        let mut output_knots: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
        for axis in 0..2 {
            let [a, b] = ranges[axis];
            let degree = degrees[axis];
            let knots = refined.knots(axis)?;
            // The right span at the lower endpoint supplies the first retained
            // control. The left span at the upper endpoint supplies the last.
            let right = knots.partition_point(|v| *v <= a) - 1;
            let left = knots.partition_point(|v| *v < b) - 1;
            starts[axis] = right - degree;
            selected_counts[axis] = left - starts[axis] + 1;
            output_knots[axis] = vec![a; degree + 1];
            output_knots[axis].extend(knots.iter().copied().filter(|v| *v > a && *v < b));
            output_knots[axis].extend(vec![b; degree + 1]);
        }
        let source_counts = refined.control_counts();
        let mut points = Vec::with_capacity(selected_counts[0] * selected_counts[1]);
        let mut weights = Vec::with_capacity(points.capacity());
        for i in starts[0]..starts[0] + selected_counts[0] {
            for j in starts[1]..starts[1] + selected_counts[1] {
                let index = i * source_counts[1] + j;
                points.push(refined.control_points()[index]);
                weights.push(refined.weights()[index]);
            }
        }
        Self::new(degrees, output_knots, selected_counts, points, weights)
    }
}
