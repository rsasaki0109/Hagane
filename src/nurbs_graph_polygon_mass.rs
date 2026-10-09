//! Positive fan/Duffy quadrature for canonical convex polygon graph bodies.
//! Gauss5 integrates h² (total degree eight) and the Duffy Jacobian exactly in
//! real arithmetic. Binary64 guards do not constitute interval certification.
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
    Error::InvalidInput("polygon graph mass-property scales or moments are unresolved")
}
impl NurbsGraphPolygonSolid {
    /// Uniform-density volume from positive material triangles.
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
        let outer = (5. + 2. * (10f64 / 7.).sqrt()).sqrt() / 3.;
        let inner = (5. - 2. * (10f64 / 7.).sqrt()).sqrt() / 3.;
        let nodes = [-outer, -inner, 0., inner, outer];
        let wo = (322. - 13. * 70f64.sqrt()) / 1800.;
        let wi = (322. + 13. * 70f64.sqrt()) / 1800.;
        let weights = [wo, wi, 64. / 225., wi, wo];
        let corners = self.polygon();
        let center = std::array::from_fn::<_, 2, _>(|a| {
            corners
                .iter()
                .map(|p| p[a] / corners.len() as f64)
                .sum::<f64>()
        });
        let mut sums: [Sum; 4] = std::array::from_fn(|_| Sum::default());
        for k in 0..corners.len() {
            let a = corners[k];
            let b = corners[(k + 1) % corners.len()];
            let da = [a[0] - center[0], a[1] - center[1]];
            let db = [b[0] - center[0], b[1] - center[1]];
            let determinant = da[0] * db[1] - da[1] * db[0];
            if !determinant.is_finite() || determinant < f64::MIN_POSITIVE {
                return Err(unresolved());
            }
            for i in 0..5 {
                for j in 0..5 {
                    let r = (1. + nodes[i]) / 2.;
                    let s = (1. + nodes[j]) / 2.;
                    let u = center[0] + r * da[0] + (1. - r) * s * db[0];
                    let v = center[1] + r * da[1] + (1. - r) * s * db[1];
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
