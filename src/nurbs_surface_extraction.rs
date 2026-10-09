//! Exact tensor-product rational Bezier patch extraction.
use crate::{Error, NurbsSurface, Result};
#[derive(Clone, Debug)]
pub struct RationalBezierPatch {
    pub surface: NurbsSurface,
    pub parameter_ranges: [[f64; 2]; 2],
}
impl NurbsSurface {
    /// Refine both axes to Bezier multiplicity, retaining original parameter
    /// domains and one common rational control-net weight scale. C0 is supported.
    pub fn bezier_patches(&self) -> Result<Vec<RationalBezierPatch>> {
        let degrees = self.degrees();
        let counts = self.control_counts();
        let mut insertions: [Vec<(f64, usize)>; 2] = [Vec::new(), Vec::new()];
        let mut final_counts = counts;
        let mut patch_counts = [1usize; 2];
        for axis in 0..2 {
            let knots = self.knots(axis)?;
            let [a, b] = self.domain()[axis];
            let mut i = degrees[axis] + 1;
            while i < counts[axis] {
                let value = knots[i];
                let next = knots.partition_point(|v| *v <= value);
                if value > a && value < b {
                    let times = degrees[axis] - (next - i);
                    final_counts[axis] = final_counts[axis].saturating_add(times);
                    patch_counts[axis] += 1;
                    if times > 0 {
                        insertions[axis].push((value, times));
                    }
                }
                i = next;
            }
        }
        if final_counts[0].saturating_mul(final_counts[1]) > 65536 {
            return Err(Error::Unsupported(
                "surface Bezier extraction exceeds 65536 controls",
            ));
        }
        if patch_counts[0].saturating_mul(patch_counts[1]) > 65536 {
            return Err(Error::Unsupported(
                "surface Bezier extraction exceeds patch limit",
            ));
        }
        let mut work = 0usize;
        let mut current = counts;
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
                "surface Bezier extraction exceeds work limit",
            ));
        }
        let mut refined = self.clone();
        for (axis, axis_insertions) in insertions.into_iter().enumerate() {
            for (parameter, times) in axis_insertions {
                refined = refined.insert_knot(axis, parameter, times)?;
            }
        }
        let [nu, nv] = refined.control_counts();
        let [p, q] = degrees;
        let ku = refined.knots(0)?;
        let kv = refined.knots(1)?;
        let mut patches = Vec::with_capacity(patch_counts[0] * patch_counts[1]);
        for i in p..nu {
            if ku[i] == ku[i + 1] {
                continue;
            }
            for j in q..nv {
                if kv[j] == kv[j + 1] {
                    continue;
                }
                let ranges = [[ku[i], ku[i + 1]], [kv[j], kv[j + 1]]];
                let mut points = Vec::with_capacity((p + 1) * (q + 1));
                let mut weights = Vec::with_capacity(points.capacity());
                for u in i - p..=i {
                    for v in j - q..=j {
                        let index = u * nv + v;
                        points.push(refined.control_points()[index]);
                        weights.push(refined.weights()[index]);
                    }
                }
                let knots = std::array::from_fn(|axis| {
                    let mut knots = vec![ranges[axis][0]; degrees[axis] + 1];
                    knots.extend(vec![ranges[axis][1]; degrees[axis] + 1]);
                    knots
                });
                patches.push(RationalBezierPatch {
                    surface: Self::new(degrees, knots, [p + 1, q + 1], points, weights)?,
                    parameter_ranges: ranges,
                });
            }
        }
        Ok(patches)
    }
}
