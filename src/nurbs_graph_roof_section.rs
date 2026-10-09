//! Exact polynomial roof pullbacks along finite affine UV segments.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphRoofSpan {
    pub curve: NurbsCurve,
    pub pcurve: PCurve,
    /// Original segment parameter, never renormalized after clipping.
    pub parameter_range: [f64; 2],
    pub face: usize,
}
#[derive(Clone, Debug)]
pub struct NurbsGraphRoofSection {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub spans: Vec<NurbsGraphRoofSpan>,
    source: Source,
    endpoints: [[f64; 2]; 2],
}
#[derive(Clone, Debug)]
enum Source {
    Plain(NurbsGraphSolid),
    Holed(NurbsGraphHoledSolid),
}
impl Source {
    fn graph(&self) -> &NurbsGraphSolid {
        match self {
            Self::Plain(s) => s,
            Self::Holed(s) => s.source(),
        }
    }
    fn hole(&self) -> Option<[[f64; 2]; 2]> {
        match self {
            Self::Plain(_) => None,
            Self::Holed(s) => Some(s.hole()),
        }
    }
    fn validate(&self, tol: Tolerance) -> Result<()> {
        match self {
            Self::Plain(s) => s.validate(tol),
            Self::Holed(s) => s.validate(tol),
        }
    }
}
fn unresolved() -> Error {
    Error::Unsupported("roof section contact, clipping, or floating-point precision is unresolved")
}
fn build(
    source: Source,
    start: [f64; 2],
    end: [f64; 2],
    tol: GeometryTolerance,
) -> Result<NurbsGraphRoofSection> {
    source.validate(tol.absolute())?;
    let graph = source.graph();
    let ranges = graph.source_domain();
    let [l, w, h] = graph.dimensions();
    if !start.iter().chain(end.iter()).all(|x| x.is_finite()) {
        return Err(Error::InvalidInput("roof section endpoints must be finite"));
    }
    for a in 0..2 {
        if start[a] < ranges[a][0]
            || start[a] > ranges[a][1]
            || end[a] < ranges[a][0]
            || end[a] > ranges[a][1]
        {
            return Err(Error::InvalidInput(
                "roof section endpoints must lie in retained UV domain",
            ));
        }
    }
    let direction = [end[0] - start[0], end[1] - start[1]];
    let local = NurbsGraphSolid::new(graph.dimensions(), graph.bulge(), tol.absolute())?
        .trimmed_uv(graph.source_domain(), tol.absolute())?;
    let bounds = local.bounds()?;
    let diag = (bounds.max - bounds.min).norm();
    let budget = tol.length_at_scale(diag)?;
    let arithmetic = graph.arithmetic_budget()?;
    let length = (l * direction[0]).hypot(w * direction[1]);
    if !length.is_finite() || length <= 2. * budget || arithmetic >= budget / 4. {
        return Err(unresolved());
    }
    let uvguard = 128. * f64::EPSILON;
    for a in 0..2 {
        if direction[a] != 0.
            && direction[a].abs()
                <= uvguard
                    * start[a]
                        .abs()
                        .max(end[a].abs())
                        .max(ranges[a][1] - ranges[a][0])
        {
            return Err(unresolved());
        }
    }
    // Paths coincident with an outer wall are intentionally outside this scope.
    for a in 0..2 {
        if direction[a] == 0. && (start[a] == ranges[a][0] || start[a] == ranges[a][1]) {
            return Err(unresolved());
        }
    }
    let mut intervals = vec![[0., 1.]];
    if let Some(hole) = source.hole() {
        let physical = [l, w];
        for p in [start, end] {
            for a in 0..2 {
                let other = 1 - a;
                if p[other] >= hole[other][0] - uvguard && p[other] <= hole[other][1] + uvguard {
                    for edge in hole[a] {
                        if (p[a] - edge).abs() * physical[a] <= budget + arithmetic {
                            return Err(unresolved());
                        }
                    }
                }
            }
        }
        let mut lower: f64 = 0.;
        let mut upper: f64 = 1.;
        let mut possible = true;
        for a in 0..2 {
            if direction[a] == 0. {
                if hole[a]
                    .iter()
                    .any(|edge| (start[a] - edge).abs() * physical[a] <= budget + arithmetic)
                {
                    return Err(unresolved());
                }
                if start[a] <= hole[a][0] || start[a] >= hole[a][1] {
                    possible = false;
                }
            } else {
                let x = (hole[a][0] - start[a]) / direction[a];
                let y = (hole[a][1] - start[a]) / direction[a];
                lower = lower.max(x.min(y));
                upper = upper.min(x.max(y));
            }
        }
        if possible && lower > upper && (lower - upper) * length <= 2. * (budget + arithmetic) {
            return Err(unresolved());
        }
        if possible && lower <= upper {
            if upper - lower <= uvguard || (upper - lower) * length <= 2. * (budget + arithmetic) {
                return Err(unresolved());
            }
            for t in [lower, upper] {
                if t > 0. && t < 1. {
                    let p = [start[0] + direction[0] * t, start[1] + direction[1] * t];
                    let corner = hole[0]
                        .iter()
                        .any(|x| (p[0] - x).abs() <= uvguard.max((budget + arithmetic) / l))
                        && hole[1]
                            .iter()
                            .any(|x| (p[1] - x).abs() <= uvguard.max((budget + arithmetic) / w));
                    if corner {
                        return Err(unresolved());
                    }
                }
            }
            intervals.clear();
            if lower > 0. {
                intervals.push([0., lower]);
            }
            if upper < 1. {
                intervals.push([upper, 1.]);
            }
        }
    }
    let mut spans = Vec::new();
    for range in intervals {
        if range[1] - range[0] <= uvguard || length * (range[1] - range[0]) <= 2. * budget {
            return Err(unresolved());
        }
        let a = std::array::from_fn::<_, 2, _>(|i| start[i] + direction[i] * range[0]);
        let z = std::array::from_fn::<_, 2, _>(|i| start[i] + direction[i] * range[1]);
        let factors = [
            [a[0], z[0]],
            [1. - a[0], 1. - z[0]],
            [a[1], z[1]],
            [1. - a[1], 1. - z[1]],
        ];
        // Bernstein product: C(n,i)C(1,j)/C(n+1,i+j).
        let mut coefficients = vec![1.];
        for factor in factors {
            let degree = coefficients.len() - 1;
            let mut next = vec![0.; degree + 2];
            for (i, c) in coefficients.iter().enumerate() {
                next[i] += c * factor[0] * (degree + 1 - i) as f64 / (degree + 1) as f64;
                next[i + 1] += c * factor[1] * (i + 1) as f64 / (degree + 1) as f64;
            }
            coefficients = next;
        }
        let controls = (0..5)
            .map(|i| {
                let f = i as f64 / 4.;
                graph.placement().point(Point3::new(
                    l * (a[0] + (z[0] - a[0]) * f),
                    w * (a[1] + (z[1] - a[1]) * f),
                    h + graph.bulge() * (4. * coefficients[i]),
                ))
            })
            .collect();
        let mut knots = vec![range[0]; 5];
        knots.extend(vec![range[1]; 5]);
        let curve = NurbsCurve::new(4, knots, controls, vec![1.; 5])?;
        spans.push(NurbsGraphRoofSpan {
            curve,
            pcurve: PCurve::Affine {
                origin: start,
                direction,
            },
            parameter_range: range,
            face: 1,
        });
    }
    Ok(NurbsGraphRoofSection {
        start,
        end,
        spans,
        source,
        endpoints: [start, end],
    })
}
impl NurbsGraphRoofSection {
    /// Validate the retained source and exact public section certificate.
    pub fn validate(&self, tol: GeometryTolerance) -> Result<()> {
        let expected = build(
            self.source.clone(),
            self.endpoints[0],
            self.endpoints[1],
            tol,
        )?;
        if self.start != expected.start
            || self.end != expected.end
            || self.spans.len() != expected.spans.len()
        {
            return Err(Error::InvalidInput("roof section metadata changed"));
        }
        for (a, b) in self.spans.iter().zip(expected.spans) {
            let same_pcurve = matches!((&a.pcurve,&b.pcurve),(PCurve::Affine{origin:a,direction:da},PCurve::Affine{origin:b,direction:db}) if a==b && da==db);
            if !same_pcurve
                || a.face != b.face
                || a.parameter_range != b.parameter_range
                || a.curve.degree() != b.curve.degree()
                || a.curve.knots() != b.curve.knots()
                || a.curve.weights() != b.curve.weights()
                || a.curve.control_points() != b.curve.control_points()
            {
                return Err(Error::InvalidInput("roof section geometry changed"));
            }
        }
        Ok(())
    }
}
impl NurbsGraphSolid {
    pub fn roof_section(
        &self,
        start: [f64; 2],
        end: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphRoofSection> {
        build(Source::Plain(self.clone()), start, end, tol)
    }
}
impl NurbsGraphHoledSolid {
    pub fn roof_section(
        &self,
        start: [f64; 2],
        end: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphRoofSection> {
        build(Source::Holed(self.clone()), start, end, tol)
    }
}
