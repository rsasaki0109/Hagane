//! Positive-weight clamped NURBS curves, independently implemented using
//! homogeneous de Boor evaluation and the analytic rational derivative.
use crate::{Error, Point3, Result, Vec3};

/// Which one-sided limit to use at an interior knot. Endpoints always use
/// the inward limit. No fuzzy parameter snapping is performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotSide {
    Left,
    Right,
}

/// Immutable, validated, nonperiodic, clamped rational B-spline curve.
/// Interior multiplicity is at most the degree (position remains continuous).
///
/// ```
/// use hagane::{NurbsCurve, Point3};
/// let circle = NurbsCurve::new(2,
///     vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
///     vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0),
///          Point3::new(0.0, 1.0, 0.0)],
///     vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0])?;
/// let point = circle.evaluate(0.5)?;
/// assert!((point.x * point.x + point.y * point.y - 1.0).abs() < 1e-14);
/// # Ok::<(), hagane::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct NurbsCurve {
    degree: usize,
    knots: Vec<f64>,
    points: Vec<Point3>,
    weights: Vec<f64>,
    homogeneous: Vec<[f64; 4]>,
}
impl NurbsCurve {
    pub fn new(
        degree: usize,
        knots: Vec<f64>,
        points: Vec<Point3>,
        weights: Vec<f64>,
    ) -> Result<Self> {
        if !(1..=16).contains(&degree) || points.len() <= degree || points.len() > 65536 {
            return Err(Error::InvalidInput(
                "NURBS requires degree 1..16 and degree+1..65536 control points",
            ));
        }
        if knots.len() != points.len() + degree + 1 || weights.len() != points.len() {
            return Err(Error::InvalidInput(
                "NURBS knot/control/weight counts disagree",
            ));
        }
        if points.iter().any(|p| !p.finite())
            || knots.iter().any(|k| !k.is_finite())
            || weights.iter().any(|w| !w.is_finite() || *w <= 0.0)
        {
            return Err(Error::InvalidInput(
                "NURBS data must be finite with strictly positive weights",
            ));
        }
        if knots.windows(2).any(|k| k[0] > k[1]) {
            return Err(Error::InvalidInput("NURBS knots must be nondecreasing"));
        }
        let start = knots[degree];
        let end = knots[points.len()];
        if start >= end || !(end - start).is_finite() {
            return Err(Error::InvalidInput(
                "NURBS domain must have a positive finite span",
            ));
        }
        if knots[..=degree].iter().any(|k| *k != start)
            || knots[points.len()..].iter().any(|k| *k != end)
        {
            return Err(Error::Unsupported(
                "only clamped nonperiodic NURBS curves are supported",
            ));
        }
        let mut i = degree + 1;
        while i < points.len() {
            let knot = knots[i];
            let mut next = i + 1;
            while next < points.len() && knots[next] == knot {
                next += 1;
            }
            if knot <= start || knot >= end || next - i > degree {
                return Err(Error::InvalidInput("interior NURBS knots must be strictly inside the domain with multiplicity at most degree"));
            }
            i = next;
        }
        // Common weight scaling preserves the rational curve and avoids overflow
        // in weighted coordinates. Reject dynamic ranges that underflow to zero.
        let max = weights.iter().copied().fold(0.0, f64::max);
        let mut homogeneous = Vec::with_capacity(points.len());
        for (p, w) in points.iter().zip(&weights) {
            let w = w / max;
            if w == 0.0 {
                return Err(Error::InvalidInput("NURBS weight dynamic range underflows"));
            }
            let h = [p.x * w, p.y * w, p.z * w, w];
            if [p.x, p.y, p.z]
                .iter()
                .zip(&h[..3])
                .any(|(p, h)| *p != 0.0 && *h == 0.0)
            {
                return Err(Error::InvalidInput("NURBS weighted coordinate underflows"));
            }
            homogeneous.push(h);
        }
        Ok(Self {
            degree,
            knots,
            points,
            weights,
            homogeneous,
        })
    }
    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }
    pub fn control_points(&self) -> &[Point3] {
        &self.points
    }
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.knots[self.degree], self.knots[self.points.len()]]
    }
    fn span(&self, u: f64, side: KnotSide) -> Result<usize> {
        let [start, end] = self.domain();
        if !u.is_finite() || u < start || u > end {
            return Err(Error::InvalidInput(
                "NURBS parameter is outside its closed domain",
            ));
        }
        if u == end {
            return Ok(self.points.len() - 1);
        }
        if u == start {
            return Ok(self.degree);
        }
        let index = match side {
            KnotSide::Right => self.knots.partition_point(|k| *k <= u),
            KnotSide::Left => self.knots.partition_point(|k| *k < u),
        };
        Ok(index - 1)
    }
    pub fn evaluate(&self, u: f64) -> Result<Point3> {
        let span = self.span(u, KnotSide::Right)?;
        let h = de_boor(
            self.degree,
            &self.knots,
            span,
            u,
            self.homogeneous[span - self.degree..=span].to_vec(),
        )?;
        project(h)
    }
    /// Analytic first derivative. At a potentially C0 interior knot, use
    /// `evaluate_with_derivative` and explicitly choose a one-sided limit.
    pub fn derivative(&self, u: f64) -> Result<Vec3> {
        self.span(u, KnotSide::Right)?;
        let [start, end] = self.domain();
        if u != start && u != end && self.knots.iter().filter(|k| **k == u).count() == self.degree {
            return Err(Error::Unsupported(
                "derivative at a C0 knot requires an explicit side",
            ));
        }
        Ok(self.evaluate_with_derivative(u, KnotSide::Right)?.1)
    }
    pub fn evaluate_with_derivative(&self, u: f64, side: KnotSide) -> Result<(Point3, Vec3)> {
        let span = self.span(u, side)?;
        let h = de_boor(
            self.degree,
            &self.knots,
            span,
            u,
            self.homogeneous[span - self.degree..=span].to_vec(),
        )?;
        let point = project(h)?;
        let mut controls = Vec::with_capacity(self.degree);
        for i in span - self.degree..span {
            let denominator = self.knots[i + self.degree + 1] - self.knots[i + 1];
            let mut q = [0.0; 4];
            for (j, value) in q.iter_mut().enumerate() {
                *value = (self.homogeneous[i + 1][j] - self.homogeneous[i][j])
                    * (self.degree as f64 / denominator);
            }
            controls.push(q);
        }
        let dh = de_boor(
            self.degree - 1,
            &self.knots[1..self.knots.len() - 1],
            span - 1,
            u,
            controls,
        )?;
        let derivative = Vec3::new(
            (dh[0] - point.x * dh[3]) / h[3],
            (dh[1] - point.y * dh[3]) / h[3],
            (dh[2] - point.z * dh[3]) / h[3],
        );
        if !derivative.finite() {
            return Err(Error::InvalidInput(
                "NURBS derivative exceeds finite numerical range",
            ));
        }
        Ok((point, derivative))
    }
}
fn project(h: [f64; 4]) -> Result<Point3> {
    if h[3] <= 0.0 || !h[3].is_finite() {
        return Err(Error::InvalidInput(
            "NURBS homogeneous denominator is unusable",
        ));
    }
    let p = Point3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3]);
    if !p.finite() {
        return Err(Error::InvalidInput(
            "NURBS evaluation exceeds finite numerical range",
        ));
    }
    Ok(p)
}
fn de_boor(
    degree: usize,
    knots: &[f64],
    span: usize,
    u: f64,
    mut d: Vec<[f64; 4]>,
) -> Result<[f64; 4]> {
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = knots[i + degree + 1 - r] - knots[i];
            if denominator <= 0.0 || !denominator.is_finite() {
                return Err(Error::InvalidInput(
                    "NURBS knot interval is numerically unusable",
                ));
            }
            let alpha = (u - knots[i]) / denominator;
            let previous = d[j - 1];
            for (value, previous) in d[j].iter_mut().zip(previous) {
                *value = (1.0 - alpha) * previous + alpha * *value;
            }
        }
    }
    let result = d[degree];
    if result.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "NURBS homogeneous arithmetic exceeds finite range",
        ));
    }
    Ok(result)
}

/// Interactive curve fixture: quadratic control points (1,0), (1,1), (0,1).
/// Weight sqrt(1/2) yields an exact unit quarter circle. Display samples are
/// uniform in parameter and do not claim a certified chord-error bound.
pub fn nurbs_demo_json(middle_weight: f64, parameter: f64) -> Result<String> {
    let curve = NurbsCurve::new(
        2,
        vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        vec![1.0, middle_weight, 1.0],
    )?;
    let (point, tangent) = curve.evaluate_with_derivative(parameter, KnotSide::Right)?;
    let mut output=format!("{{\"degree\":2,\"weight\":{middle_weight},\"parameter\":{parameter},\"point\":[{},{},{}],\"tangent\":[{},{},{}],\"samples\":[",point.x,point.y,point.z,tangent.x,tangent.y,tangent.z);
    for i in 0..=128 {
        if i > 0 {
            output.push(',');
        }
        let p = curve.evaluate(i as f64 / 128.0)?;
        output.push_str(&format!("{},{},{}", p.x, p.y, p.z));
    }
    output.push_str("]}");
    Ok(output)
}
