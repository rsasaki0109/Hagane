//! Checked source-axis vertical material sections of canonical graph bodies.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphVerticalEvent {
    /// Physical source-height parameter of the unit placed source-Z line.
    pub parameter: f64,
    pub point: Point3,
    pub normal: Vec3,
    pub face: usize,
    pub entering: bool,
}
#[derive(Clone, Debug)]
pub struct NurbsGraphVerticalSection {
    pub uv: [f64; 2],
    pub origin: Point3,
    pub direction: Vec3,
    /// Physical source-height intervals; these are not Curve::Line parameters.
    pub intervals: Vec<[f64; 2]>,
    pub events: Vec<NurbsGraphVerticalEvent>,
    /// Retained finite boundary-to-boundary line segments, each parameterized 0..1.
    pub segments: Vec<Curve>,
    source: SectionSource,
    certified_uv: [f64; 2],
    tolerance: GeometryTolerance,
}
#[derive(Clone, Debug)]
enum SectionSource {
    Plain(NurbsGraphSolid),
    Holed(NurbsGraphHoledSolid),
}
impl SectionSource {
    fn validate(&self, tol: GeometryTolerance) -> Result<()> {
        match self {
            Self::Plain(s) => s.validate(tol.absolute()),
            Self::Holed(s) => s.validate(tol.absolute()),
        }
    }
    fn graph(&self) -> &NurbsGraphSolid {
        match self {
            Self::Plain(s) => s,
            Self::Holed(s) => s.source(),
        }
    }
    fn solid(&self) -> &Solid {
        match self {
            Self::Plain(s) => s.brep(),
            Self::Holed(s) => s.brep(),
        }
    }
    fn hole(&self) -> Option<[[f64; 2]; 2]> {
        match self {
            Self::Plain(_) => None,
            Self::Holed(s) => Some(s.hole()),
        }
    }
}
fn perimeter_distance(point: [f64; 2], ranges: [[f64; 2]; 2]) -> Result<f64> {
    let [x, y] = point;
    let [[x0, x1], [y0, y1]] = ranges;
    let cx = x.clamp(x0, x1);
    let cy = y.clamp(y0, y1);
    let distance = (x - cx)
        .hypot(y - y0)
        .min((x - cx).hypot(y - y1))
        .min((x - x0).hypot(y - cy))
        .min((x - x1).hypot(y - cy));
    if !distance.is_finite() {
        return Err(Error::InvalidInput(
            "vertical section wall distance overflows",
        ));
    }
    Ok(distance)
}
fn generate(
    source: SectionSource,
    uv: [f64; 2],
    tol: GeometryTolerance,
) -> Result<NurbsGraphVerticalSection> {
    source.validate(tol)?;
    if uv.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "vertical section requires finite source UV",
        ));
    }
    let graph = source.graph();
    let [l, w, h] = graph.dimensions();
    let placement = graph.placement();
    let xy = [l * uv[0], w * uv[1]];
    let origin = placement.point(Point3::new(xy[0], xy[1], 0.));
    let direction = placement.axes()[2];
    if !origin.finite() || !direction.finite() {
        return Err(Error::InvalidInput("vertical section world line overflows"));
    }
    let local = NurbsGraphSolid::new(graph.dimensions(), graph.bulge(), tol.absolute())?
        .trimmed_uv(graph.source_domain(), tol.absolute())?;
    let bounds = local.bounds()?;
    let budget = tol.length_at_scale((bounds.max - bounds.min).norm())?;
    let worldscale = origin.x.abs().max(origin.y.abs()).max(origin.z.abs());
    let guard = graph.arithmetic_budget()? + 256. * f64::EPSILON * worldscale;
    if !guard.is_finite() || guard >= budget / 8. {
        return Err(Error::Unsupported(
            "vertical section precision cannot resolve its Euclidean contact tolerance",
        ));
    }
    let domain = graph.source_domain();
    let outer = [
        [l * domain[0][0], l * domain[0][1]],
        [w * domain[1][0], w * domain[1][1]],
    ];
    // Every vertical wall includes its floor edge. The minimum distance from
    // the complete vertical line to that wall equals XY segment distance.
    // Corner contacts therefore use hypot rather than independent axis bands.
    if perimeter_distance(xy, outer)? <= budget + guard {
        return Err(Error::Unsupported(
            "vertical section contacts the outer wall tolerance band",
        ));
    }
    let inside = uv[0] > domain[0][0]
        && uv[0] < domain[0][1]
        && uv[1] > domain[1][0]
        && uv[1] < domain[1][1];
    let mut material = inside;
    if inside {
        if let Some(hole) = source.hole() {
            let rectangle = [
                [l * hole[0][0], l * hole[0][1]],
                [w * hole[1][0], w * hole[1][1]],
            ];
            if perimeter_distance(xy, rectangle)? <= budget + guard {
                return Err(Error::Unsupported(
                    "vertical section contacts the opening wall tolerance band",
                ));
            }
            material = !(uv[0] > hole[0][0]
                && uv[0] < hole[0][1]
                && uv[1] > hole[1][0]
                && uv[1] < hole[1][1]);
        }
    }
    let mut intervals = Vec::new();
    let mut events = Vec::new();
    let mut segments = Vec::new();
    if material {
        let height = h + 4. * graph.bulge() * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1]);
        if !height.is_finite() || height <= guard {
            return Err(Error::Unsupported(
                "vertical section source height is unresolved",
            ));
        }
        let mut endpoints = [Point3::new(0., 0., 0.); 2];
        for (face, endpoint) in endpoints.iter_mut().enumerate() {
            let retained = &source.solid().shell.faces[face];
            let Surface::Nurbs(surface) = &retained.surface else {
                return Err(Error::InvalidTopology(
                    "vertical section requires rational caps",
                ));
            };
            let jet = surface.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
            let parameter = if face == 0 { 0. } else { height };
            let expected = origin + direction * parameter;
            let distance = (jet.point - expected).norm();
            if !distance.is_finite() || distance + guard > budget / 4. {
                return Err(Error::Unsupported(
                    "vertical section cap and source-height line disagree within tolerance",
                ));
            }
            let normal = jet.normal()? * retained.orientation as f64;
            if !normal.finite() || normal.dot(direction) * if face == 0 { -1. } else { 1. } <= 0. {
                return Err(Error::InvalidTopology(
                    "vertical section cap crossing orientation is invalid",
                ));
            }
            *endpoint = jet.point;
            events.push(NurbsGraphVerticalEvent {
                parameter,
                point: jet.point,
                normal,
                face,
                entering: face == 0,
            });
        }
        intervals.push([0., height]);
        segments.push(Curve::Line {
            a: endpoints[0],
            b: endpoints[1],
        });
    }
    Ok(NurbsGraphVerticalSection {
        uv,
        origin,
        direction,
        intervals,
        events,
        segments,
        certified_uv: uv,
        source,
        tolerance: tol,
    })
}
impl NurbsGraphSolid {
    /// Section by the exact placed source-Z line at original-source UV.
    /// Wall contact bands are unsupported; arbitrary world directions are not inferred.
    pub fn vertical_section(
        &self,
        uv: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphVerticalSection> {
        generate(SectionSource::Plain(self.clone()), uv, tol)
    }
}
impl NurbsGraphHoledSolid {
    pub fn vertical_section(
        &self,
        uv: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphVerticalSection> {
        generate(SectionSource::Holed(self.clone()), uv, tol)
    }
}
impl NurbsGraphVerticalSection {
    /// Validate the public line, interval, event and segment metadata against the
    /// retained canonical body and original source-coordinate certificate.
    pub fn validate(&self, tol: GeometryTolerance) -> Result<()> {
        let expected = generate(self.source.clone(), self.certified_uv, tol)?;
        let bad = || Error::InvalidTopology("vertical section differs from its source certificate");
        if self.uv != expected.uv
            || self.origin != expected.origin
            || self.direction != expected.direction
            || self.intervals != expected.intervals
            || self.events.len() != expected.events.len()
            || self.segments.len() != expected.segments.len()
        {
            return Err(bad());
        }
        for (a, b) in self.events.iter().zip(&expected.events) {
            if a.parameter != b.parameter
                || a.point != b.point
                || a.normal != b.normal
                || a.face != b.face
                || a.entering != b.entering
            {
                return Err(bad());
            }
        }
        for (a, b) in self.segments.iter().zip(&expected.segments) {
            let (Curve::Line { a: a0, b: a1 }, Curve::Line { a: b0, b: b1 }) = (a, b) else {
                return Err(bad());
            };
            if a0 != b0 || a1 != b1 {
                return Err(bad());
            }
        }
        Ok(())
    }
    /// Evaluate the physical source-height line; finite segment curves instead use 0..1.
    pub fn evaluate(&self, height: f64) -> Result<Point3> {
        self.validate(self.tolerance)?;
        let point = self.origin + self.direction * height;
        if !height.is_finite() || !point.finite() {
            return Err(Error::InvalidInput(
                "vertical section line evaluation overflows",
            ));
        }
        Ok(point)
    }
}
