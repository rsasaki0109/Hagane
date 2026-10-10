//! Positive annular polynomial moments of the checked retained body.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported("bored frustum metric arithmetic is unresolved, overflows or underflows")
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
fn volume(body: &NurbsBoredFrustumSolid) -> Result<f64> {
    let [a, b] = body.source().radii();
    let r = body.radius();
    let scale = a.max(b);
    let d0 = (a - r) / scale;
    let d1 = (b - r) / scale;
    let coefficient =
        (d0 * (a / scale + r / scale + b / scale) + d1 * (b / scale + 2. * r / scale)) / 3.;
    product(&[
        std::f64::consts::PI,
        body.source().height(),
        scale,
        scale,
        coefficient,
    ])
}
struct Moments {
    volume: f64,
    mean: f64,
    radial: f64,
    axial: f64,
    scale: f64,
}
fn moments(body: &NurbsBoredFrustumSolid) -> Result<Moments> {
    let [r0, r1] = body.source().radii();
    let h = body.source().height();
    let scale = r0.max(r1).max(h);
    let [a, b, c] = [r0 / scale, r1 / scale, body.radius() / scale];
    let da = (r0 - body.radius()) / scale;
    let db = (r1 - body.radius()) / scale;
    let hn = h / scale;
    let area = [da * (a + c), da * b + c * db, db * (b + c)];
    let sum = area.iter().sum::<f64>();
    if area.iter().any(|v| !v.is_normal() || *v <= 0.) || !hn.is_normal() {
        return Err(unresolved());
    }
    let mean = (area[0] + 2. * area[1] + 3. * area[2]) / (4. * sum);
    let second = (area[0] / 30. + area[1] / 10. + area[2] / 5.) / (sum / 3.);
    let variance = second - mean * mean;
    if !variance.is_normal() || variance <= 8192. * f64::EPSILON * (second + mean * mean) {
        return Err(unresolved());
    }
    let mut fourth = 0.;
    for i in 0..5 {
        let factors = [
            if i < 4 { a } else { b },
            if i < 3 { a } else { b },
            if i < 2 { a } else { b },
            if i < 1 { a } else { b },
        ];
        let gaps = [
            if i < 4 { da } else { db },
            if i < 3 { da } else { db },
            if i < 2 { da } else { db },
            if i < 1 { da } else { db },
        ];
        let mut coefficient = 0.;
        for k in 0..4 {
            coefficient += gaps[k] * c.powi(k as i32) * factors[k + 1..].iter().product::<f64>();
        }
        if !coefficient.is_normal() || coefficient <= 0. {
            return Err(unresolved());
        }
        fourth += coefficient;
    }
    let radial = 3. * fourth / (20. * sum);
    let axial = hn * hn * variance;
    if !radial.is_normal() || !axial.is_normal() {
        return Err(unresolved());
    }
    let volume = volume(body)?;
    Ok(Moments {
        volume,
        mean,
        radial,
        axial,
        scale,
    })
}
impl NurbsBoredFrustumSolid {
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        volume(self)
    }
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(policy)?;
        let m = moments(self)?;
        let centroid =
            self.source()
                .frame()
                .point(Point3::new(0., 0., self.source().height() * m.mean));
        let world = m
            .scale
            .max(centroid.x.abs())
            .max(centroid.y.abs())
            .max(centroid.z.abs());
        if !centroid.finite() || 65536. * f64::EPSILON * world >= policy.linear() / 8. {
            return Err(unresolved());
        }
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid,
        })
    }
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        let mass = self.mass_properties(policy)?;
        let m = moments(self)?;
        let transverse = product(&[m.volume, m.scale, m.scale, m.radial + m.axial])?;
        let longitudinal = product(&[m.volume, m.scale, m.scale, 2. * m.radial])?;
        let axes = self.source().frame().axes();
        let rows = [
            [axes[0].x, axes[1].x, axes[2].x],
            [axes[0].y, axes[1].y, axes[2].y],
            [axes[0].z, axes[1].z, axes[2].z],
        ];
        let tensor_scale = transverse.max(longitudinal);
        let diagonal = [
            transverse / tensor_scale,
            transverse / tensor_scale,
            longitudinal / tensor_scale,
        ];
        let mut inertia = [[0.; 3]; 3];
        for i in 0..3 {
            for j in i..3 {
                let coefficient = (0..3)
                    .map(|k| rows[i][k] * diagonal[k] * rows[j][k])
                    .sum::<f64>();
                let value = coefficient * tensor_scale;
                if !value.is_finite() || (coefficient != 0. && !value.is_normal()) {
                    return Err(unresolved());
                }
                inertia[i][j] = value;
                inertia[j][i] = value;
            }
        }
        Ok(NurbsGraphInertiaProperties {
            volume: mass.volume,
            centroid: mass.centroid,
            inertia,
        })
    }
}
