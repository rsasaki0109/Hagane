//! Positive material-triangle polynomial moments for a convex polygon opening.
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
impl NurbsGraphPolygonHoledSolid {
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
        let material = crate::nurbs_graph_polygon_material::material_triangles(
            self.outer_polygon(),
            self.opening(),
            [l, w],
            4. * tol.linear + source.arithmetic_budget()?,
        )?;
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
