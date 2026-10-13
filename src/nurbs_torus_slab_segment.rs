//! Original finite source intervals clipped by both actual annular slab caps.
//! Discarded source tangencies and contacts may conservatively remain unsupported.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "torus slab segment cap contact, physical rim or crossing precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
struct Cut {
    face: usize,
    origin: Point3,
    normal: Vec3,
    gaps: [f64; 2],
    slope: f64,
}
impl NurbsTorusHorizontalSlab {
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusSegmentIntersection> {
        self.validate(policy)?;
        if !start.finite() || !end.finite() || start == end {
            return Err(Error::InvalidInput(
                "torus slab segment endpoints must be finite and distinct",
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
                "torus slab segment endpoint is within the boundary band; move it away",
            ));
        }
        let [major, minor] = self.source().radii();
        let scale = 2. * (major + minor).hypot(minor);
        let band = policy.length_at_scale(scale)?;
        if length <= 10. * band {
            return Err(unresolved());
        }
        let frame = self.source().frame();
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let chains = self.chain_ranges();
        let cap_start = 4 * (chains[0].len() + chains[1].len());
        let mut cuts = Vec::new();
        let mut world = [start, end, frame.origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        for face_id in cap_start..cap_start + 2 {
            let face = &self.solid().shell.faces[face_id];
            let Surface::Plane { origin, .. } = face.surface else {
                return Err(unresolved());
            };
            let normal = face.surface.normal_at(0., 0.)? * f64::from(face.orientation);
            world = world
                .max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs());
            let gaps = [normal.dot(start - origin), normal.dot(end - origin)];
            let slope = normal.dot(delta);
            if !normal.finite() || gaps.iter().any(|v| !v.is_finite()) || !slope.is_finite() {
                return Err(unresolved());
            }
            cuts.push(Cut {
                face: face_id,
                origin,
                normal,
                gaps,
                slope,
            });
        }
        let arithmetic = 65536. * f64::EPSILON * world
            + 8. * gram * (norm(start - frame.origin()) + length + scale);
        if !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. {
            return Err(unresolved());
        }
        // Original source validation and original endpoint/root precision precede
        // empty/removed shortcuts; no surrogate segment discards unresolved roots.
        let source = self.source().intersect_segment(start, end, policy)?;
        let mut intervals = Vec::new();
        let mut roots = [None; 2];
        for original in &source.material_intervals {
            let mut interval = *original;
            let mut removed = false;
            for (index, cut) in cuts.iter().enumerate() {
                if cut.slope.abs() <= 2. * arithmetic {
                    if cut.gaps.iter().all(|g| *g < -band - arithmetic) {
                    } else if cut.gaps.iter().all(|g| *g > band + arithmetic) {
                        removed = true;
                        break;
                    } else {
                        return Err(unresolved());
                    }
                } else {
                    let t = -cut.gaps[0] / cut.slope;
                    if !t.is_finite() {
                        return Err(unresolved());
                    }
                    if t > interval[0] && t < interval[1] {
                        if arithmetic / (cut.slope.abs() - 2. * arithmetic) * length
                            >= policy.linear() / 8.
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
                        if query.boundary.as_ref().is_none_or(|w| w.face != cut.face) {
                            return Err(unresolved());
                        }
                        if cut.slope > 0. {
                            interval[1] = t;
                        } else {
                            interval[0] = t;
                        }
                        roots[index] = Some(t);
                    } else {
                        let endpoint_gaps =
                            interval.map(|t| cut.normal.dot(start + delta * t - cut.origin));
                        if endpoint_gaps.iter().all(|g| *g > band + arithmetic) {
                            removed = true;
                            break;
                        }
                        if !endpoint_gaps.iter().all(|g| *g < -band - arithmetic) {
                            return Err(unresolved());
                        }
                    }
                }
            }
            if !removed {
                intervals.push(interval);
            }
        }
        let mut hits = Vec::new();
        for interval in &intervals {
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
                let query = self.point_query(point, policy)?;
                let mut boundary = query.boundary.ok_or_else(unresolved)?;
                let cut_matches: Vec<_> = roots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| (*r == Some(t)).then_some(i))
                    .collect();
                if cut_matches.len() > 1 {
                    return Err(unresolved());
                }
                if let Some(&index) = cut_matches.first() {
                    if boundary.face != cuts[index].face {
                        return Err(unresolved());
                    }
                } else {
                    let original = source
                        .hits
                        .iter()
                        .find(|h| h.parameter == t)
                        .ok_or_else(unresolved)?;
                    let q = original.boundary.face % 4;
                    let major_quarter = original.boundary.face / 4;
                    let uv = original.boundary.uv;
                    let mut offset = 0;
                    let mut mapped = None;
                    for chain in chains {
                        if let Some(k) = chain.iter().position(|(quarter, [a, b])| {
                            *quarter == q && uv[1] >= *a && uv[1] <= *b
                        }) {
                            mapped = Some(offset + major_quarter * chain.len() + k);
                            break;
                        }
                        offset += 4 * chain.len();
                    }
                    let face = mapped.ok_or_else(unresolved)?;
                    if boundary.face != face {
                        return Err(unresolved());
                    }
                    let surface = &self.solid().shell.faces[face].surface;
                    let actual = surface.try_evaluate(uv[0], uv[1])?;
                    boundary = NurbsTorusBoundaryWitness {
                        face,
                        uv,
                        point: actual,
                        distance: norm(point - actual),
                        normal: surface.normal_at(uv[0], uv[1])?,
                    };
                }
                if !boundary.distance.is_finite()
                    || boundary.distance + arithmetic >= policy.linear() / 4.
                {
                    return Err(unresolved());
                }
                let crossing = boundary.normal.dot(delta);
                let directional =
                    crossing.abs() / length - 64. * arithmetic / minor.min(major - minor);
                let forward = (boundary.distance + arithmetic) / directional;
                if !directional.is_finite()
                    || directional <= 0.
                    || !forward.is_finite()
                    || forward >= policy.linear() / 4.
                    || !crossing.is_finite()
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
            || intervals.len() > 2
            || hits
                .windows(2)
                .any(|w| (w[1].parameter - w[0].parameter) * length <= 10. * band)
            || intervals.windows(2).any(|w| w[0][1] >= w[1][0])
        {
            return Err(unresolved());
        }
        if (locations[0] == PointLocation::Inside) != intervals.iter().any(|i| i[0] == 0.)
            || (locations[1] == PointLocation::Inside) != intervals.iter().any(|i| i[1] == 1.)
        {
            return Err(unresolved());
        }
        Ok(NurbsTorusSegmentIntersection {
            hits,
            material_intervals: intervals,
        })
    }
}
