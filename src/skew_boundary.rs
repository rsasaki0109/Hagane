//! Euclidean boundary-band decisions for circular translation walls.
//! A chord swept along the exact generator is a parallelogram. Its Hausdorff
//! distance from the corresponding arc patch is bounded by the circle sagitta.
//! Refinement decides a band; it never substitutes a mesh for the analytic wall.
use crate::*;
use std::f64::consts::FRAC_PI_2;

fn segment_closest(p: Point3, a: Point3, b: Point3) -> Result<Point3> {
    let e = b - a;
    let length = e.norm();
    let unit = e.normalized()?;
    let travel = (p - a).dot(unit).clamp(0., length);
    let q = a + unit * travel;
    if !length.is_finite() || !travel.is_finite() || !q.finite() {
        return Err(Error::Unsupported(
            "skew boundary segment exceeds finite range",
        ));
    }
    Ok(q)
}

fn patch_closest(p: Point3, a: Point3, b: Point3, g: Vec3, ratio: f64) -> Result<(Point3, f64)> {
    let e = b - a;
    let eu = e.normalized()?;
    let gu = g.normalized()?;
    let cross = eu.cross(gu);
    if cross.norm() <= 256. * f64::EPSILON {
        return Err(Error::Unsupported(
            "skew boundary chord patch is ill-conditioned",
        ));
    }
    let n = cross.normalized()?;
    let q = p - n * (p - a).dot(n);
    let s = (q - a).cross(gu).dot(n) / e.cross(gu).dot(n);
    let t = eu.cross(q - a).dot(n) / eu.cross(g).dot(n);
    if !q.finite() || !s.is_finite() || !t.is_finite() {
        return Err(Error::Unsupported(
            "skew boundary patch projection exceeds finite range",
        ));
    }
    let condition = cross.norm().powi(2);
    let coordinate_guard = 256. * f64::EPSILON * (p - a).norm().max(e.norm()).max(g.norm())
        / (condition * e.norm().min(g.norm()));
    if !coordinate_guard.is_finite() || coordinate_guard >= 0.1 {
        return Err(Error::Unsupported(
            "skew boundary patch coordinates are unresolved",
        ));
    }
    // Near a trapezoid edge, plane distance remains a conservative lower bound.
    // Avoid selecting a farther edge from uncertain projected coordinates.
    if (-coordinate_guard..=1. + coordinate_guard).contains(&s)
        && t >= -coordinate_guard
        && t <= 1. + (ratio - 1.) * s + coordinate_guard * ratio.max(1.)
    {
        return Ok((q, condition));
    }
    let mut best = a;
    let mut distance = f64::INFINITY;
    for (start, end) in [
        (a, b),
        (b, b + g * ratio),
        (b + g * ratio, a + g),
        (a + g, a),
    ] {
        let q = segment_closest(p, start, end)?;
        let d = (p - q).norm();
        if d < distance {
            best = q;
            distance = d;
        }
    }
    Ok((best, condition))
}

pub(crate) fn within_boundary(
    p: Point3,
    radius: f64,
    height: f64,
    span: f64,
    drift: [f64; 2],
    budget: f64,
    coordinate_roundoff: f64,
) -> Result<bool> {
    let g = Vec3::new(drift[0] * height, drift[1] * height, height);
    let scale = radius
        .max(g.norm())
        .max(p.x.abs())
        .max(p.y.abs())
        .max(p.z.abs());
    // Guard local arithmetic/trigonometry, including reconstructed chord endpoints.
    // This is a checked binary64 error allowance, not interval arithmetic.
    let roundoff = 256. * f64::EPSILON * scale + coordinate_roundoff;
    if !p.finite() || !g.finite() || !scale.is_finite() || roundoff >= budget {
        return Err(Error::Unsupported(
            "skew boundary distance has insufficient precision",
        ));
    }
    let point = |u: f64| Point3::new(radius * u.cos(), radius * u.sin(), 0.);
    let count = (span / FRAC_PI_2).ceil() as usize;
    let mut pending: Vec<_> = (0..count)
        .map(|i| {
            (
                span * i as f64 / count as f64,
                span * (i + 1) as f64 / count as f64,
                0,
            )
        })
        .collect();
    let mut visits = 0;
    while let Some((lo, hi, depth)) = pending.pop() {
        visits += 1;
        if visits > 16384 || depth > 48 {
            return Err(Error::Unsupported(
                "skew boundary distance bounds remain unresolved",
            ));
        }
        let a = point(lo);
        let b = point(hi);
        let (q, condition) = patch_closest(p, a, b, g, 1.)?;
        let patch_roundoff = roundoff / condition;
        let distance = (p - q).norm();
        let sagitta = 2. * radius * ((hi - lo) / 4.).sin().powi(2);
        if !distance.is_finite() || !sagitta.is_finite() {
            return Err(Error::Unsupported(
                "skew boundary distance exceeds finite range",
            ));
        }
        if distance - sagitta - patch_roundoff > budget {
            continue;
        }
        // A point on the actual circle generator supplies an upper distance bound.
        let angle = (q.y - drift[1] * q.z)
            .atan2(q.x - drift[0] * q.z)
            .rem_euclid(std::f64::consts::TAU)
            .clamp(lo, hi);
        let middle = lo + (hi - lo) / 2.;
        for u in [lo, hi, middle, angle] {
            let base = point(u);
            let candidate = segment_closest(p, base, base + g)?;
            if (p - candidate).norm() + roundoff <= budget {
                return Ok(true);
            }
        }
        if middle == lo || middle == hi {
            return Err(Error::Unsupported(
                "skew boundary refinement loses angular resolution",
            ));
        }
        pending.push((lo, middle, depth + 1));
        pending.push((middle, hi, depth + 1));
    }
    Ok(false)
}

/// Euclidean band decision using ruled chord trapezoids. Corresponding points
/// differ by at most max rim second derivative * angular_width^2 / 8.
pub(crate) fn within_harmonic_boundary(
    p: Point3,
    radius: f64,
    span: f64,
    drift: [f64; 2],
    bands: [[f64; 3]; 2],
    budget: f64,
    coordinate_roundoff: f64,
) -> Result<bool> {
    let g = Vec3::new(drift[0], drift[1], 1.);
    let rim = |u: f64, index: usize| {
        Vec3::new(radius * u.cos(), radius * u.sin(), 0.)
            + g * crate::circular_trims::value(bands[index], u)
    };
    let curvature = radius + g.norm() * bands.iter().map(|b| b[1].hypot(b[2])).fold(0., f64::max);
    let scale = bands
        .iter()
        .map(|b| b[0].abs() + b[1].hypot(b[2]))
        .fold(radius, f64::max)
        * g.norm();
    let roundoff = 512. * f64::EPSILON * scale.max(p.norm()) + coordinate_roundoff;
    if !p.finite() || !curvature.is_finite() || !roundoff.is_finite() || roundoff >= budget {
        return Err(Error::Unsupported(
            "harmonic boundary distance has insufficient precision",
        ));
    }
    let count = (span / FRAC_PI_2).ceil() as usize;
    let mut pending: Vec<_> = (0..count)
        .map(|i| {
            (
                span * i as f64 / count as f64,
                span * (i + 1) as f64 / count as f64,
                0,
            )
        })
        .collect();
    let mut visits = 0;
    while let Some((lo, hi, depth)) = pending.pop() {
        visits += 1;
        if visits > 16384 || depth > 48 {
            return Err(Error::Unsupported(
                "harmonic boundary distance bounds remain unresolved",
            ));
        }
        let a = rim(lo, 0);
        let b = rim(hi, 0);
        let first = g
            * (crate::circular_trims::value(bands[1], lo)
                - crate::circular_trims::value(bands[0], lo));
        let last = g
            * (crate::circular_trims::value(bands[1], hi)
                - crate::circular_trims::value(bands[0], hi));
        let (q, condition) = patch_closest(p, a, b, first, last.norm() / first.norm())?;
        let distance = (p - q).norm();
        let error = curvature * (hi - lo).powi(2) / 8.;
        if !distance.is_finite() || !error.is_finite() {
            return Err(Error::Unsupported(
                "harmonic boundary distance exceeds finite range",
            ));
        }
        if distance - error - roundoff / condition > budget {
            continue;
        }
        let angle = (q.y - drift[1] * q.z)
            .atan2(q.x - drift[0] * q.z)
            .rem_euclid(std::f64::consts::TAU)
            .clamp(lo, hi);
        let middle = lo + (hi - lo) / 2.;
        for u in [lo, hi, middle, angle] {
            let candidate = segment_closest(p, rim(u, 0), rim(u, 1))?;
            if (p - candidate).norm() + roundoff <= budget {
                return Ok(true);
            }
        }
        if middle == lo || middle == hi {
            return Err(Error::Unsupported(
                "harmonic boundary refinement loses angular resolution",
            ));
        }
        pending.push((lo, middle, depth + 1));
        pending.push((middle, hi, depth + 1));
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn harmonic_tolerance_threshold_is_unresolved() {
        let surface = Surface::ExtrudedCircle {
            frame: Frame3::translation(Point3::new(0., 0., 0.)).unwrap(),
            radius: 2.,
            height: 4.,
            drift: [0.75, -0.5],
        };
        let budget = 1e-5;
        let point = surface.evaluate(0.37, 1.) + surface.normal(0.37) * budget;
        assert!(matches!(
            within_harmonic_boundary(
                point,
                2.,
                std::f64::consts::PI,
                [0.75, -0.5],
                [[0., 0., 0.], [2., -0.5, 0.]],
                budget,
                0.
            ),
            Err(Error::Unsupported(_))
        ));
    }
    #[test]
    fn tolerance_threshold_returns_unresolved_instead_of_rounding_to_success() {
        let surface = Surface::ExtrudedCircle {
            frame: Frame3::translation(Point3::new(0., 0., 0.)).unwrap(),
            radius: 2.,
            height: 4.,
            drift: [0.75, -0.5],
        };
        let budget = 1e-5;
        let point = surface.evaluate(0.37, 2.) + surface.normal(0.37) * budget;
        assert!(matches!(
            within_boundary(
                point,
                2.,
                4.,
                std::f64::consts::FRAC_PI_2,
                [0.75, -0.5],
                budget,
                0.
            ),
            Err(Error::Unsupported(_))
        ));
    }
}
