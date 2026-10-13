//! Analytic disk-segment moments of completely certified horizontal torus children.
//! Local normalized centered moments precede the world tensor rotation; arithmetic
//! reserves are engineering checks rather than formal interval certificates.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "horizontal torus mass, centroid or centered inertia precision is unresolved",
    )
}
#[derive(Clone, Copy)]
struct Estimate {
    value: f64,
    error: f64,
}
fn quotient(a: Estimate, b: Estimate) -> Result<Estimate> {
    if !b.value.is_finite() || b.value <= b.error || b.error < 0. {
        return Err(unresolved());
    }
    let value = a.value / b.value;
    let error =
        (a.error + value.abs() * b.error) / (b.value - b.error) + 32. * f64::EPSILON * value.abs();
    if !value.is_finite() || !error.is_finite() {
        return Err(unresolved());
    }
    Ok(Estimate { value, error })
}
fn j(k: usize, q2: f64) -> Estimate {
    let mut coefficient = 1.;
    let mut value = 1. / (3 + 2 * k) as f64;
    let mut operands = value;
    for n in 1..=16 {
        coefficient *= ((n as f64 - 1.5) / n as f64) * q2;
        let term = coefficient / (3 + 2 * k + 2 * n) as f64;
        value += term;
        operands += term.abs();
    }
    let next = coefficient * ((17. - 1.5) / 17.) * q2 / (3 + 2 * k + 34) as f64;
    Estimate {
        value,
        error: 8192. * f64::EPSILON * operands + next.abs() / (1. - q2),
    }
}
struct Moments {
    volume: f64,
    centroid: Point3,
    radial: f64,
    axial: f64,
    scale: f64,
}
fn moments(body: &NurbsTorusHorizontalSolid, p: GeometryTolerance) -> Result<Moments> {
    body.validate(p)?;
    let [major, minor] = body.source().radii();
    let cut = body.cut_height();
    let sign = if body.is_lower() { -1. } else { 1. };
    // Form the physical tiny-cap height before normalizing; independently rounded
    // ratios must not erase a resolved tip distance.
    let q2 = if body.is_lower() {
        (minor + cut) / minor / 2.
    } else {
        (minor - cut) / minor / 2.
    };
    let (mean, radial, variance) = if q2 <= 1. / 16. {
        let a = j(0, q2);
        let first = quotient(j(1, q2), a)?;
        let second = quotient(j(2, q2), a)?;
        let mean = Estimate {
            value: sign * (1. - 2. * q2 * first.value),
            error: 2. * q2 * first.error + 128. * f64::EPSILON,
        };
        let difference = second.value - first.value * first.value;
        let difference_error = second.error
            + 2. * first.value.abs() * first.error
            + first.error * first.error
            + 128. * f64::EPSILON * (second.value.abs() + first.value * first.value);
        let radial = Estimate {
            value: (4. / 3.) * q2 * (first.value - q2 * second.value),
            error: (4. / 3.) * q2 * (first.error + q2 * second.error) + 256. * f64::EPSILON * q2,
        };
        let variance = Estimate {
            value: 4. * q2 * q2 * difference,
            error: 4. * q2 * q2 * difference_error,
        };
        (mean, radial, variance)
    } else {
        let s = cut / minor;
        let sine = ((1. - s) * (1. + s)).sqrt();
        let cosine = if body.is_lower() { -s } else { s };
        let theta = sine.atan2(cosine);
        let area = Estimate {
            value: theta - sine * cosine,
            error: 8192. * f64::EPSILON * (theta.abs() + (sine * cosine).abs()),
        };
        let mean = quotient(
            Estimate {
                value: sign * (2. / 3.) * sine * sine * sine,
                // Input/root perturbations in a nearly complete cap have
                // absolute O(eps) first-moment error, not merely eps*sin³.
                error: 8192. * f64::EPSILON * (1. + (2. / 3.) * sine * sine * sine),
            },
            area,
        )?;
        let sin2 = (2. * theta).sin();
        let sin4 = (4. * theta).sin();
        let radial = quotient(
            Estimate {
                value: theta / 4. - sin2 / 6. + sin4 / 48.,
                error: 8192. * f64::EPSILON * (theta / 4. + sin2.abs() / 6. + sin4.abs() / 48.),
            },
            area,
        )?;
        let raw = quotient(
            Estimate {
                value: theta / 4. - sin4 / 16.,
                error: 8192. * f64::EPSILON * (theta / 4. + sin4.abs() / 16.),
            },
            area,
        )?;
        let variance = Estimate {
            value: raw.value - mean.value * mean.value,
            error: raw.error
                + 2. * mean.value.abs() * mean.error
                + mean.error * mean.error
                + 128. * f64::EPSILON * (raw.value.abs() + mean.value * mean.value),
        };
        (mean, radial, variance)
    };
    if !q2.is_finite()
        || q2 <= 0.
        || q2 >= 1.
        || radial.value <= 8. * radial.error
        || variance.value <= 8. * variance.error
    {
        return Err(unresolved());
    }
    let ratio = minor / major;
    let ratio2 = ratio * ratio;
    let radial_cov = 0.5 + 1.5 * ratio2 * radial.value;
    let axial_cov = ratio2 * variance.value;
    let cov_error = ratio2 * (1.5 * radial.error + variance.error)
        + 8192. * f64::EPSILON * (radial_cov + axial_cov);
    if !radial_cov.is_finite()
        || !axial_cov.is_finite()
        || axial_cov <= 0.
        || !cov_error.is_finite()
        || cov_error >= p.linear() / major / 8.
    {
        return Err(unresolved());
    }
    let frame = body.source().frame();
    let z = minor * mean.value;
    let centroid = frame.point(Point3::new(0., 0., z));
    let world = [frame.origin(), centroid]
        .into_iter()
        .fold(major + minor, |s, p| {
            s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs())
        });
    let error = minor * mean.error + 32768. * f64::EPSILON * world;
    if !centroid.finite() || !error.is_finite() || error >= p.linear() / 8. {
        return Err(unresolved());
    }
    Ok(Moments {
        volume: body.volume(p)?,
        centroid,
        radial: radial_cov,
        axial: axial_cov,
        scale: major,
    })
}
impl NurbsTorusHorizontalSolid {
    pub fn mass_properties(&self, p: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = moments(self, p)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    pub fn inertia_properties(&self, p: GeometryTolerance) -> Result<NurbsGraphInertiaProperties> {
        let m = moments(self, p)?;
        let transverse = product(&[m.volume, m.scale, m.scale, m.radial + m.axial])?;
        let axial = product(&[m.volume, m.scale, m.scale, 2. * m.radial])?;
        let axes = self.source().frame().axes();
        let rows = [
            [axes[0].x, axes[1].x, axes[2].x],
            [axes[0].y, axes[1].y, axes[2].y],
            [axes[0].z, axes[1].z, axes[2].z],
        ];
        let scale = transverse.max(axial);
        let d = [transverse / scale, transverse / scale, axial / scale];
        let mut inertia = [[0.; 3]; 3];
        for i in 0..3 {
            for k in i..3 {
                let coefficient = (0..3).map(|j| rows[i][j] * d[j] * rows[k][j]).sum::<f64>();
                let value = coefficient * scale;
                if !value.is_finite() || (coefficient != 0. && value == 0.) {
                    return Err(unresolved());
                }
                inertia[i][k] = value;
                inertia[k][i] = value;
            }
        }
        Ok(NurbsGraphInertiaProperties {
            volume: m.volume,
            centroid: m.centroid,
            inertia,
        })
    }
}

fn product(factors: &[f64]) -> Result<f64> {
    let mut mantissa = 1.;
    let mut exponent = 0i32;
    for &value in factors {
        if !value.is_finite() || value <= 0. {
            return Err(unresolved());
        }
        let (value, adjustment) = if value < f64::MIN_POSITIVE {
            (value * 4503599627370496., -52)
        } else {
            (value, 0)
        };
        let bits = value.to_bits();
        exponent += ((bits >> 52) & 2047) as i32 - 1023 + adjustment;
        mantissa *= f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
        if mantissa >= 2. {
            mantissa *= 0.5;
            exponent += 1;
        }
    }
    if !(-1022..=1023).contains(&exponent) {
        return Err(unresolved());
    }
    let result = mantissa * f64::from_bits(((exponent + 1023) as u64) << 52);
    if !result.is_finite() || result < f64::MIN_POSITIVE {
        return Err(unresolved());
    }
    Ok(result)
}
