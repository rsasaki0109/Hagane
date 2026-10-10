//! Original-parameter line and ray queries on actual retained frustum bodies.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsFrustumMaterialLineIntersection {
    pub hits: Vec<NurbsFrustumLineHit>,
    pub material_intervals: Vec<[f64; 2]>,
}
#[derive(Clone, Copy)]
enum Retained<'a> {
    Bored(&'a NurbsBoredFrustumSolid),
    Oblique(&'a NurbsObliqueFrustumSolid),
    BoredOblique(&'a NurbsBoredObliqueFrustumSolid),
}
struct Segment {
    hits: Vec<NurbsFrustumSegmentHit>,
    material_intervals: Vec<[f64; 2]>,
}
impl Retained<'_> {
    fn source(&self) -> &NurbsFrustumSolid {
        match self {
            Self::Bored(v) => v.source(),
            Self::Oblique(v) => v.source(),
            Self::BoredOblique(v) => v.source().source(),
        }
    }
    fn solid(&self) -> &Solid {
        match self {
            Self::Bored(v) => v.solid(),
            Self::Oblique(v) => v.solid(),
            Self::BoredOblique(v) => v.solid(),
        }
    }
    fn validate(&self, p: GeometryTolerance) -> Result<()> {
        match self {
            Self::Bored(v) => v.validate(p),
            Self::Oblique(v) => v.validate(p),
            Self::BoredOblique(v) => v.validate(p),
        }
    }
    fn classify_point(&self, q: Point3, p: GeometryTolerance) -> Result<PointLocation> {
        match self {
            Self::Bored(v) => v.classify_point(q, p),
            Self::Oblique(v) => v.classify_point(q, p),
            Self::BoredOblique(v) => v.classify_point(q, p),
        }
    }
    fn intersect_segment(&self, a: Point3, b: Point3, p: GeometryTolerance) -> Result<Segment> {
        match self {
            Self::Bored(v) => {
                let s = v.intersect_segment(a, b, p)?;
                Ok(Segment {
                    hits: s.hits,
                    material_intervals: s.material_intervals,
                })
            }
            Self::BoredOblique(v) => {
                let s = v.intersect_segment(a, b, p)?;
                Ok(Segment {
                    hits: s.hits,
                    material_intervals: s.material_intervals,
                })
            }
            Self::Oblique(v) => {
                let s = v.intersect_segment(a, b, p)?;
                Ok(Segment {
                    hits: s.hits,
                    material_intervals: s.material_interval.into_iter().collect(),
                })
            }
        }
    }
}
fn unresolved() -> Error {
    Error::Unsupported(
        "frustum line/ray cropping, original-parameter mapping or query precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn empty() -> NurbsFrustumMaterialLineIntersection {
    NurbsFrustumMaterialLineIntersection {
        hits: vec![],
        material_intervals: vec![],
    }
}
impl Retained<'_> {
    /// Query an infinite world line, retaining its original nonunit parameter.
    /// Unresolved contact/precision cases inherit the checked segment refusal.
    pub fn intersect_line(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        self.validate(policy)?;
        if !origin.finite() || !direction.finite() || direction == Vec3::new(0., 0., 0.) {
            return Err(Error::InvalidInput(
                "line requires a finite origin and finite nonzero direction",
            ));
        }
        let unit = direction.normalized()?;
        let [r0, r1] = self.source().radii();
        let height = self.source().height();
        let r = r0.max(r1);
        let scale = (2. * r).hypot(height);
        let band = policy.length_at_scale(scale)?;
        // Guard the original anchor before any cropping can conceal cancellation.
        let frame = self.source().frame();
        let delta = origin - frame.origin();
        let distance = norm(delta);
        let world = scale
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs())
            .max(frame.origin().x.abs())
            .max(frame.origin().y.abs())
            .max(frame.origin().z.abs());
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let arithmetic = 65536. * f64::EPSILON * world + 8. * gram * distance;
        if !delta.finite()
            || !distance.is_finite()
            || !arithmetic.is_finite()
            || arithmetic >= policy.linear() / 16.
        {
            return Err(unresolved());
        }
        let a = frame.local_point(origin);
        let d = frame.local_vector(unit);
        if !a.finite() || !d.finite() {
            return Err(unresolved());
        }
        let margin = 32. * band + 16. * arithmetic + 16. * gram * scale;
        let minima = [-r - margin, -r - margin, -margin];
        let maxima = [r + margin, r + margin, height + margin];
        let coords = [a.x, a.y, a.z];
        let slopes = [d.x, d.y, d.z];
        // Any finite-body crossing is within this unit-speed travel enclosure.
        // It also bounds directional rounding before slab comparisons.
        let travel_bound = distance + 2. * scale;
        if !travel_bound.is_finite() {
            return Err(unresolved());
        }
        let mut lower = -travel_bound;
        let mut upper = travel_bound;
        for axis in 0..3 {
            if slopes[axis] == 0. {
                if coords[axis] < minima[axis] - arithmetic
                    || coords[axis] > maxima[axis] + arithmetic
                {
                    return Ok(empty());
                }
                if coords[axis] < minima[axis] + arithmetic
                    || coords[axis] > maxima[axis] - arithmetic
                {
                    return Err(unresolved());
                }
                continue;
            }
            let x = (minima[axis] - coords[axis]) / slopes[axis];
            let y = (maxima[axis] - coords[axis]) / slopes[axis];
            if !x.is_finite() || !y.is_finite() {
                return Err(unresolved());
            }
            lower = lower.max(x.min(y));
            upper = upper.min(x.max(y));
        }
        if !lower.is_finite() || !upper.is_finite() {
            return Err(unresolved());
        }
        if lower > upper {
            if lower - upper > 16. * band + 16. * arithmetic + 256. * f64::EPSILON * travel_bound {
                return Ok(empty());
            }
            return Err(unresolved());
        }
        if upper - lower <= 16. * band + 16. * arithmetic {
            return Err(unresolved());
        }
        let start = origin + unit * lower;
        let end = origin + unit * upper;
        if !start.finite()
            || !end.finite()
            || self.source().classify_point(start, policy)? != PointLocation::Outside
            || self.source().classify_point(end, policy)? != PointLocation::Outside
        {
            return Err(unresolved());
        }
        let segment = self.intersect_segment(start, end, policy)?;
        let map = |u: f64| -> Result<f64> {
            let travel = lower * (1. - u) + upper * u;
            let t = crate::intersections::line_parameter(travel, direction)?;
            let original = origin + direction * t;
            let cropped = start + (end - start) * u;
            if !original.finite() || norm(original - cropped) + arithmetic >= policy.linear() / 4. {
                return Err(unresolved());
            }
            Ok(t)
        };
        let mut hits = vec![];
        for hit in segment.hits {
            let parameter = map(hit.parameter)?;
            let point = origin + direction * parameter;
            for witness in &hit.faces {
                let actual = self.solid().shell.faces[witness.face_id]
                    .surface
                    .try_evaluate(witness.uv[0], witness.uv[1])?;
                if norm(actual - point) + arithmetic >= policy.linear() / 2. {
                    return Err(unresolved());
                }
            }
            if self.classify_point(point, policy)? != PointLocation::Boundary {
                return Err(unresolved());
            }
            hits.push(NurbsFrustumLineHit {
                parameter,
                point,
                faces: hit.faces,
            });
        }
        if hits.windows(2).any(|w| w[0].parameter >= w[1].parameter) {
            return Err(unresolved());
        }
        let mut material_intervals = vec![];
        for interval in segment.material_intervals {
            let v = [map(interval[0])?, map(interval[1])?];
            if v[0] >= v[1] {
                return Err(unresolved());
            }
            if material_intervals
                .last()
                .is_some_and(|last: &[f64; 2]| last[1] >= v[0])
            {
                return Err(unresolved());
            }
            let first = origin + direction * v[0];
            let last = origin + direction * v[1];
            let midpoint = origin + direction * (v[0] + (v[1] - v[0]) / 2.);
            if !first.finite()
                || !last.finite()
                || !midpoint.finite()
                || norm(last - first) <= 10. * band
                || self.classify_point(midpoint, policy)? != PointLocation::Inside
            {
                return Err(unresolved());
            }
            material_intervals.push(v);
        }
        Ok(NurbsFrustumMaterialLineIntersection {
            hits,
            material_intervals,
        })
    }

    /// Query a forward ray with original nonunit parameter t >= 0.
    /// Origins in the boundary band are explicitly unsupported.
    pub fn intersect_ray(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        self.validate(policy)?;
        if !origin.finite() || !direction.finite() || direction == Vec3::new(0., 0., 0.) {
            return Err(Error::InvalidInput(
                "ray requires a finite origin and finite nonzero direction",
            ));
        }
        let location = self.classify_point(origin, policy)?;
        if location == PointLocation::Boundary {
            return Err(Error::Unsupported(
                "ray origin is within the boundary band; move it away from the boundary",
            ));
        }
        let line = self.intersect_line(origin, direction, policy)?;
        let straddles = line
            .material_intervals
            .iter()
            .filter(|i| i[0] < 0. && i[1] > 0.)
            .count();
        if (location == PointLocation::Inside) != (straddles == 1) || straddles > 1 {
            return Err(unresolved());
        }
        let mut material_intervals = vec![];
        for interval in &line.material_intervals {
            if interval[1] < 0. {
                continue;
            }
            if interval[1] == 0. || interval[0] == 0. {
                return Err(unresolved());
            }
            material_intervals.push([interval[0].max(0.), interval[1]]);
        }
        let unit = direction.normalized()?;
        let scale = (2. * self.source().radii()[0].max(self.source().radii()[1]))
            .hypot(self.source().height());
        let band = policy.length_at_scale(scale)?;
        let mut hits = vec![];
        for hit in line.hits {
            let travel = (hit.point - origin).dot(unit);
            if !travel.is_finite() || travel.abs() <= 10. * band {
                return Err(unresolved());
            }
            if (travel > 0.) != (hit.parameter > 0.) {
                return Err(unresolved());
            }
            if hit.parameter > 0. {
                hits.push(hit);
            }
        }
        Ok(NurbsFrustumMaterialLineIntersection {
            hits,
            material_intervals,
        })
    }
}
impl NurbsBoredFrustumSolid {
    pub fn intersect_line(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::Bored(self).intersect_line(origin, direction, policy)
    }
    pub fn intersect_ray(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::Bored(self).intersect_ray(origin, direction, policy)
    }
}
impl NurbsObliqueFrustumSolid {
    pub fn intersect_line(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::Oblique(self).intersect_line(origin, direction, policy)
    }
    pub fn intersect_ray(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::Oblique(self).intersect_ray(origin, direction, policy)
    }
}

impl NurbsBoredObliqueFrustumSolid {
    pub fn intersect_line(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::BoredOblique(self).intersect_line(origin, direction, policy)
    }
    pub fn intersect_ray(
        &self,
        origin: Point3,
        direction: Vec3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumMaterialLineIntersection> {
        Retained::BoredOblique(self).intersect_ray(origin, direction, policy)
    }
}
