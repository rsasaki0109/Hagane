//! Exact real-algebra Bernstein composition; binary64 admission uses conservative engineering guards.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported("rational surface composition arithmetic is unresolved")
}
fn checked(x: f64) -> Result<f64> {
    if !x.is_finite() || (x != 0. && x.abs() < f64::MIN_POSITIVE) {
        Err(unresolved())
    } else {
        Ok(x)
    }
}
fn mul(a: f64, b: f64) -> Result<f64> {
    let x = checked(a * b)?;
    if a != 0. && b != 0. && x == 0. {
        Err(unresolved())
    } else {
        Ok(x)
    }
}
fn divide(a: f64, b: f64) -> Result<f64> {
    let x = checked(a / b)?;
    if a != 0. && x == 0. {
        Err(unresolved())
    } else {
        Ok(x)
    }
}
fn binomial(n: usize, k: usize) -> f64 {
    (0..k.min(n - k)).fold(1., |v, i| v * (n - i) as f64 / (i + 1) as f64)
}
fn product(a: &[f64], b: &[f64]) -> Result<Vec<f64>> {
    let (p, q) = (a.len() - 1, b.len() - 1);
    let mut c = vec![0.; p + q + 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            let factor = binomial(p, i) * binomial(q, j) / binomial(p + q, i + j);
            let term = mul(mul(x, y)?, factor)?;
            c[i + j] = checked(c[i + j] + term)?;
        }
    }
    Ok(c)
}
fn power(a: &[f64], n: usize) -> Result<Vec<f64>> {
    let mut v = vec![1.];
    for _ in 0..n {
        v = product(&v, a)?;
    }
    Ok(v)
}
impl NurbsSurface {
    /// Compose a single rational Bezier UV curve with a single rational Bezier patch.
    /// UV controls must have zero Z and lie in the patch domain. The original curve
    /// parameter domain and raw weights are retained in the homogeneous algebra.
    /// Numerical guards are conservative engineering checks, not interval proofs.
    pub fn parameter_curve_nurbs(&self, uv: &NurbsCurve, tol: Tolerance) -> Result<NurbsCurve> {
        Tolerance::new(tol.linear)?;
        let [p, q] = self.degrees();
        let d = uv.degree();
        let degree = d.checked_mul(p + q).ok_or_else(unresolved)?;
        if degree > 16 {
            return Err(Error::Unsupported(
                "rational surface composition degree exceeds sixteen",
            ));
        }
        if self.control_counts() != [p + 1, q + 1] || uv.control_points().len() != d + 1 {
            return Err(Error::Unsupported(
                "rational surface composition requires single Bezier geometries",
            ));
        }
        let domain = self.domain();
        let td = uv.domain();
        let span = [domain[0][1] - domain[0][0], domain[1][1] - domain[1][0]];
        if span.iter().any(|x| !x.is_finite() || *x <= 0.) || !((td[1] - td[0]).is_finite()) {
            return Err(unresolved());
        }
        let origin = self.control_points()[0];
        let world = self
            .control_points()
            .iter()
            .fold(f64::MIN_POSITIVE, |m, x| {
                m.max(x.x.abs()).max(x.y.abs()).max(x.z.abs())
            });
        let sr = self.weights().iter().copied().fold(0., f64::max)
            / self.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let ur = uv.weights().iter().copied().fold(0., f64::max)
            / uv.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let condition = checked(sr * ur.powi((p + q) as i32))?;
        let mut guard = 4096. * f64::EPSILON * world * condition * (degree + 2) as f64;
        let diameter = self
            .control_points()
            .iter()
            .map(|x| {
                let v = *x - origin;
                v.x.hypot(v.y).hypot(v.z)
            })
            .fold(0., f64::max);
        for (a, range) in domain.iter().enumerate() {
            let scale = range[0].abs().max(range[1].abs()).max(span[a]);
            guard +=
                4096. * f64::EPSILON * scale / span[a] * diameter * condition * (p + q + 2) as f64;
        }
        if !guard.is_finite() || guard >= tol.linear {
            return Err(unresolved());
        }
        let mut axes: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::with_capacity(d + 1));
        for (point, &w) in uv.control_points().iter().zip(uv.weights()) {
            if point.z != 0.
                || point.x < domain[0][0]
                || point.x > domain[0][1]
                || point.y < domain[1][0]
                || point.y > domain[1][1]
            {
                return Err(Error::InvalidInput(
                    "rational UV controls must be planar and inside the source patch",
                ));
            }
            for (a, range) in domain.iter().enumerate() {
                let coord = if a == 0 { point.x } else { point.y };
                let s = divide(checked(coord - range[0])?, span[a])?;
                let t = divide(checked(range[1] - coord)?, span[a])?;
                axes[2 * a].push(mul(s, w)?);
                axes[2 * a + 1].push(mul(t, w)?);
            }
        }
        let mut h = vec![[0.; 4]; degree + 1];
        for i in 0..=p {
            let a = product(&power(&axes[0], i)?, &power(&axes[1], p - i)?)?;
            for j in 0..=q {
                let b = product(&power(&axes[2], j)?, &power(&axes[3], q - j)?)?;
                let basis = product(&a, &b)?;
                let idx = i * (q + 1) + j;
                let local = self.control_points()[idx] - origin;
                if !local.finite() {
                    return Err(unresolved());
                }
                let scale = mul(self.weights()[idx], binomial(p, i) * binomial(q, j))?;
                for (k, &v) in basis.iter().enumerate() {
                    let w = mul(v, scale)?;
                    for (a, coord) in [local.x, local.y, local.z, 1.].into_iter().enumerate() {
                        h[k][a] = checked(h[k][a] + mul(w, coord)?)?;
                    }
                }
            }
        }
        let mut points = Vec::with_capacity(degree + 1);
        let mut weights = Vec::with_capacity(degree + 1);
        for v in h {
            if v[3] <= 0. {
                return Err(unresolved());
            }
            let local = Vec3::new(
                divide(v[0], v[3])?,
                divide(v[1], v[3])?,
                divide(v[2], v[3])?,
            );
            let point = origin + local;
            if !point.finite() {
                return Err(unresolved());
            }
            for (a, b) in [
                (local.x, point.x - origin.x),
                (local.y, point.y - origin.y),
                (local.z, point.z - origin.z),
            ] {
                if a != 0. && b == 0. {
                    return Err(unresolved());
                }
            }
            points.push(point);
            weights.push(v[3]);
        }
        let mut knots = vec![td[0]; degree + 1];
        knots.extend(vec![td[1]; degree + 1]);
        NurbsCurve::new(degree, knots, points, weights)
    }
}
