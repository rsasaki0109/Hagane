//! Analytic local cone/disk moments of certified oblique frustum pieces.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "oblique frustum mass moments overflow, underflow or have unresolved cancellation",
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
struct Raw {
    v: f64,
    m: [f64; 3],
    q: [[f64; 3]; 3],
    ev: f64,
    em: [f64; 3],
    eq: [[f64; 3]; 3],
}
fn cone(apex: [f64; 3], center: [f64; 3], cov: [[f64; 3]; 3], v: f64, kappa: f64) -> Result<Raw> {
    let d: [f64; 3] = std::array::from_fn(|i| center[i] - apex[i]);
    let m = std::array::from_fn(|i| v * (apex[i] + 0.75 * d[i]));
    let em = std::array::from_fn(|i| kappa * v * (apex[i].abs() + 0.75 * d[i].abs()));
    let q = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            v * (apex[i] * apex[j]
                + 0.75 * (apex[i] * d[j] + d[i] * apex[j])
                + 0.6 * (d[i] * d[j] + cov[i][j]))
        })
    });
    let eq = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            kappa
                * v
                * ((apex[i] * apex[j]).abs()
                    + 0.75 * ((apex[i] * d[j]).abs() + (d[i] * apex[j]).abs())
                    + 0.6 * ((d[i] * d[j]).abs() + cov[i][j].abs()))
        })
    });
    if !v.is_normal()
        || m.iter()
            .chain(q.iter().flatten())
            .chain(em.iter())
            .chain(eq.iter().flatten())
            .any(|x| !x.is_finite())
    {
        return Err(unresolved());
    }
    Ok(Raw {
        v,
        m,
        q,
        ev: kappa * v,
        em,
        eq,
    })
}
fn disk_raw(radius: f64, depth: f64, a: f64, b: f64, offset: f64, reflection: f64) -> Result<Raw> {
    let area = std::f64::consts::PI * radius * radius;
    let r2 = radius * radius;
    let l2 = a * a + b * b;
    let v = area * depth;
    let mut m = [
        -a * area * r2 / 4.,
        -b * area * r2 / 4.,
        area * (depth * depth + l2 * r2 / 4.) / 2.,
    ];
    let mut q = [
        [depth * area * r2 / 4., 0., -depth * a * area * r2 / 4.],
        [0., depth * area * r2 / 4., -depth * b * area * r2 / 4.],
        [
            -depth * a * area * r2 / 4.,
            -depth * b * area * r2 / 4.,
            area * (depth * depth * depth / 3. + depth * l2 * r2 / 4.),
        ],
    ];
    let kz = 1024. * f64::EPSILON;
    let mut em = m.map(|x| kz * x.abs());
    let mut eq = q.map(|row| row.map(|x| kz * x.abs()));
    let mz = m[2];
    let qzz = q[2][2];
    m[2] = offset * v + reflection * mz;
    em[2] += kz * (offset * v).abs();
    q[2][2] = offset * offset * v + 2. * offset * reflection * mz + qzz;
    eq[2][2] += kz * ((offset * offset * v).abs() + 2. * (offset * mz).abs());
    for i in 0..2 {
        q[i][2] = offset * m[i] + reflection * q[i][2];
        q[2][i] = q[i][2];
        eq[i][2] += kz * (offset * m[i]).abs();
        eq[2][i] = eq[i][2];
    }
    if !v.is_normal() || m.iter().chain(q.iter().flatten()).any(|x| !x.is_finite()) {
        return Err(unresolved());
    }
    Ok(Raw {
        v,
        m,
        q,
        ev: kz * v,
        em,
        eq,
    })
}
struct Centered {
    volume: f64,
    centroid: Point3,
    cov: [[f64; 3]; 3],
    scale: f64,
}
fn moments(body: &NurbsObliqueFrustumSolid, policy: GeometryTolerance) -> Result<Centered> {
    body.validate(policy)?;
    let source = body.source();
    let [r0, r1] = source.radii();
    let h = source.height();
    let scale = r0.max(r1).max(h);
    let r = r0 / scale;
    let top = r1 / scale;
    let height = h / scale;
    let Surface::Plane { origin, u, v } = *body.section().plane() else {
        return Err(unresolved());
    };
    let normal = u.cross(v).normalized()?;
    let n = source.frame().local_vector(normal);
    let c = normal.dot(origin - source.frame().origin()) / (n.z * scale);
    let a = n.x / n.z;
    let b = n.y / n.z;
    let slope = (r1 - r0) / h;
    let l2 = a * a + b * b;
    let raw = if r0 == r1 {
        if body.is_lower() {
            disk_raw(r, c, a, b, 0., 1.)?
        } else {
            disk_raw(r, height - c, -a, -b, height, -1.)?
        }
    } else {
        let g = 1. - slope * slope * l2;
        let q = g.sqrt();
        let rr = r + slope * c;
        let factor = rr * rr / (4. * g);
        let covxx = factor * (1. + slope * slope * a * a / g);
        let covyy = factor * (1. + slope * slope * b * b / g);
        let covxy = factor * slope * slope * a * b / g;
        let covxz = -a * covxx - b * covxy;
        let covyz = -a * covxy - b * covyy;
        let covzz = a * a * covxx + 2. * a * b * covxy + b * b * covyy;
        let cov = [
            [covxx, covxy, covxz],
            [covxy, covyy, covyz],
            [covxz, covyz, covzz],
        ];
        let center = [-rr * slope * a / g, -rr * slope * b / g, 0.];
        let center = [center[0], center[1], c - a * center[0] - b * center[1]];
        let apex = [0., 0., -r / slope];
        let kappa = 1024. * f64::EPSILON * (1. + 1. / g);
        if !q.is_normal() || !rr.is_normal() || !kappa.is_finite() {
            return Err(unresolved());
        }
        let cut = cone(
            apex,
            center,
            cov,
            std::f64::consts::PI * rr * rr * rr / (3. * slope.abs() * q * q * q),
            kappa,
        )?;
        let (first, last) = if body.is_lower() {
            let base = cone(
                apex,
                [0.; 3],
                [[r * r / 4., 0., 0.], [0., r * r / 4., 0.], [0., 0., 0.]],
                std::f64::consts::PI * r * r * r / (3. * slope.abs()),
                kappa,
            )?;
            (base, cut)
        } else {
            let upper = cone(
                apex,
                [0., 0., height],
                [
                    [top * top / 4., 0., 0.],
                    [0., top * top / 4., 0.],
                    [0., 0., 0.],
                ],
                std::f64::consts::PI * top * top * top / (3. * slope.abs()),
                kappa,
            )?;
            (cut, upper)
        };
        let sign = slope.signum();
        Raw {
            v: sign * (last.v - first.v),
            m: std::array::from_fn(|i| sign * (last.m[i] - first.m[i])),
            q: std::array::from_fn(|i| {
                std::array::from_fn(|j| sign * (last.q[i][j] - first.q[i][j]))
            }),
            ev: last.ev + first.ev,
            em: std::array::from_fn(|i| last.em[i] + first.em[i]),
            eq: std::array::from_fn(|i| std::array::from_fn(|j| last.eq[i][j] + first.eq[i][j])),
        }
    };
    let volume = body.volume(policy)?;
    let source_volume = source.volume(policy)?;
    let stable = (std::f64::consts::PI / 3.)
        * height
        * (r * r + r * top + top * top)
        * (volume / source_volume);
    if !stable.is_normal()
        || !raw.v.is_normal()
        || raw.v <= raw.ev
        || (raw.v - stable).abs() > raw.ev + 1024. * f64::EPSILON * stable
    {
        return Err(unresolved());
    }
    let mean: [f64; 3] = std::array::from_fn(|i| raw.m[i] / stable);
    let errors: [f64; 3] = std::array::from_fn(|i| {
        raw.em[i] / stable + mean[i].abs() * raw.ev / stable + 1024. * f64::EPSILON * mean[i].abs()
    });
    if errors
        .iter()
        .any(|e| !e.is_finite() || e * scale > policy.linear() / 8.)
    {
        return Err(unresolved());
    }
    let cov: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| raw.q[i][j] / stable - mean[i] * mean[j]));
    let mut error = 0f64;
    for i in 0..3 {
        for j in 0..3 {
            let e = raw.eq[i][j] / stable
                + (raw.q[i][j] / stable).abs() * raw.ev / stable
                + mean[i].abs() * errors[j]
                + mean[j].abs() * errors[i]
                + errors[i] * errors[j]
                + 1024. * f64::EPSILON * (raw.q[i][j] / stable).abs();
            error = error.max(e);
        }
    }
    if !error.is_finite()
        || 3. * error > policy.linear() / (8. * scale)
        || cov.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(unresolved());
    }
    // Subtract a spectral error enclosure before Cholesky; never clamp an
    // unresolved eigenvalue or off-diagonal coefficient into a valid tensor.
    let mut lower = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let mut x = cov[i][j] - if i == j { 3. * error } else { 0. };
            for (a, b) in lower[i].iter().zip(lower[j].iter()).take(j) {
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
    let local = Point3::new(mean[0] * scale, mean[1] * scale, mean[2] * scale);
    let centroid = source.frame().point(local);
    let world = centroid
        .x
        .abs()
        .max(centroid.y.abs())
        .max(centroid.z.abs())
        .max(scale);
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
impl NurbsObliqueFrustumSolid {
    pub fn mass_properties(&self, policy: GeometryTolerance) -> Result<NurbsGraphMassProperties> {
        let m = moments(self, policy)?;
        Ok(NurbsGraphMassProperties {
            volume: m.volume,
            centroid: m.centroid,
        })
    }
    /// Uniform-density centroidal inertia in world axes, in length^5 units.
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
        let axes = self.source().frame().axes();
        let component = |a: Vec3, i: usize| [a.x, a.y, a.z][i];
        let coefficients: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let mut coefficient = 0.;
                for (a, row) in local.iter().enumerate() {
                    for (b, value) in row.iter().enumerate() {
                        coefficient += component(axes[a], i) * value * component(axes[b], j);
                    }
                }
                coefficient
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
