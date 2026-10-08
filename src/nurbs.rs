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
        validate_axis(degree, &knots, points.len())?;
        let homogeneous = weighted_controls(&points, &weights)?;
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
        knot_span(self.degree, &self.knots, self.points.len(), u, side)
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
        let (h, dh) = homogeneous_jet(
            self.degree,
            &self.knots,
            span,
            u,
            self.homogeneous[span - self.degree..=span].to_vec(),
        )?;
        let point = project(h)?;
        Ok((point, rational_partial(point, h, dh)?))
    }
}
pub(crate) fn project(h: [f64; 4]) -> Result<Point3> {
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
pub(crate) fn de_boor(
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

// Shared axis/weight validation for curves and tensor-product surfaces.
pub(crate) fn validate_axis(degree: usize, knots: &[f64], count: usize) -> Result<()> {
    if !(1..=16).contains(&degree) || count <= degree || count > 65536 {
        return Err(Error::InvalidInput(
            "NURBS requires degree 1..16 and degree+1..65536 control points per axis",
        ));
    }
    if knots.len() != count + degree + 1 {
        return Err(Error::InvalidInput(
            "NURBS knot and control counts disagree",
        ));
    }
    if knots.iter().any(|k| !k.is_finite()) || knots.windows(2).any(|k| k[0] > k[1]) {
        return Err(Error::InvalidInput(
            "NURBS knots must be finite and nondecreasing",
        ));
    }
    let start = knots[degree];
    let end = knots[count];
    if start >= end || !(end - start).is_finite() {
        return Err(Error::InvalidInput(
            "NURBS domain must have a positive finite span",
        ));
    }
    if knots[..=degree].iter().any(|k| *k != start) || knots[count..].iter().any(|k| *k != end) {
        return Err(Error::Unsupported(
            "only clamped nonperiodic NURBS axes are supported",
        ));
    }
    let mut i = degree + 1;
    while i < count {
        let knot = knots[i];
        let mut next = i + 1;
        while next < count && knots[next] == knot {
            next += 1;
        }
        if knot <= start || knot >= end || next - i > degree {
            return Err(Error::InvalidInput(
                "interior NURBS knots must lie inside the domain with multiplicity at most degree",
            ));
        }
        i = next;
    }
    Ok(())
}
pub(crate) fn weighted_controls(points: &[Point3], weights: &[f64]) -> Result<Vec<[f64; 4]>> {
    if weights.len() != points.len() || weights.is_empty() {
        return Err(Error::InvalidInput(
            "NURBS control and weight counts disagree",
        ));
    }
    if points.iter().any(|p| !p.finite()) || weights.iter().any(|w| !w.is_finite() || *w <= 0.0) {
        return Err(Error::InvalidInput(
            "NURBS controls must be finite with strictly positive weights",
        ));
    }
    let max = weights.iter().copied().fold(0.0, f64::max);
    let mut output = Vec::with_capacity(points.len());
    for (p, w) in points.iter().zip(weights) {
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
        output.push(h);
    }
    Ok(output)
}
pub(crate) fn knot_span(
    degree: usize,
    knots: &[f64],
    count: usize,
    u: f64,
    side: KnotSide,
) -> Result<usize> {
    let start = knots[degree];
    let end = knots[count];
    if !u.is_finite() || u < start || u > end {
        return Err(Error::InvalidInput(
            "NURBS parameter is outside its closed domain",
        ));
    }
    if u == end {
        return Ok(count - 1);
    }
    if u == start {
        return Ok(degree);
    }
    let index = match side {
        KnotSide::Right => knots.partition_point(|k| *k <= u),
        KnotSide::Left => knots.partition_point(|k| *k < u),
    };
    Ok(index - 1)
}
pub(crate) fn is_c0(degree: usize, knots: &[f64], count: usize, u: f64) -> bool {
    u != knots[degree] && u != knots[count] && knots.iter().filter(|k| **k == u).count() == degree
}
pub(crate) fn homogeneous_jet(
    degree: usize,
    knots: &[f64],
    span: usize,
    u: f64,
    controls: Vec<[f64; 4]>,
) -> Result<([f64; 4], [f64; 4])> {
    let mut derivative = Vec::with_capacity(degree);
    for j in 0..degree {
        let i = span - degree + j;
        let denominator = knots[i + degree + 1] - knots[i + 1];
        let mut q = [0.0; 4];
        for (k, value) in q.iter_mut().enumerate() {
            *value = (controls[j + 1][k] - controls[j][k]) * (degree as f64 / denominator);
        }
        derivative.push(q);
    }
    let h = de_boor(degree, knots, span, u, controls)?;
    let dh = de_boor(
        degree - 1,
        &knots[1..knots.len() - 1],
        span - 1,
        u,
        derivative,
    )?;
    Ok((h, dh))
}
pub(crate) fn rational_partial(point: Point3, h: [f64; 4], dh: [f64; 4]) -> Result<Vec3> {
    let d = Vec3::new(
        (dh[0] - point.x * dh[3]) / h[3],
        (dh[1] - point.y * dh[3]) / h[3],
        (dh[2] - point.z * dh[3]) / h[3],
    );
    if !d.finite() {
        return Err(Error::InvalidInput(
            "NURBS derivative exceeds finite numerical range",
        ));
    }
    Ok(d)
}
