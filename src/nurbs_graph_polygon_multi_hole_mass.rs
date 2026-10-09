//! Positive material-triangle mass and centered inertia for multiple convex openings.
//! Gauss5 integrates h²; Gauss7 integrates h³ after the Duffy substitution.
//! Floating-point guards are engineering allowances, not interval certificates.
use crate::*;
#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
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
    }
    fn total(&self) -> f64 {
        self.value + self.correction
    }
}
fn unresolved() -> Error {
    Error::InvalidInput("polygon opening mass-property scales or positive moments are unresolved")
}
impl NurbsGraphPolygonMultiHoledSolid {
    pub fn volume(&self) -> Result<f64> {
        Ok(self.mass_properties(self.construction_tolerance())?.volume)
    }
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(tol)?;
        let source = self.source();
        let [l, w, h] = source.dimensions();
        let bulge = source.bulge();
        let height_scale = h.max((h + bulge).abs());
        let source_volume = source.volume()?;
        let source_integral = ((source_volume / l) / w) / height_scale;
        if !height_scale.is_finite()
            || height_scale <= 0.
            || !source_integral.is_finite()
            || source_integral < f64::MIN_POSITIVE
        {
            return Err(unresolved());
        }
        let material = self.material()?;
        let outer = (5. + 2. * (10f64 / 7.).sqrt()).sqrt() / 3.;
        let inner = (5. - 2. * (10f64 / 7.).sqrt()).sqrt() / 3.;
        let nodes = [-outer, -inner, 0., inner, outer];
        let wo = (322. - 13. * 70f64.sqrt()) / 1800.;
        let wi = (322. + 13. * 70f64.sqrt()) / 1800.;
        let weights = [wo, wi, 64. / 225., wi, wo];
        let mut sums: [Sum; 4] = std::array::from_fn(|_| Sum::default());
        for tri in material.triangles {
            let c = material.points[tri[0]];
            let a = material.points[tri[1]];
            let b = material.points[tri[2]];
            let da = [a[0] - c[0], a[1] - c[1]];
            let db = [b[0] - c[0], b[1] - c[1]];
            let determinant = da[0] * db[1] - da[1] * db[0];
            if !determinant.is_finite() || determinant < f64::MIN_POSITIVE {
                return Err(unresolved());
            }
            for i in 0..5 {
                for j in 0..5 {
                    let r = (1. + nodes[i]) / 2.;
                    let s = (1. + nodes[j]) / 2.;
                    let u = c[0] + r * da[0] + (1. - r) * s * db[0];
                    let v = c[1] + r * da[1] + (1. - r) * s * db[1];
                    let normalized = h / height_scale
                        + (bulge / height_scale) * (4. * u * (1. - u) * v * (1. - v));
                    let term = determinant * (1. - r) * weights[i] * weights[j] * normalized;
                    let terms = [term, term * u, term * v, term * normalized];
                    if normalized <= 0.
                        || terms
                            .iter()
                            .any(|x| !x.is_finite() || *x < f64::MIN_POSITIVE)
                    {
                        return Err(unresolved());
                    }
                    for (sum, value) in sums.iter_mut().zip(terms) {
                        sum.add(value);
                    }
                }
            }
        }
        let sums = sums.map(|x| x.total());
        let fraction = sums[0] / source_integral;
        if !fraction.is_finite() || fraction <= 0. || fraction > 1. + 4096. * f64::EPSILON {
            return Err(unresolved());
        }
        let volume = source_volume * fraction;
        let local = Point3::new(
            l * (sums[1] / sums[0]),
            w * (sums[2] / sums[0]),
            height_scale * (sums[3] / sums[0]) / 2.,
        );
        let centroid = source.placement().point(local);
        if !volume.is_finite()
            || volume < f64::MIN_POSITIVE
            || !local.finite()
            || !centroid.finite()
        {
            return Err(unresolved());
        }
        Ok(NurbsGraphMassProperties { volume, centroid })
    }
}

fn integrate_inertia(
    body: &NurbsGraphPolygonMultiHoledSolid,
    actual: NurbsGraphMassProperties,
    tol: Tolerance,
) -> Result<NurbsGraphInertiaProperties> {
    let source = body.source();
    // Reconstruct in the local frame; never subtract a large world translation
    // from the reported centroid to recover centered integration coordinates.
    let local = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol)?
        .trimmed_uv(source.source_domain(), tol)?;
    let outer = NurbsGraphPolygonSolid::new(&local, body.outer_polygon().to_vec(), tol)?;
    let local_body = NurbsGraphPolygonMultiHoledSolid::new(&outer, body.holes().to_vec(), tol)?;
    let local_mass = local_body.mass_properties(tol)?;
    let material = local_body.material()?;
    let triangles = material
        .triangles
        .iter()
        .map(|tri| tri.map(|i| material.points[i]));
    let [u, v] = source.source_domain();
    let [l, w, h] = source.dimensions();
    let b = source.bulge();
    let bounds = local.bounds()?;
    let scale = (l * (u[1] - u[0])).max(w * (v[1] - v[0])).max(bounds.max.z);
    if !scale.is_finite() || scale < f64::MIN_POSITIVE {
        return Err(unresolved());
    }
    let cx = local_mass.centroid.x / l;
    let cy = local_mass.centroid.y / w;
    let cz = local_mass.centroid.z / scale;
    // Seven-point Gauss-Legendre has degree-thirteen precision. Duffy
    // substitution raises the degree-twelve h³ polynomial by one Jacobian degree.
    let nodes = [
        -0.9491079123427585,
        -0.7415311855993945,
        -0.4058451513773972,
        0.,
        0.4058451513773972,
        0.7415311855993945,
        0.9491079123427585,
    ];
    let weights = [
        0.06474248308443485,
        0.1398526957446383,
        0.19091502525255945,
        0.2089795918367347,
        0.19091502525255945,
        0.1398526957446383,
        0.06474248308443485,
    ];
    let mut moments: [Sum; 7] = std::array::from_fn(|_| Sum::default());
    for [c, a, bp] in triangles {
        let da = [a[0] - c[0], a[1] - c[1]];
        let db = [bp[0] - c[0], bp[1] - c[1]];
        let area = da[0] * db[1] - da[1] * db[0];
        if !area.is_finite() || area < f64::MIN_POSITIVE {
            return Err(unresolved());
        }
        for i in 0..7 {
            for j in 0..7 {
                let r = (1. + nodes[i]) / 2.;
                let t = (1. + nodes[j]) / 2.;
                let pu = c[0] + r * da[0] + (1. - r) * t * db[0];
                let pv = c[1] + r * da[1] + (1. - r) * t * db[1];
                let height = h / scale + (b / scale) * (4. * pu * (1. - pu) * pv * (1. - pv));
                let dx = (l / scale) * (pu - cx);
                let dy = (w / scale) * (pv - cy);
                let dz = height / 2. - cz;
                let weight = area * (1. - r) * weights[i] * weights[j] * height;
                let terms = [
                    weight,
                    weight * dx * dx,
                    weight * dy * dy,
                    weight * (dz * dz + height * height / 12.),
                    weight * dx * dy,
                    weight * dx * dz,
                    weight * dy * dz,
                ];
                // Preserve genuinely zero cross moments, but reject lost nonzero
                // normalized products before physical scaling could hide underflow.
                let expected_nonzero = [
                    true,
                    dx != 0.,
                    dy != 0.,
                    true,
                    dx != 0. && dy != 0.,
                    dx != 0. && dz != 0.,
                    dy != 0. && dz != 0.,
                ];
                if height <= 0.
                    || weight <= 0.
                    || height * height / 12. < f64::MIN_POSITIVE
                    || terms.iter().zip(expected_nonzero).any(|(x, nonzero)| {
                        !x.is_finite() || (nonzero && x.abs() < f64::MIN_POSITIVE)
                    })
                {
                    return Err(unresolved());
                }
                for (sum, term) in moments.iter_mut().zip(terms) {
                    sum.add(term);
                }
            }
        }
    }
    let m = moments.map(|x| x.total());
    if !m[0].is_finite() || m[0] <= 0. {
        return Err(unresolved());
    }
    let mut tensor = [[0.; 3]; 3];
    tensor[0][0] = (m[2] + m[3]) / m[0];
    tensor[1][1] = (m[1] + m[3]) / m[0];
    tensor[2][2] = (m[1] + m[2]) / m[0];
    tensor[0][1] = -m[4] / m[0];
    tensor[0][2] = -m[5] / m[0];
    tensor[1][2] = -m[6] / m[0];
    tensor[1][0] = tensor[0][1];
    tensor[2][0] = tensor[0][2];
    tensor[2][1] = tensor[1][2];
    let axes = source.placement().axes();
    let rotation = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| [axes[j].x, axes[j].y, axes[j].z][i])
    });
    let mut world = [[0.; 3]; 3];
    for i in 0..3 {
        for j in i..3 {
            let mut sum = Sum::default();
            let mut envelope = 0.;
            for a in 0..3 {
                for (bb, row) in rotation[j].iter().enumerate() {
                    let term = rotation[i][a] * tensor[a][bb] * row;
                    sum.add(term);
                    envelope += term.abs();
                }
            }
            let normalized = sum.total();
            if !normalized.is_finite() || (i == j && normalized <= 2048. * f64::EPSILON * envelope)
            {
                return Err(unresolved());
            }
            let mut value = normalized;
            for factor in [scale, scale, actual.volume] {
                let next = value * factor;
                if !next.is_finite()
                    || (value != 0. && (next == 0. || next.abs() < f64::MIN_POSITIVE))
                {
                    return Err(unresolved());
                }
                value = next;
            }
            if i == j && value < f64::MIN_POSITIVE {
                return Err(unresolved());
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

impl NurbsGraphPolygonMultiHoledSolid {
    /// Uniform-density centroidal inertia in world axes (length to the fifth).
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        integrate_inertia(self, self.mass_properties(tol)?, tol)
    }
}
