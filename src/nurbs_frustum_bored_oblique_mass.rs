//! Checked signed parallel-axis composition in one source-local frame.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "bored oblique mass cancellation, covariance or physical precision is unresolved",
    )
}
fn product(factors: &[f64]) -> Result<f64> {
    let mut m = 1.;
    let mut e = 0i32;
    for &x in factors {
        if !x.is_finite() || x <= 0. {
            return Err(unresolved());
        }
        let (x, adjust) = if x < f64::MIN_POSITIVE {
            (x * 4503599627370496., -52)
        } else {
            (x, 0)
        };
        let bits = x.to_bits();
        e += ((bits >> 52) & 2047) as i32 - 1023 + adjust;
        m *= f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
        if m >= 2. {
            m *= 0.5;
            e += 1;
        }
    }
    if !(-1022..=1023).contains(&e) {
        return Err(unresolved());
    }
    let v = m * f64::from_bits(((e + 1023) as u64) << 52);
    if !v.is_normal() {
        return Err(unresolved());
    }
    Ok(v)
}
struct Centered {
    volume: f64,
    centroid: Point3,
    cov: [[f64; 3]; 3],
    scale: f64,
}
fn moments(body: &NurbsBoredObliqueFrustumSolid, policy: GeometryTolerance) -> Result<Centered> {
    body.validate(policy)?;
    let outer = crate::nurbs_frustum_oblique_mass::checked_centered_moments(body.outer(), policy)?;
    let inner = crate::nurbs_frustum_oblique_mass::checked_centered_moments(body.inner(), policy)?;
    let volume = body.volume(policy)?;
    let scale = outer.scale;
    let ratio = inner.scale / scale;
    let wo = outer.volume / volume;
    let wi = inner.volume / volume;
    let kappa = 1024. * f64::EPSILON;
    let eo = wo * (outer.relative_volume_error + kappa);
    let ei = wi * (inner.relative_volume_error + kappa);
    let ev = eo + ei + kappa * (wo + wi);
    if !wo.is_finite() || !wi.is_finite() || !ev.is_finite() || ev >= 1. / 8. {
        return Err(unresolved());
    }
    let mi = inner.mean.map(|x| x * ratio);
    let emi: [f64; 3] = std::array::from_fn(|i| inner.mean_error[i] * ratio + kappa * mi[i].abs());
    let ci = inner.cov.map(|row| row.map(|x| x * ratio * ratio));
    let eci = inner.covariance_error * ratio * ratio
        + kappa * ci.iter().flatten().fold(0f64, |a, x| a.max(x.abs()));
    let mean: [f64; 3] = std::array::from_fn(|i| wo * outer.mean[i] - wi * mi[i]);
    let em: [f64; 3] = std::array::from_fn(|i| {
        2. * (wo * outer.mean_error[i]
            + wi * emi[i]
            + eo * outer.mean[i].abs()
            + ei * mi[i].abs()
            + mean[i].abs() * ev
            + kappa * (wo * outer.mean[i].abs() + wi * mi[i].abs()))
    });
    if em
        .iter()
        .any(|e| !e.is_finite() || e * scale >= policy.linear() / 8.)
    {
        return Err(unresolved());
    }
    let do_: [f64; 3] = std::array::from_fn(|i| outer.mean[i] - mean[i]);
    let di: [f64; 3] = std::array::from_fn(|i| mi[i] - mean[i]);
    let edo: [f64; 3] = std::array::from_fn(|i| outer.mean_error[i] + em[i]);
    let edi: [f64; 3] = std::array::from_fn(|i| emi[i] + em[i]);
    let cov: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            wo * (outer.cov[i][j] + do_[i] * do_[j]) - wi * (ci[i][j] + di[i] * di[j])
        })
    });
    let mut error = 0f64;
    for i in 0..3 {
        for j in 0..3 {
            let a = outer.cov[i][j] + do_[i] * do_[j];
            let b = ci[i][j] + di[i] * di[j];
            let ea = outer.covariance_error
                + do_[i].abs() * edo[j]
                + do_[j].abs() * edo[i]
                + edo[i] * edo[j];
            let eb = eci + di[i].abs() * edi[j] + di[j].abs() * edi[i] + edi[i] * edi[j];
            error = error.max(
                wo * ea
                    + wi * eb
                    + eo * a.abs()
                    + ei * b.abs()
                    + ev * cov[i][j].abs()
                    + kappa
                        * (wo * (outer.cov[i][j].abs() + (do_[i] * do_[j]).abs())
                            + wi * (ci[i][j].abs() + (di[i] * di[j]).abs())),
            );
        }
    }
    // ev < 1/8; factor two encloses denominator perturbation beyond
    // first order and the subsequent signed weighted accumulation.
    error *= 2.;
    if !error.is_finite()
        || 3. * error >= policy.linear() / (8. * scale)
        || cov.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(unresolved());
    }
    let mut lower = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let mut x = cov[i][j] - if i == j { 3. * error } else { 0. };
            for (a, b) in lower[i].iter().zip(&lower[j]).take(j) {
                x -= a * b;
            }
            if i == j {
                if !x.is_normal() || x <= 0. {
                    return Err(unresolved());
                }
                lower[i][j] = x.sqrt();
            } else {
                lower[i][j] = x / lower[j][j];
                if !lower[i][j].is_finite() {
                    return Err(unresolved());
                }
            }
        }
    }
    let centroid = body.source().source().frame().point(Point3::new(
        mean[0] * scale,
        mean[1] * scale,
        mean[2] * scale,
    ));
    let world = scale
        .max(centroid.x.abs())
        .max(centroid.y.abs())
        .max(centroid.z.abs());
    if !centroid.finite() || 32768. * f64::EPSILON * world >= policy.linear() / 8. {
        return Err(unresolved());
    }
    Ok(Centered {
        volume,
        centroid,
        cov,
        scale,
    })
}
impl NurbsBoredObliqueFrustumSolid {
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = moments(self, policy)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        let m = moments(self, policy)?;
        let trace = m.cov[0][0] + m.cov[1][1] + m.cov[2][2];
        let local: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                if i == j {
                    trace - m.cov[i][j]
                } else {
                    -m.cov[i][j]
                }
            })
        });
        let axes = self.source().source().frame().axes();
        let component = |a: Vec3, i: usize| [a.x, a.y, a.z][i];
        let coefficients: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let mut sum = 0.;
                for (a, row) in local.iter().enumerate() {
                    for (b, value) in row.iter().enumerate() {
                        sum += component(axes[a], i) * value * component(axes[b], j);
                    }
                }
                sum
            })
        });
        let mut inertia = [[0.; 3]; 3];
        for (i, row) in coefficients.iter().enumerate() {
            for (j, &coefficient) in row.iter().enumerate().skip(i) {
                let value = if coefficient == 0. {
                    0.
                } else {
                    product(&[m.volume, m.scale, m.scale, coefficient.abs()])?
                        * coefficient.signum()
                };
                inertia[i][j] = value;
                inertia[j][i] = value;
            }
        }
        Ok(NurbsGraphInertiaProperties {
            volume: m.volume,
            centroid: m.centroid,
            inertia,
        })
    }
}
