//! Uniform-density centroidal inertia of the checked polynomial graph family.
//! Four-point tensor Gauss integrates each column moment exactly in real arithmetic
//! (degree at most six). Floating-point guards are engineering allowances.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NurbsGraphInertiaProperties {
    pub volume: f64,
    pub centroid: Point3,
    /// Centroidal tensor in world axes, for unit density (length to the fifth power).
    pub inertia: [[f64; 3]; 3],
}
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
    Error::InvalidInput("graph inertia overflows, underflows, or loses relative precision")
}
fn integrate(
    source: &NurbsGraphSolid,
    hole: Option<[[f64; 2]; 2]>,
    actual: NurbsGraphMassProperties,
    tol: Tolerance,
) -> Result<NurbsGraphInertiaProperties> {
    // Reconstruct in the local frame rather than subtracting a large world origin.
    let local = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol)?
        .trimmed_uv(source.source_domain(), tol)?;
    let local_mass = if let Some(hole) = hole {
        NurbsGraphHoledSolid::new(&local, hole, tol)?.mass_properties(tol)?
    } else {
        local.mass_properties(tol)?
    };
    let [u, v] = source.source_domain();
    let rectangles = if let Some([hu, hv]) = hole {
        vec![
            [[u[0], hu[0]], v],
            [[hu[1], u[1]], v],
            [hu, [v[0], hv[0]]],
            [hu, [hv[1], v[1]]],
        ]
    } else {
        vec![[u, v]]
    };
    let [l, w, h] = source.dimensions();
    let b = source.bulge();
    let bounds = local.bounds()?;
    let scale = (l * (u[1] - u[0])).max(w * (v[1] - v[0])).max(bounds.max.z);
    if !scale.is_finite() || scale < f64::MIN_POSITIVE {
        return Err(unresolved());
    }
    let widths: [f64; 2] = std::array::from_fn(|a| {
        rectangles
            .iter()
            .map(|r| r[a][1] - r[a][0])
            .fold(0., f64::max)
    });
    let cx = local_mass.centroid.x / l;
    let cy = local_mass.centroid.y / w;
    let cz = local_mass.centroid.z / scale;
    let outer = ((3. + 2. * (6f64 / 5.).sqrt()) / 7.).sqrt();
    let inner = ((3. - 2. * (6f64 / 5.).sqrt()) / 7.).sqrt();
    let nodes = [-outer, -inner, inner, outer];
    let wo = (18. - 30f64.sqrt()) / 72.;
    let wi = (18. + 30f64.sqrt()) / 72.;
    let weights = [wo, wi, wi, wo];
    let mut moments: [Sum; 7] = std::array::from_fn(|_| Sum::default());
    for r in rectangles {
        let d = r.map(|[a, b]| b - a);
        let area = (d[0] / widths[0]) * (d[1] / widths[1]);
        if !area.is_finite() || area <= 0. {
            return Err(unresolved());
        }
        for i in 0..4 {
            for j in 0..4 {
                let pu = r[0][0] + d[0] * (1. + nodes[i]) / 2.;
                let pv = r[1][0] + d[1] * (1. + nodes[j]) / 2.;
                let height = h / scale + (b / scale) * 4. * pu * (1. - pu) * pv * (1. - pv);
                let dx = (l / scale) * (pu - cx);
                let dy = (w / scale) * (pv - cy);
                let dz = height / 2. - cz;
                let weight = area * weights[i] * weights[j] * height;
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
impl NurbsGraphSolid {
    /// Checked uniform-density centroidal inertia, expressed in world axes.
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        integrate(self, None, self.mass_properties(tol)?, tol)
    }
}
impl NurbsGraphHoledSolid {
    /// Integrate four positive material strips, without subtracting large tensors.
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        integrate(
            self.source(),
            Some(self.hole()),
            self.mass_properties(tol)?,
            tol,
        )
    }
}
