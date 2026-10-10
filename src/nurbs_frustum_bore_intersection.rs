//! Checked finite segment intervals in actual annular frustum material.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsBoredFrustumSegmentIntersection {
    pub hits: Vec<NurbsFrustumSegmentHit>,
    pub material_intervals: Vec<[f64; 2]>,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "bored frustum segment has unresolved contact, root separation or retained-face provenance",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsBoredFrustumSolid {
    /// Subtract the checked cylindrical interval from the checked source
    /// interval. Boundary endpoints, tangencies and ambiguous rims reject.
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumSegmentIntersection> {
        self.validate(policy)?;
        if !start.finite() || !end.finite() || start == end {
            return Err(Error::InvalidInput(
                "segment endpoints must be finite and distinct",
            ));
        }
        let delta = end - start;
        let length = norm(delta);
        if !delta.finite() || !length.is_normal() {
            return Err(unresolved());
        }
        let endpoint_locations = [
            self.classify_point(start, policy)?,
            self.classify_point(end, policy)?,
        ];
        if endpoint_locations.contains(&PointLocation::Boundary) {
            return Err(Error::Unsupported(
                "bored segment endpoint is within the boundary band; move it away",
            ));
        }
        let [a, b] = self.source().radii();
        let source_scale = (2. * a.max(b)).hypot(self.source().height());
        let band = policy.length_at_scale(source_scale)?;
        if length <= 10. * band {
            return Err(unresolved());
        }
        // The physical disk is convex. Guarded strictly interior radial
        // endpoints prove the entire segment stays in the open cylindrical
        // void, independently of cap root rounding in the two solvers.
        let frame = self.source().frame();
        let void = [start, end].into_iter().all(|p| {
            let local = frame.local_point(p);
            local.x.hypot(local.y) + band + policy.linear() / 4. < self.radius()
        });
        if void {
            return Ok(NurbsBoredFrustumSegmentIntersection {
                hits: vec![],
                material_intervals: vec![],
            });
        }
        let source = self.source().intersect_segment(start, end, policy)?;
        // Retain the caller's absolute root budget, while choosing relative
        // tolerance to give the cylinder exactly the source's effective band.
        let tool_scale = (2. * self.radius()).hypot(self.source().height());
        let tool_policy =
            GeometryTolerance::new(policy.linear(), policy.angular(), band / tool_scale)?;
        let tool = self.tool().intersect_segment(start, end, tool_policy)?;
        let mut material_intervals = vec![];
        if let Some([lo, hi]) = source.material_interval {
            if let Some([tl, th]) = tool.material_interval {
                if th <= lo || tl >= hi {
                    material_intervals.push([lo, hi]);
                } else {
                    if tl > lo {
                        material_intervals.push([lo, tl]);
                    }
                    if th < hi {
                        material_intervals.push([th, hi]);
                    }
                }
            } else {
                material_intervals.push([lo, hi]);
            }
        }
        let mut hits = vec![];
        for interval in &material_intervals {
            if (interval[1] - interval[0]) * length <= 10. * band {
                return Err(unresolved());
            }
            let middle = start + delta * (interval[0] + (interval[1] - interval[0]) / 2.);
            if self.classify_point(middle, policy)? != PointLocation::Inside {
                return Err(unresolved());
            }
            for &t in interval {
                if t == 0. || t == 1. {
                    continue;
                }
                let mut candidates = source
                    .hits
                    .iter()
                    .filter(|h| h.parameter == t)
                    .cloned()
                    .collect::<Vec<_>>();
                candidates.extend(tool.hits.iter().filter(|h| h.parameter == t).cloned().map(
                    |mut h| {
                        h.faces.retain(|f| f.face_id >= 2);
                        for f in &mut h.faces {
                            f.face_id += 4;
                        }
                        h
                    },
                ));
                if candidates.len() != 1 || candidates[0].faces.is_empty() {
                    return Err(unresolved());
                }
                let mut hit = candidates.remove(0);
                hit.point = start + delta * t;
                if self.classify_point(hit.point, policy)? != PointLocation::Boundary {
                    return Err(unresolved());
                }
                for witness in &hit.faces {
                    let face = &self.solid().shell.faces[witness.face_id];
                    let normal = face.surface.normal_at(witness.uv[0], witness.uv[1])?
                        * f64::from(face.orientation);
                    let crossing = normal.dot(delta);
                    let entering = t == interval[0];
                    if !crossing.is_finite()
                        || crossing.abs() <= 256. * f64::EPSILON * length + policy.linear() / 8.
                        || (crossing < 0.) != entering
                    {
                        return Err(unresolved());
                    }
                    let represented = face.surface.try_evaluate(witness.uv[0], witness.uv[1])?;
                    let error = norm(represented - hit.point);
                    if !error.is_finite() || error >= policy.linear() / 4. {
                        return Err(unresolved());
                    }
                }
                hits.push(hit);
            }
        }
        hits.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
        if hits
            .windows(2)
            .any(|w| (w[1].parameter - w[0].parameter) * length <= 10. * band)
        {
            return Err(unresolved());
        }
        for (t, location) in [0., 1.].into_iter().zip(endpoint_locations) {
            let covered = material_intervals.iter().any(|r| r[0] <= t && t <= r[1]);
            if covered != (location == PointLocation::Inside) {
                return Err(unresolved());
            }
        }
        Ok(NurbsBoredFrustumSegmentIntersection {
            hits,
            material_intervals,
        })
    }
}
