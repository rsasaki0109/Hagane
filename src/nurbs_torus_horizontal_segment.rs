//! Finite segments clipped against the actual annular cut cap; source quartic
//! refusals remain conservative, including discarded tangencies and endpoints.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "horizontal torus segment cut contact, physical rim or parameter precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsTorusHorizontalSolid {
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusSegmentIntersection> {
        self.validate(policy)?;
        if !start.finite() || !end.finite() || start == end {
            return Err(Error::InvalidInput(
                "horizontal torus segment endpoints must be finite and distinct",
            ));
        }
        let delta = end - start;
        let length = norm(delta);
        if !delta.finite() || !length.is_normal() {
            return Err(unresolved());
        }
        let locations = [
            self.classify_point(start, policy)?,
            self.classify_point(end, policy)?,
        ];
        if locations.contains(&PointLocation::Boundary) {
            return Err(Error::Unsupported(
                "horizontal torus segment endpoint is within the boundary band; move it away",
            ));
        }
        let [major, minor] = self.source().radii();
        let scale = 2. * (major + minor).hypot(minor);
        let band = policy.length_at_scale(scale)?;
        if length <= 10. * band {
            return Err(unresolved());
        }
        let n = self.strip_ranges().len();
        let cap_id = 4 * n;
        let cap = &self.solid().shell.faces[cap_id];
        let Surface::Plane { origin, .. } = cap.surface else {
            return Err(unresolved());
        };
        let normal = cap.surface.normal_at(0., 0.)? * f64::from(cap.orientation);
        let frame = self.source().frame();
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let world = [start, end, origin, frame.origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let arithmetic = 65536. * f64::EPSILON * world
            + 8. * gram * (norm(start - frame.origin()) + length + scale);
        if !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. {
            return Err(unresolved());
        }
        let gaps = [normal.dot(start - origin), normal.dot(end - origin)];
        let slope = normal.dot(delta);
        if gaps.iter().any(|g| !g.is_finite()) || !slope.is_finite() {
            return Err(unresolved());
        }
        // The original source checks the original endpoints and all quartic roots
        // before empty/removed-side shortcuts; no cropped surrogate hides precision.
        let source = self.source().intersect_segment(start, end, policy)?;
        let mut material_intervals = Vec::new();
        let mut cut_root = None;
        for mut interval in source.material_intervals.iter().copied() {
            if slope.abs() <= 2. * arithmetic {
                if gaps.iter().all(|g| *g < -band - arithmetic) {
                } else if gaps.iter().all(|g| *g > band + arithmetic) {
                    continue;
                } else {
                    return Err(unresolved());
                }
            } else {
                let t = -gaps[0] / slope;
                if !t.is_finite() {
                    return Err(unresolved());
                }
                if t > interval[0] && t < interval[1] {
                    if arithmetic / (slope.abs() - 2. * arithmetic) * length >= policy.linear() / 8.
                        || (t - interval[0]) * length <= 10. * band
                        || (interval[1] - t) * length <= 10. * band
                    {
                        return Err(unresolved());
                    }
                    let point = start + delta * t;
                    if self.source().classify_point(point, policy)? != PointLocation::Inside {
                        return Err(unresolved());
                    }
                    let query = self.point_query(point, policy)?;
                    if query.boundary.as_ref().is_none_or(|w| w.face != cap_id) {
                        return Err(unresolved());
                    }
                    if slope > 0. {
                        interval[1] = t;
                    } else {
                        interval[0] = t;
                    }
                    cut_root = Some(t);
                } else {
                    let endpoint_gaps = interval.map(|t| normal.dot(start + delta * t - origin));
                    if endpoint_gaps.iter().all(|g| *g > band + arithmetic) {
                        continue;
                    }
                    if !endpoint_gaps.iter().all(|g| *g < -band - arithmetic) {
                        return Err(unresolved());
                    }
                }
            }
            material_intervals.push(interval);
        }
        let mut hits = Vec::new();
        for interval in &material_intervals {
            if (interval[1] - interval[0]) * length <= 10. * band
                || self.classify_point(
                    start + delta * (interval[0] + (interval[1] - interval[0]) * 0.5),
                    policy,
                )? != PointLocation::Inside
            {
                return Err(unresolved());
            }
            for &t in interval {
                if t == 0. || t == 1. {
                    continue;
                }
                let point = start + delta * t;
                let actual = self.point_query(point, policy)?;
                let mut boundary = actual.boundary.ok_or_else(unresolved)?;
                if cut_root == Some(t) {
                    if boundary.face != cap_id {
                        return Err(unresolved());
                    }
                } else {
                    let original = source
                        .hits
                        .iter()
                        .find(|h| h.parameter == t)
                        .ok_or_else(unresolved)?;
                    let minor_quarter = original.boundary.face % 4;
                    let major_quarter = original.boundary.face / 4;
                    let uv = original.boundary.uv;
                    let strip = self
                        .strip_ranges()
                        .iter()
                        .position(|(q, [a, b])| *q == minor_quarter && uv[1] >= *a && uv[1] <= *b)
                        .ok_or_else(unresolved)?;
                    let face = major_quarter * n + strip;
                    if boundary.face != face {
                        return Err(unresolved());
                    }
                    let surface = &self.solid().shell.faces[face].surface;
                    boundary = NurbsTorusBoundaryWitness {
                        face,
                        uv,
                        point: surface.try_evaluate(uv[0], uv[1])?,
                        distance: 0.,
                        normal: surface.normal_at(uv[0], uv[1])?,
                    };
                    boundary.distance = norm(point - boundary.point);
                }
                if boundary.distance + arithmetic >= policy.linear() / 4. {
                    return Err(unresolved());
                }
                let crossing = boundary.normal.dot(delta);
                let directional =
                    crossing.abs() / length - 64. * arithmetic / minor.min(major - minor);
                let forward_error = (boundary.distance + arithmetic) / directional;
                if !directional.is_finite()
                    || directional <= 0.
                    || !forward_error.is_finite()
                    || forward_error >= policy.linear() / 4.
                {
                    return Err(Error::Unsupported(
                        "horizontal torus segment forward crossing precision is unresolved",
                    ));
                }

                if !crossing.is_finite()
                    || crossing.abs() <= 256. * f64::EPSILON * length + arithmetic
                    || (crossing < 0.) != (t == interval[0])
                {
                    return Err(unresolved());
                }
                hits.push(NurbsTorusSegmentHit {
                    parameter: t,
                    point,
                    boundary,
                });
            }
        }
        hits.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
        if hits.len() > 4
            || material_intervals.len() > 2
            || hits
                .windows(2)
                .any(|w| (w[1].parameter - w[0].parameter) * length <= 10. * band)
            || material_intervals.windows(2).any(|w| w[0][1] >= w[1][0])
        {
            return Err(unresolved());
        }
        if (locations[0] == PointLocation::Inside) != material_intervals.iter().any(|v| v[0] == 0.)
            || (locations[1] == PointLocation::Inside)
                != material_intervals.iter().any(|v| v[1] == 1.)
        {
            return Err(unresolved());
        }
        Ok(NurbsTorusSegmentIntersection {
            hits,
            material_intervals,
        })
    }
}
