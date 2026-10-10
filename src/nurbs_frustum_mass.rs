//! Analytic uniform-density metrics of the completely validated NURBS frustum.
use crate::*;

fn unresolved() -> Error {
    Error::Unsupported("frustum metric arithmetic overflows, underflows, or is unresolved")
}
// Accumulate positive factors in binary mantissa/exponent form. A finite final
// result need not have finite intermediate products (e.g. a tall thin solid).
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
struct Moments {
    volume: f64,
    centroid_fraction: f64,
    radial_fourth: f64,
    axial_second: f64,
    radial_second: f64,
    radius_scale: f64,
}
fn moments(radii: [f64; 2], height: f64) -> Result<Moments> {
    let radius_scale = radii[0].max(radii[1]);
    let [a, b] = radii.map(|r| r / radius_scale);
    let sum = a * a + a * b + b * b;
    let radial_second = sum / 3.;
    let centroid_fraction = (a * a + 2. * a * b + 3. * b * b) / (4. * sum);
    let volume = product(&[
        std::f64::consts::PI,
        height,
        radius_scale,
        radius_scale,
        radial_second,
    ])?;
    // Positive three-point Gauss-Legendre quadrature is exact in real
    // arithmetic through degree five: q^4 and q^2(t-c)^2 have degree four.
    let offset = (3f64 / 5.).sqrt() / 2.;
    let mut radial_fourth = 0.;
    let mut axial_second = 0.;
    for (t, weight) in [
        (0.5 - offset, 5. / 18.),
        (0.5, 4. / 9.),
        (0.5 + offset, 5. / 18.),
    ] {
        let q = a * (1. - t) + b * t;
        let square = q * q;
        radial_fourth += weight * square * square;
        axial_second += weight * square * (t - centroid_fraction).powi(2);
    }
    if !centroid_fraction.is_finite()
        || !radial_fourth.is_finite()
        || !axial_second.is_finite()
        || radial_second <= 0.
        || radial_fourth <= 0.
        || axial_second <= 0.
    {
        return Err(unresolved());
    }
    Ok(Moments {
        volume,
        centroid_fraction,
        radial_fourth,
        axial_second,
        radial_second,
        radius_scale,
    })
}
impl NurbsFrustumSolid {
    /// Uniform-density analytic volume; the actual retained B-rep is checked first.
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        Ok(moments(self.radii(), self.height())?.volume)
    }
    /// Uniform-density mass and world centroid, derived in the local frame.
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(policy)?;
        let m = moments(self.radii(), self.height())?;
        let centroid = self
            .frame()
            .point(Vec3::new(0., 0., self.height() * m.centroid_fraction));
        if !centroid.finite() {
            return Err(unresolved());
        }
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid,
        })
    }
    /// Centroidal inertia tensor in world axes, for unit density.
    pub fn inertia_properties(
        &self,
        policy: GeometryTolerance,
    ) -> Result<NurbsGraphInertiaProperties> {
        self.validate(policy)?;
        let m = moments(self.radii(), self.height())?;
        let scale = m.radius_scale.max(self.height());
        let radial = (m.radius_scale / scale).powi(2) * m.radial_fourth / m.radial_second;
        let axial = (self.height() / scale).powi(2) * m.axial_second / m.radial_second;
        let transverse = product(&[m.volume, scale, scale, radial / 4. + axial])?;
        let longitudinal = product(&[m.volume, scale, scale, radial / 2.])?;
        let axes = self.frame().axes();
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
                if !value.is_finite() || (coefficient != 0. && value == 0.) {
                    return Err(unresolved());
                }
                inertia[i][j] = value;
                inertia[j][i] = value;
            }
        }
        let centroid = self
            .frame()
            .point(Vec3::new(0., 0., self.height() * m.centroid_fraction));
        if !centroid.finite() {
            return Err(unresolved());
        }
        Ok(NurbsGraphInertiaProperties {
            volume: m.volume,
            centroid,
            inertia,
        })
    }
    /// Exact-real rigid AABB: every coordinate extremum occurs on an end circle.
    /// Binary64 evaluation uses the validated frame's engineering precision budget.
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        let frame = self.frame();
        let axes = frame.axes();
        let centers = [
            frame.origin(),
            frame.point(Vec3::new(0., 0., self.height())),
        ];
        let radii = self.radii();
        let mut lower = [f64::INFINITY; 3];
        let mut upper = [f64::NEG_INFINITY; 3];
        for (center, radius) in centers.into_iter().zip(radii) {
            let values = [center.x, center.y, center.z];
            let extent = [
                axes[0].x.hypot(axes[1].x),
                axes[0].y.hypot(axes[1].y),
                axes[0].z.hypot(axes[1].z),
            ]
            .map(|v| v * radius);
            for i in 0..3 {
                lower[i] = lower[i].min(values[i] - extent[i]);
                upper[i] = upper[i].max(values[i] + extent[i]);
            }
        }
        let result = Bounds {
            min: Point3::new(lower[0], lower[1], lower[2]),
            max: Point3::new(upper[0], upper[1], upper[2]),
        };
        if !result.min.finite() || !result.max.finite() {
            return Err(unresolved());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaled_products_avoid_intermediate_overflow_and_reject_final_loss() {
        let value = product(&[1e200, 1e200, 1e-200]).unwrap();
        assert!((value / 1e200 - 1.).abs() < 1e-14);
        assert!(product(&[1e200, 1e200]).is_err());
        assert!(product(&[1e-200, 1e-200]).is_err());
    }
    #[test]
    fn independent_cylinder_and_asymmetric_centroid_formulas() {
        let m = moments([2., 2.], 5.).unwrap();
        assert!((m.volume - 20. * std::f64::consts::PI).abs() < 1e-13);
        assert!((m.centroid_fraction - 0.5).abs() < 1e-15);
        assert!((m.radial_fourth - 1.).abs() < 1e-15);
        assert!((m.axial_second - 1. / 12.).abs() < 1e-15);
        let forward = moments([1., 2.], 5.).unwrap();
        let reverse = moments([2., 1.], 5.).unwrap();
        assert!((forward.centroid_fraction - 17. / 28.).abs() < 1e-15);
        assert!((forward.centroid_fraction + reverse.centroid_fraction - 1.).abs() < 1e-15);
        assert!((forward.volume - 35. * std::f64::consts::PI / 3.).abs() < 1e-13);
    }
    #[test]
    fn validated_actual_cylinder_metrics_and_mutation_rejection() {
        let policy = GeometryTolerance::new(1e-6, 1e-10, 0.).unwrap();
        let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [2., 2.], 5., policy).unwrap();
        let mass = body.mass_properties(policy).unwrap();
        assert!((mass.volume - 20. * std::f64::consts::PI).abs() < 1e-13);
        assert_eq!(mass.centroid, Point3::new(0., 0., 2.5));
        let inertia = body.inertia_properties(policy).unwrap();
        assert!((inertia.inertia[0][0] / mass.volume - (1. + 25. / 12.)).abs() < 1e-14);
        assert!((inertia.inertia[2][2] / mass.volume - 2.).abs() < 1e-14);
        assert_eq!(
            body.bounds(policy).unwrap(),
            Bounds {
                min: Point3::new(-2., -2., 0.),
                max: Point3::new(2., 2., 5.)
            }
        );
        let mut changed = body.solid().clone();
        changed.shell.faces[2].orientation *= -1;
        assert!(
            NurbsFrustumSolid::from_brep(changed, Frame3::IDENTITY, [2., 2.], 5., policy).is_err()
        );
    }
}
