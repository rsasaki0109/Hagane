//! Checked uniform-density torus metrics in the retained frame.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported("torus metric arithmetic overflows, underflows, or is unresolved")
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
impl NurbsTorusSolid {
    pub fn mass_properties(&self, p: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(p)?;
        Ok(NurbsGraphMassProperties {
            volume: self.volume(p)?,
            centroid: self.frame().origin(),
        })
    }
    pub fn inertia_properties(&self, p: GeometryTolerance) -> Result<NurbsGraphInertiaProperties> {
        self.validate(p)?;
        let volume = self.volume(p)?;
        let [r, s] = self.radii();
        let q = s / r;
        let transverse = product(&[volume, r, r, 0.5 + 0.625 * q * q])?;
        let longitudinal = product(&[volume, r, r, 1. + 0.75 * q * q])?;
        let axes = self.frame().axes();
        let rows = [
            [axes[0].x, axes[1].x, axes[2].x],
            [axes[0].y, axes[1].y, axes[2].y],
            [axes[0].z, axes[1].z, axes[2].z],
        ];
        let scale = longitudinal.max(transverse);
        let d = [transverse / scale, transverse / scale, longitudinal / scale];
        let mut inertia = [[0.; 3]; 3];
        for i in 0..3 {
            for j in i..3 {
                let c = (0..3).map(|k| rows[i][k] * d[k] * rows[j][k]).sum::<f64>();
                let v = c * scale;
                if !v.is_finite() || (c != 0. && v == 0.) {
                    return Err(unresolved());
                }
                inertia[i][j] = v;
                inertia[j][i] = v;
            }
        }
        Ok(NurbsGraphInertiaProperties {
            volume,
            centroid: self.frame().origin(),
            inertia,
        })
    }
    /// Positive-weight rational control hull, conservatively enclosing the body.
    pub fn bounds(&self, p: GeometryTolerance) -> Result<Bounds> {
        self.validate(p)?;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for face in &self.solid().shell.faces {
            if let Surface::Nurbs(s) = &face.surface {
                for point in s.control_points() {
                    for (i, v) in [point.x, point.y, point.z].into_iter().enumerate() {
                        lo[i] = lo[i].min(v);
                        hi[i] = hi[i].max(v);
                    }
                }
            }
        }
        Ok(Bounds {
            min: Point3::new(lo[0], lo[1], lo[2]),
            max: Point3::new(hi[0], hi[1], hi[2]),
        })
    }
}
