//! Shape-preserving rational B-spline refinement and bounded display sampling.
//! Algorithms use homogeneous knot insertion and positive Bernstein convex hulls.
use crate::nurbs::{knot_span, project, weighted_controls};
use crate::{Error, KnotSide, NurbsCurve, Point3, Result};

#[derive(Clone, Debug)]
pub struct RationalBezierSpan {
    pub parameter_range: [f64; 2],
    pub control_points: Vec<Point3>,
    pub weights: Vec<f64>,
}
impl RationalBezierSpan {
    /// Evaluate in the original curve parameter interval.
    pub fn evaluate(&self, parameter: f64) -> Result<Point3> {
        let [a, b] = self.parameter_range;
        if !a.is_finite()
            || !b.is_finite()
            || a >= b
            || !(b - a).is_finite()
            || !(2..=17).contains(&self.control_points.len())
            || !parameter.is_finite()
            || parameter < a
            || parameter > b
        {
            return Err(Error::InvalidInput("Bezier parameter is outside its span"));
        }
        let mut h = weighted_controls(&self.control_points, &self.weights)?;
        let u = (parameter - a) / (b - a);
        for n in (1..h.len()).rev() {
            for i in 0..n {
                let next = h[i + 1];
                for (value, next) in h[i].iter_mut().zip(next) {
                    *value = (1.0 - u) * *value + u * next;
                }
            }
        }
        project(h[0])
    }
}
#[derive(Clone, Debug)]
pub struct NurbsPolyline {
    pub points: Vec<Point3>,
    pub parameters: Vec<f64>,
    /// Conservative curve-to-chord distance bound for each parameter interval.
    pub error_bounds: Vec<f64>,
}
impl NurbsCurve {
    /// Insert an interior knot without changing the mathematical curve.
    /// Multiplicity cannot exceed the degree; endpoint insertion is unsupported.
    pub fn insert_knot(&self, parameter: f64, times: usize) -> Result<Self> {
        let p = self.degree();
        let mut k = self.knots().to_vec();
        let mut points = self.control_points().to_vec();
        let mut weights = self.weights().to_vec();
        let [a, b] = self.domain();
        if !parameter.is_finite() || parameter < a || parameter > b {
            return Err(Error::InvalidInput(
                "knot insertion parameter is outside the domain",
            ));
        }
        if times == 0 {
            return Ok(self.clone());
        }
        if parameter == a || parameter == b {
            return Err(Error::Unsupported(
                "clamped endpoint knots cannot be inserted",
            ));
        }
        let multiplicity = k.iter().filter(|v| **v == parameter).count();
        if times > p - multiplicity || points.len() + times > 65536 {
            return Err(Error::InvalidInput(
                "knot insertion exceeds multiplicity or control resource limit",
            ));
        }
        for _ in 0..times {
            let span = knot_span(p, &k, points.len(), parameter, KnotSide::Right)?;
            let s = k.iter().filter(|v| **v == parameter).count();
            let h = weighted_controls(&points, &weights)?;
            let mut q = vec![[0.0; 4]; h.len() + 1];
            q[..=span - p].copy_from_slice(&h[..=span - p]);
            q[span - s + 1..].copy_from_slice(&h[span - s..]);
            for i in span - p + 1..=span - s {
                let denominator = k[i + p] - k[i];
                if denominator <= 0.0 || !denominator.is_finite() {
                    return Err(Error::InvalidInput("knot insertion interval is unusable"));
                }
                let alpha = (parameter - k[i]) / denominator;
                for j in 0..4 {
                    q[i][j] = (1.0 - alpha) * h[i - 1][j] + alpha * h[i][j];
                }
            }
            points = q.iter().map(|v| project(*v)).collect::<Result<Vec<_>>>()?;
            weights = q.iter().map(|v| v[3]).collect();
            k.insert(span + 1, parameter);
        }
        Self::new(p, k, points, weights)
    }
    /// Extract exact rational Bezier pieces by elevating each interior knot's
    /// multiplicity to the degree. No geometric approximation is performed.
    pub fn bezier_spans(&self) -> Result<Vec<RationalBezierSpan>> {
        let mut curve = self.clone();
        let [a, b] = self.domain();
        let mut interiors = self
            .knots()
            .iter()
            .copied()
            .filter(|v| *v > a && *v < b)
            .collect::<Vec<_>>();
        interiors.dedup();
        let required_controls = interiors
            .iter()
            .fold(self.control_points().len(), |count, u| {
                let multiplicity = self.knots().partition_point(|v| *v <= *u)
                    - self.knots().partition_point(|v| *v < *u);
                count.saturating_add(self.degree() - multiplicity)
            });
        if required_controls > 65536 {
            return Err(Error::Unsupported(
                "Bezier extraction exceeds control limit",
            ));
        }
        let mut refinement_work = 0usize;
        for u in interiors {
            let count = curve.knots().partition_point(|v| *v <= u)
                - curve.knots().partition_point(|v| *v < u);
            let insertions = self.degree() - count;
            if insertions == 0 {
                continue;
            }
            refinement_work = refinement_work.saturating_add(
                insertions.saturating_mul(curve.control_points().len().saturating_add(insertions)),
            );
            if refinement_work > 16_000_000 {
                return Err(Error::Unsupported(
                    "Bezier extraction exceeds refinement work limit",
                ));
            }
            curve = curve.insert_knot(u, insertions)?;
        }
        let p = curve.degree();
        let mut output = Vec::new();
        for i in p..curve.control_points().len() {
            if curve.knots()[i] < curve.knots()[i + 1] {
                output.push(RationalBezierSpan {
                    parameter_range: [curve.knots()[i], curve.knots()[i + 1]],
                    control_points: curve.control_points()[i - p..=i].to_vec(),
                    weights: curve.weights()[i - p..=i].to_vec(),
                });
            }
        }
        Ok(output)
    }
    /// Adaptively approximate the curve with explicit per-chord distance bounds.
    /// Extremely ill-conditioned weights or coordinates are rejected if their
    /// conservative arithmetic allowance exhausts the requested error budget.
    pub fn tessellate_bounded(
        &self,
        chord_error: f64,
        max_segments: usize,
    ) -> Result<NurbsPolyline> {
        if !chord_error.is_finite()
            || chord_error <= 0.0
            || !(1..=1_000_000).contains(&max_segments)
        {
            return Err(Error::InvalidInput(
                "NURBS sampling needs positive finite error and 1..1000000 segments",
            ));
        }
        let scale = self
            .control_points()
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(0.0, f64::max);
        let min = self.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let max = self.weights().iter().copied().fold(0.0, f64::max);
        let arithmetic = 4096.0
            * f64::EPSILON
            * scale.max(f64::MIN_POSITIVE)
            * (max / min)
            * (self.degree() as f64 + 1.0);
        if !arithmetic.is_finite() || arithmetic >= chord_error * 0.25 {
            return Err(Error::Tessellation(
                "NURBS coordinate/weight precision cannot resolve the requested chord error",
            ));
        }
        let span_count = self
            .knots()
            .windows(2)
            .filter(|pair| pair[0] < pair[1])
            .count();
        if span_count > max_segments {
            return Err(Error::Tessellation(
                "NURBS span count exceeds segment limit",
            ));
        }
        let spans = self.bezier_spans()?;
        if spans.len() > max_segments {
            return Err(Error::Tessellation(
                "NURBS span count exceeds segment limit",
            ));
        }
        let mut output = NurbsPolyline {
            points: Vec::new(),
            parameters: Vec::new(),
            error_bounds: Vec::new(),
        };
        for span in spans {
            let origin = span.control_points[0];
            let local = span
                .control_points
                .iter()
                .map(|p| *p - origin)
                .collect::<Vec<_>>();
            let h = weighted_controls(&local, &span.weights)?;
            if output.points.is_empty() {
                output.points.push(origin);
                output.parameters.push(span.parameter_range[0]);
            }
            let mut stack = vec![(h, span.parameter_range, 0usize)];
            while let Some((h, [a, b], depth)) = stack.pop() {
                let points = h.iter().map(|v| project(*v)).collect::<Result<Vec<_>>>()?;
                let bound = points.iter().try_fold(0.0f64, |bound, point| {
                    Ok::<_, Error>(bound.max(chord_distance(
                        *point,
                        points[0],
                        *points.last().unwrap(),
                    )?))
                })? + arithmetic;
                if !bound.is_finite() {
                    return Err(Error::Tessellation(
                        "NURBS chord bound exceeds numerical range",
                    ));
                }
                if bound <= chord_error {
                    if output.error_bounds.len() == max_segments {
                        return Err(Error::Tessellation(
                            "NURBS subdivision exceeds segment limit",
                        ));
                    }
                    output.points.push(origin + *points.last().unwrap());
                    output.parameters.push(b);
                    output.error_bounds.push(bound);
                } else {
                    let middle = a + (b - a) * 0.5;
                    if depth == 48
                        || middle == a
                        || middle == b
                        || output.error_bounds.len() + stack.len() + 2 > max_segments
                    {
                        return Err(Error::Tessellation(
                            "NURBS subdivision cannot satisfy error within resource limits",
                        ));
                    }
                    let fraction = (middle - a) / (b - a);
                    let (left, right) = split(&h, fraction);
                    stack.push((right, [middle, b], depth + 1));
                    stack.push((left, [a, middle], depth + 1));
                }
            }
        }
        Ok(output)
    }
}
fn split(h: &[[f64; 4]], fraction: f64) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let n = h.len();
    let mut work = h.to_vec();
    let mut left = vec![h[0]];
    let mut right = vec![h[n - 1]];
    for remaining in (1..n).rev() {
        for i in 0..remaining {
            let next = work[i + 1];
            for (value, next) in work[i].iter_mut().zip(next) {
                *value = *value * (1.0 - fraction) + next * fraction;
            }
        }
        left.push(work[0]);
        right.push(work[remaining - 1]);
    }
    right.reverse();
    (left, right)
}
fn chord_distance(point: Point3, a: Point3, b: Point3) -> Result<f64> {
    let d = b - a;
    let length = d.norm();
    if !length.is_finite() || !(point - a).finite() {
        return Err(Error::Tessellation(
            "NURBS chord arithmetic exceeds finite range",
        ));
    }
    if length == 0.0 {
        return Ok((point - a).norm());
    }
    let direction = d.normalized()?;
    let projection = (point - a).dot(direction);
    if !projection.is_finite() {
        return Err(Error::Tessellation(
            "NURBS chord projection exceeds finite range",
        ));
    }
    let along = projection.clamp(0.0, length);
    Ok((point - a - direction * along).norm())
}
