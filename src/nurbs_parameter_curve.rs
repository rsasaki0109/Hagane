//! Exact rational composition of tensor Bernstein patches with affine UV lines.
use crate::nurbs::project;
use crate::*;
#[derive(Clone)]
struct Break {
    t: f64,
    uv: [f64; 2],
    axis: Option<usize>,
}
impl NurbsSurface {
    /// Exact mathematical affine-UV section, parameterized on t in [0,1].
    /// Homogeneous Bernstein composition is used; no points are fitted.
    pub fn parameter_curve(&self, start: [f64; 2], end: [f64; 2]) -> Result<NurbsCurve> {
        let domain = self.domain();
        let delta = [end[0] - start[0], end[1] - start[1]];
        for axis in 0..2 {
            if !start[axis].is_finite()
                || !end[axis].is_finite()
                || start[axis] < domain[axis][0]
                || start[axis] > domain[axis][1]
                || end[axis] < domain[axis][0]
                || end[axis] > domain[axis][1]
                || !delta[axis].is_finite()
            {
                return Err(Error::InvalidInput(
                    "surface parameter curve endpoints must be finite and inside the source domain",
                ));
            }
        }
        for axis in 0..2 {
            let scale = domain[axis][0]
                .abs()
                .max(domain[axis][1].abs())
                .max(domain[axis][1] - domain[axis][0])
                .max(f64::MIN_POSITIVE);
            if delta[axis] != 0. && delta[axis].abs() <= 128. * f64::EPSILON * scale {
                return Err(Error::Unsupported(
                    "surface UV direction is too short for stable affine composition",
                ));
            }
        }
        if delta == [0., 0.] {
            return Err(Error::InvalidInput(
                "surface parameter curve needs distinct UV endpoints",
            ));
        }
        let degrees = self.degrees();
        let effective = [
            if delta[0] == 0. { 0 } else { degrees[0] },
            if delta[1] == 0. { 0 } else { degrees[1] },
        ];
        let degree = effective[0] + effective[1];
        if degree > 16 {
            return Err(Error::Unsupported(
                "affine UV composition requires combined curve degree at most 16",
            ));
        }
        let mut breaks = vec![
            Break {
                t: 0.,
                uv: start,
                axis: None,
            },
            Break {
                t: 1.,
                uv: end,
                axis: None,
            },
        ];
        let mut intervals: [Vec<[f64; 2]>; 2] = [Vec::new(), Vec::new()];
        for axis in 0..2 {
            let knots = self.knots(axis)?;
            intervals[axis] = knots
                .windows(2)
                .filter(|pair| pair[0] < pair[1])
                .map(|pair| [pair[0], pair[1]])
                .collect();
            if delta[axis] == 0. {
                continue;
            }
            let mut unique = knots.to_vec();
            unique.dedup();
            for knot in unique {
                if knot > start[axis].min(end[axis]) && knot < start[axis].max(end[axis]) {
                    let t = (knot - start[axis]) / delta[axis];
                    if !t.is_finite() || t <= 128. * f64::EPSILON || t >= 1. - 128. * f64::EPSILON {
                        return Err(Error::Unsupported(
                            "surface parameter curve knot crossing is numerically unresolved",
                        ));
                    }
                    let mut uv = [start[0] + delta[0] * t, start[1] + delta[1] * t];
                    uv[axis] = knot;
                    breaks.push(Break {
                        t,
                        uv,
                        axis: Some(axis),
                    });
                }
            }
        }
        breaks.sort_by(|a, b| a.t.total_cmp(&b.t));
        let mut merged: Vec<Break> = Vec::with_capacity(breaks.len());
        for candidate in breaks {
            if let Some(previous) = merged.last_mut() {
                if candidate.t - previous.t <= 128. * f64::EPSILON {
                    if let (Some(a), Some(b)) = (previous.axis, candidate.axis) {
                        if a != b {
                            let mut uv = previous.uv;
                            uv[b] = candidate.uv[b];
                            if orient2d(start, end, uv)? == Orientation::Collinear {
                                previous.uv = uv;
                                previous.axis = None;
                                continue;
                            }
                        }
                    }
                    return Err(Error::Unsupported(
                        "distinct UV knot crossings are too close to resolve",
                    ));
                }
            }
            merged.push(candidate);
        }
        let spans = merged.len() - 1;
        if spans > 4096 || spans.saturating_mul(degree).saturating_add(1) > 65536 {
            return Err(Error::Unsupported(
                "surface parameter curve exceeds span or control limit",
            ));
        }
        if spans
            .saturating_mul(degrees[0] + 1)
            .saturating_mul(degrees[1] + 1)
            .saturating_mul(degrees[0] + degrees[1] + 2)
            > 16_000_000
        {
            return Err(Error::Unsupported(
                "surface parameter curve exceeds composition work limit",
            ));
        }
        let patches = self.bezier_patches()?;
        let weight_scale = patches
            .iter()
            .flat_map(|patch| patch.surface.weights())
            .copied()
            .fold(0., f64::max);
        let source_scale = self
            .control_points()
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(0., f64::max)
            .max(f64::MIN_POSITIVE);
        let origin = self.control_points()[0];
        let mut output: Vec<[f64; 4]> = Vec::with_capacity(spans * degree + 1);
        let mut knots = vec![0.; degree + 1];
        for (span, pair) in merged.windows(2).enumerate() {
            let middle = pair[0].t + (pair[1].t - pair[0].t) * 0.5;
            if middle == pair[0].t || middle == pair[1].t {
                return Err(Error::Unsupported(
                    "surface parameter curve span has no representable midpoint",
                ));
            }
            let uv = [start[0] + delta[0] * middle, start[1] + delta[1] * middle];
            let ui = intervals[0]
                .partition_point(|range| range[1] <= uv[0])
                .min(intervals[0].len() - 1);
            let vi = intervals[1]
                .partition_point(|range| range[1] <= uv[1])
                .min(intervals[1].len() - 1);
            let patch = &patches[ui * intervals[1].len() + vi].surface;
            let ranges = patch.domain();
            let mut h = patch
                .control_points()
                .iter()
                .zip(patch.weights())
                .map(|(point, weight)| {
                    let w = weight / weight_scale;
                    let p = *point - origin;
                    if !p.finite() || !w.is_finite() || w <= 0. {
                        return Err(Error::InvalidInput(
                            "surface parameter curve local homogeneous control is unusable",
                        ));
                    }
                    let h = [p.x * w, p.y * w, p.z * w, w];
                    if [p.x, p.y, p.z]
                        .iter()
                        .zip(&h[..3])
                        .any(|(p, h)| *p != 0. && *h == 0.)
                    {
                        return Err(Error::InvalidInput(
                            "surface parameter curve weighted local control underflows",
                        ));
                    }
                    if h.iter().any(|v| !v.is_finite()) {
                        return Err(Error::InvalidInput(
                            "surface parameter curve homogeneous control exceeds finite range",
                        ));
                    }
                    Ok(h)
                })
                .collect::<Result<Vec<_>>>()?;
            let mut counts = patch.control_counts();
            for axis in 0..2 {
                let a = (pair[0].uv[axis] - ranges[axis][0]) / (ranges[axis][1] - ranges[axis][0]);
                let b = (pair[1].uv[axis] - ranges[axis][0]) / (ranges[axis][1] - ranges[axis][0]);
                if !a.is_finite()
                    || !b.is_finite()
                    || !(0.0..=1.0).contains(&a)
                    || !(0.0..=1.0).contains(&b)
                {
                    return Err(Error::Unsupported(
                        "surface UV crossing loses patch parameter agreement",
                    ));
                }
                if delta[axis] != 0. && a == b {
                    return Err(Error::Unsupported(
                        "surface parameter curve axis interval rounds to zero",
                    ));
                }
                h = restrict_axis(&h, counts, axis, a, b);
                counts[axis] = if delta[axis] == 0. { 1 } else { counts[axis] };
            }
            let mut controls = vec![[0.; 4]; degree + 1];
            for i in 0..counts[0] {
                for j in 0..counts[1] {
                    let factor = binomial(effective[0], i) * binomial(effective[1], j)
                        / binomial(degree, i + j);
                    for (value, h) in controls[i + j].iter_mut().zip(h[i * counts[1] + j]) {
                        *value += factor * h;
                    }
                }
            }
            if let Some(previous) = output.last() {
                let a = project(*previous)?;
                let b = project(controls[0])?;
                let denominator = previous[3].abs().max(controls[0][3].abs());
                if (a - b).norm() > 512. * f64::EPSILON * source_scale * (degree + 1) as f64
                    || (previous[3] - controls[0][3]).abs()
                        > 512. * f64::EPSILON * denominator * (degree + 1) as f64
                {
                    return Err(Error::Unsupported(
                        "surface parameter curve homogeneous joins lose continuity",
                    ));
                }
                controls[0] = *previous;
            }
            output.extend(controls.into_iter().skip(usize::from(span > 0)));
            if span + 1 < spans {
                knots.extend(vec![pair[1].t; degree]);
            }
        }
        knots.extend(vec![1.; degree + 1]);
        let points = output
            .iter()
            .map(|h| Ok(origin + project(*h)?))
            .collect::<Result<Vec<_>>>()?;
        let weights = output.iter().map(|h| h[3]).collect();
        NurbsCurve::new(degree, knots, points, weights)
    }
}
fn binomial(n: usize, k: usize) -> f64 {
    let mut value = 1.;
    for i in 0..k {
        value *= (n - i) as f64 / (i + 1) as f64;
    }
    value
}
fn split(h: &[[f64; 4]], t: f64) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut work = h.to_vec();
    let n = h.len();
    let mut left = vec![h[0]];
    let mut right = vec![h[n - 1]];
    for remaining in (1..n).rev() {
        for i in 0..remaining {
            let next = work[i + 1];
            for (value, next) in work[i].iter_mut().zip(next) {
                *value = (1. - t) * *value + t * next;
            }
        }
        left.push(work[0]);
        right.push(work[remaining - 1]);
    }
    right.reverse();
    (left, right)
}
fn restrict_axis(h: &[[f64; 4]], counts: [usize; 2], axis: usize, a: f64, b: f64) -> Vec<[f64; 4]> {
    let n = counts[axis];
    let rows = counts[1 - axis];
    let out_n = if a == b { 1 } else { n };
    let mut out_counts = counts;
    out_counts[axis] = out_n;
    let mut output = vec![[0.; 4]; out_counts[0] * out_counts[1]];
    let index = |i, j, c: [usize; 2]| {
        if axis == 0 {
            i * c[1] + j
        } else {
            j * c[1] + i
        }
    };
    for row in 0..rows {
        let h = (0..n).map(|i| h[index(i, row, counts)]).collect::<Vec<_>>();
        let mut values = if a == b {
            vec![split(&h, a).0[n - 1]]
        } else {
            let lo = a.min(b);
            let hi = a.max(b);
            let left = if hi == 1. { h } else { split(&h, hi).0 };
            if lo == 0. {
                left
            } else {
                split(&left, lo / hi).1
            }
        };
        if a > b {
            values.reverse();
        }
        for (i, value) in values.into_iter().enumerate() {
            output[index(i, row, out_counts)] = value;
        }
    }
    output
}
