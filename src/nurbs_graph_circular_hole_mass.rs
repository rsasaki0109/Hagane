//! Polynomial column moments for rectangular graph stock minus a circular bore.
//! Gauss4 on the rectangle and Gauss4 radial/16 angular disk sampling integrate
//! every degree-twelve column polynomial in real arithmetic. Each region has
//! positive weights; subtracting regions requires a checked cancellation margin.
//! Binary64 allowances are engineering checks, not interval certificates.
use crate::*;
#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
    envelope: f64,
}
impl Sum {
    fn add(&mut self, x: f64) {
        let next = self.value + x;
        self.correction += if self.value.abs() >= x.abs() {
            (self.value - next) + x
        } else {
            (x - next) + self.value
        };
        self.value = next;
        self.envelope += x.abs();
    }
    fn total(&self) -> f64 {
        self.value + self.correction
    }
}
fn unresolved() -> Error {
    Error::InvalidInput("circular graph moments overflow, underflow or lose precision")
}
#[derive(Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
    height: f64,
    weight: f64,
}
struct Integration {
    stock: Vec<Sample>,
    disk: Vec<Sample>,
    scale: f64,
    origin: [f64; 2],
}
impl Integration {
    fn new(body: &NurbsGraphCircularHoledSolid, tol: Tolerance) -> Result<Self> {
        body.validate(tol)?;
        let source = body.source();
        let [l, w, h] = source.dimensions();
        let b = source.bulge();
        let [u, v] = source.source_domain();
        let local = NurbsGraphSolid::new(source.dimensions(), b, tol)?
            .trimmed_uv(source.source_domain(), tol)?;
        let scale = (l * (u[1] - u[0]))
            .max(w * (v[1] - v[0]))
            .max(local.bounds()?.max.z);
        if !scale.is_finite() || scale < f64::MIN_POSITIVE {
            return Err(unresolved());
        }
        let middle = [u[0] + (u[1] - u[0]) / 2., v[0] + (v[1] - v[0]) / 2.];
        let origin = [l * middle[0], w * middle[1]];
        let [cx, cy] = body.center();
        let r = body.radius();
        let nodes = [
            -0.8611363115940526,
            -0.3399810435848563,
            0.3399810435848563,
            0.8611363115940526,
        ];
        let weights = [
            0.1739274225687269,
            0.3260725774312731,
            0.3260725774312731,
            0.1739274225687269,
        ];
        let area = (l / scale * (u[1] - u[0])) * (w / scale * (v[1] - v[0]));
        let disk_area = std::f64::consts::PI * (r / scale) * (r / scale);
        if !area.is_finite()
            || !disk_area.is_finite()
            || area < f64::MIN_POSITIVE
            || disk_area < f64::MIN_POSITIVE
        {
            return Err(unresolved());
        }
        let sample = |uv: [f64; 2], x: f64, y: f64, area_weight: f64| -> Result<Sample> {
            let height = h / scale + 4. * (b / scale) * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1]);
            let weight = area_weight * height;
            if [x, y, height, weight].iter().any(|v| !v.is_finite())
                || height < f64::MIN_POSITIVE
                || weight < f64::MIN_POSITIVE
            {
                return Err(unresolved());
            }
            Ok(Sample {
                x,
                y,
                height,
                weight,
            })
        };
        let mut stock = Vec::with_capacity(16);
        let mut disk = Vec::with_capacity(64);
        for i in 0..4 {
            for j in 0..4 {
                let du = (u[1] - u[0]) * nodes[i] / 2.;
                let dv = (v[1] - v[0]) * nodes[j] / 2.;
                stock.push(sample(
                    [middle[0] + du, middle[1] + dv],
                    l / scale * du,
                    w / scale * dv,
                    area * weights[i] * weights[j],
                )?);
            }
        }
        for i in 0..4 {
            let rho = r * ((nodes[i] + 1.) / 2.).sqrt();
            for j in 0..4 {
                let angle = std::f64::consts::TAU * (j as f64 + 0.5) / 16.;
                let x = rho * angle.cos();
                let y = rho * angle.sin();
                for [dx, dy] in [[x, y], [-y, x], [-x, -y], [y, -x]] {
                    disk.push(sample(
                        [(cx + dx) / l, (cy + dy) / w],
                        (cx - origin[0]) / scale + dx / scale,
                        (cy - origin[1]) / scale + dy / scale,
                        disk_area * weights[i] / 16.,
                    )?);
                }
            }
        }
        Ok(Self {
            stock,
            disk,
            scale,
            origin,
        })
    }
    fn moments<const N: usize>(
        &self,
        values: impl Fn(Sample) -> [f64; N],
    ) -> Result<([f64; N], [f64; N])> {
        let mut sums: [Sum; N] = std::array::from_fn(|_| Sum::default());
        for (samples, sign) in [(&self.stock, 1.), (&self.disk, -1.)] {
            for &sample in samples {
                for (sum, value) in sums.iter_mut().zip(values(sample)) {
                    let term = sample.weight * value;
                    if !term.is_finite() || (value != 0. && term.abs() < f64::MIN_POSITIVE) {
                        return Err(unresolved());
                    }
                    sum.add(sign * term);
                }
            }
        }
        let value = std::array::from_fn(|i| sums[i].total());
        let error = std::array::from_fn(|i| 8192. * f64::EPSILON * sums[i].envelope);
        if value.iter().chain(error.iter()).any(|v| !v.is_finite()) {
            return Err(unresolved());
        }
        Ok((value, error))
    }
    fn first(&self) -> Result<([f64; 4], [f64; 3])> {
        let (m, error) = self.moments(|p| [1., p.x, p.y, p.height / 2.])?;
        if m[0] <= error[0] || m[0] < f64::MIN_POSITIVE {
            return Err(unresolved());
        }
        let center = [m[1] / m[0], m[2] / m[0], m[3] / m[0]];
        if center.iter().any(|v| !v.is_finite()) {
            return Err(unresolved());
        }
        Ok((m, center))
    }
    fn physical_mass(
        &self,
        body: &NurbsGraphCircularHoledSolid,
        mass: f64,
        center: [f64; 3],
    ) -> Result<NurbsGraphMassProperties> {
        let mut volume = mass;
        for _ in 0..3 {
            volume *= self.scale;
            if !volume.is_finite() || volume < f64::MIN_POSITIVE {
                return Err(unresolved());
            }
        }
        let local = Point3::new(
            self.origin[0] + self.scale * center[0],
            self.origin[1] + self.scale * center[1],
            self.scale * center[2],
        );
        let centroid = body.source().placement().point(local);
        if !local.finite() || !centroid.finite() {
            return Err(unresolved());
        }
        Ok(NurbsGraphMassProperties { volume, centroid })
    }
}
impl NurbsGraphCircularHoledSolid {
    /// Positive bore volume, evaluated directly without subtracting stock volumes.
    pub fn removed_volume(&self, tol: Tolerance) -> Result<f64> {
        let integration = Integration::new(self, tol)?;
        let mut sum = Sum::default();
        for sample in &integration.disk {
            sum.add(sample.weight);
        }
        let mut volume = sum.total();
        if !volume.is_finite() || volume <= 8192. * f64::EPSILON * sum.envelope {
            return Err(unresolved());
        }
        for _ in 0..3 {
            volume *= integration.scale;
            if !volume.is_finite() || volume < f64::MIN_POSITIVE {
                return Err(unresolved());
            }
        }
        Ok(volume)
    }
    pub fn volume(&self) -> Result<f64> {
        Ok(self.mass_properties(self.construction_tolerance())?.volume)
    }
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        let integration = Integration::new(self, tol)?;
        let (m, c) = integration.first()?;
        integration.physical_mass(self, m[0], c)
    }
    /// Uniform-density centroidal tensor in world axes, in length to the fifth.
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        let integration = Integration::new(self, tol)?;
        let (mass, center) = integration.first()?;
        let actual = integration.physical_mass(self, mass[0], center)?;
        let (m, error) = integration.moments(|p| {
            let x = p.x - center[0];
            let y = p.y - center[1];
            let z = p.height / 2. - center[2];
            [
                x * x,
                y * y,
                z * z + p.height * p.height / 12.,
                x * y,
                x * z,
                y * z,
            ]
        })?;
        if (0..3).any(|i| m[i] <= error[i]) {
            return Err(unresolved());
        }
        let mut tensor = [[0.; 3]; 3];
        tensor[0][0] = (m[1] + m[2]) / mass[0];
        tensor[1][1] = (m[0] + m[2]) / mass[0];
        tensor[2][2] = (m[0] + m[1]) / mass[0];
        tensor[0][1] = -m[3] / mass[0];
        tensor[0][2] = -m[4] / mass[0];
        tensor[1][2] = -m[5] / mass[0];
        tensor[1][0] = tensor[0][1];
        tensor[2][0] = tensor[0][2];
        tensor[2][1] = tensor[1][2];
        let axes = self.source().placement().axes();
        let rotation: [[f64; 3]; 3] =
            std::array::from_fn(|i| std::array::from_fn(|j| [axes[j].x, axes[j].y, axes[j].z][i]));
        let mut world = [[0.; 3]; 3];
        for i in 0..3 {
            for j in i..3 {
                let mut sum = Sum::default();
                for a in 0..3 {
                    for b in 0..3 {
                        sum.add(rotation[i][a] * tensor[a][b] * rotation[j][b]);
                    }
                }
                let mut value = sum.total();
                if !value.is_finite() || (i == j && value <= 8192. * f64::EPSILON * sum.envelope) {
                    return Err(unresolved());
                }
                for factor in [integration.scale, integration.scale, actual.volume] {
                    let next = value * factor;
                    if !next.is_finite() || (value != 0. && next.abs() < f64::MIN_POSITIVE) {
                        return Err(unresolved());
                    }
                    value = next;
                }
                world[i][j] = value;
                world[j][i] = value;
            }
        }
        Ok(NurbsGraphInertiaProperties {
            volume: actual.volume,
            centroid: actual.centroid,
            inertia: world,
        })
    }
}
