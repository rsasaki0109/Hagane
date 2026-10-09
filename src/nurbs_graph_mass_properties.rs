//! Uniform-density volume and centroid of the canonical polynomial graph family.
//! Tensor three-point Gauss is mathematically exact through degree five per axis.
//! Column moments h, u*h, v*h and h² have degrees at most four. Binary64 guards
//! remain engineering allowances; this does not claim interval-certified moments.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NurbsGraphMassProperties {
    pub volume: f64,
    pub centroid: Point3,
}
#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
}
impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.value + value;
        self.correction += if self.value.abs() >= value.abs() {
            (self.value - next) + value
        } else {
            (value - next) + self.value
        };
        self.value = next;
    }
    fn total(&self) -> f64 {
        self.value + self.correction
    }
}
fn integrate(
    source: &NurbsGraphSolid,
    rectangles: &[[[f64; 2]; 2]],
    volume: f64,
) -> Result<NurbsGraphMassProperties> {
    let [l, w, h] = source.dimensions();
    let b = source.bulge();
    let height_scale = h.max((h + b).abs());
    let width_scale: [f64; 2] = std::array::from_fn(|axis| {
        rectangles
            .iter()
            .map(|r| r[axis][1] - r[axis][0])
            .fold(0., f64::max)
    });
    if !volume.is_finite()
        || volume < f64::MIN_POSITIVE
        || !height_scale.is_finite()
        || height_scale <= 0.
        || width_scale.iter().any(|s| !s.is_finite() || *s <= 0.)
    {
        return Err(Error::InvalidInput(
            "graph mass-property scales or volume are unresolved",
        ));
    }
    let coordinate = [l * width_scale[0], w * width_scale[1]];
    if coordinate.iter().any(|s| !s.is_finite() || *s <= 0.) {
        return Err(Error::InvalidInput(
            "graph mass-property physical interval is unresolved",
        ));
    }
    // Normalize each axis and height separately: neither physical area, h²,
    // nor world-coordinate first moments need to be formed.
    let nodes = [-(3f64 / 5.).sqrt(), 0., (3f64 / 5.).sqrt()];
    let weights = [5. / 18., 4. / 9., 5. / 18.];
    let mut height = Sum::default();
    let mut squared = Sum::default();
    let mut u_moment = Sum::default();
    let mut v_moment = Sum::default();
    for rectangle in rectangles {
        let widths = rectangle.map(|[a, b]| b - a);
        let middle = std::array::from_fn::<_, 2, _>(|axis| rectangle[axis][0] + widths[axis] / 2.);
        let area = (widths[0] / width_scale[0]) * (widths[1] / width_scale[1]);
        if !area.is_finite() || area <= 0. {
            return Err(Error::InvalidInput(
                "graph mass-property strip weight is unresolved",
            ));
        }
        for i in 0..3 {
            for j in 0..3 {
                let u = middle[0] + widths[0] * nodes[i] / 2.;
                let v = middle[1] + widths[1] * nodes[j] / 2.;
                let normalized =
                    h / height_scale + (b / height_scale) * 4. * u * (1. - u) * v * (1. - v);
                let weight = area * weights[i] * weights[j];
                let term = weight * normalized;
                if !normalized.is_finite() || normalized <= 0. || !term.is_finite() || term <= 0. {
                    return Err(Error::InvalidInput(
                        "graph mass-property positive quadrature is unresolved",
                    ));
                }
                let first_u = term * u;
                let first_v = term * v;
                let second = term * normalized;
                if !first_u.is_finite()
                    || !first_v.is_finite()
                    || !second.is_finite()
                    || (u > 0. && first_u < f64::MIN_POSITIVE)
                    || (v > 0. && first_v < f64::MIN_POSITIVE)
                    || second < f64::MIN_POSITIVE
                {
                    return Err(Error::InvalidInput("graph mass-property normalized moments underflow or lose relative precision"));
                }
                height.add(term);
                squared.add(second);
                u_moment.add(first_u);
                v_moment.add(first_v);
            }
        }
    }
    let height = height.total();
    let squared = squared.total();
    let mu = u_moment.total() / height;
    let mv = v_moment.total() / height;
    let mz = squared / height;
    let expected = ((volume / coordinate[0]) / coordinate[1]) / height_scale;
    let allowance = 2048. * f64::EPSILON * height.abs().max(expected.abs());
    if !height.is_finite()
        || height <= 0.
        || !expected.is_finite()
        || expected <= 0.
        || !allowance.is_finite()
        || (height - expected).abs() > allowance
        || ![mu, mv, mz].iter().all(|x| x.is_finite() && *x >= 0.)
    {
        return Err(Error::InvalidInput(
            "graph mass-property volume or normalized moments are unresolved",
        ));
    }
    let local = Point3::new(l * mu, w * mv, height_scale * (mz / 2.));
    let centroid = source.placement().point(local);
    if !local.finite() || !centroid.finite() {
        return Err(Error::InvalidInput(
            "graph mass-property centroid placement overflows",
        ));
    }
    Ok(NurbsGraphMassProperties { volume, centroid })
}
impl NurbsGraphSolid {
    /// Uniform-density volume and centroid from polynomial column integrals.
    /// Validates the retained B-rep; no tessellation or world first moments are used.
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(tol)?;
        integrate(self, &[self.source_domain()], self.volume()?)
    }
}
impl NurbsGraphHoledSolid {
    /// Sum positive material-strip moments, avoiding subtraction of large bodies.
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        self.validate(tol)?;
        let [u, v] = self.source().source_domain();
        let [hu, hv] = self.hole();
        let strips = [
            [[u[0], hu[0]], v],
            [[hu[1], u[1]], v],
            [hu, [v[0], hv[0]]],
            [hu, [hv[1], v[1]]],
        ];
        integrate(self.source(), &strips, self.volume()?)
    }
}
