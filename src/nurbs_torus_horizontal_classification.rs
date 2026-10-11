//! Finite arc and annular-cap distances for actual horizontal torus children.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "horizontal torus point distance, physical trim rim or query precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsTorusHorizontalSolid {
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        Ok(self.point_query(point, policy)?.location)
    }
    pub fn point_query(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusPointQuery> {
        self.validate(policy)?;
        // Original source point query establishes world/frame precision before any
        // finite-child shortcuts, and supplies a retained surface witness if needed.
        let source_query = self.source().point_query(point, policy)?;
        let frame = self.source().frame();
        let [major, minor] = self.source().radii();
        let cut = self.cut_height();
        let local = frame.local_point(point);
        let rho = local.x.hypot(local.y);
        let tube = (rho - major).hypot(local.z);
        let scale = 2. * (major + minor).hypot(minor);
        let band = policy.length_at_scale(scale)?;
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let world = [point, frame.origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let error = 65536. * f64::EPSILON * world + 8. * gram * norm(point - frame.origin());
        if !local.finite()
            || !rho.is_finite()
            || !tube.is_finite()
            || !error.is_finite()
            || error >= policy.linear() / 8.
        {
            return Err(unresolved());
        }
        let half = minor * ((1. - cut / minor) * (1. + cut / minor)).sqrt();
        let inner = major - half;
        let outer = major + half;
        let cap_distance = (rho - rho.clamp(inner, outer)).hypot(local.z - cut);
        let foot_z = if tube > 0. {
            minor * (local.z / tube)
        } else {
            if self.is_lower() {
                -minor
            } else {
                minor
            }
        };
        let retained = if self.is_lower() {
            foot_z <= cut
        } else {
            foot_z >= cut
        };
        let wall_distance = if retained {
            (tube - minor).abs()
        } else {
            (rho - inner)
                .hypot(local.z - cut)
                .min((rho - outer).hypot(local.z - cut))
        };
        let distance = wall_distance.min(cap_distance);
        if !distance.is_finite() {
            return Err(unresolved());
        }
        if distance - error > band {
            let inside = tube < minor
                && if self.is_lower() {
                    local.z < cut
                } else {
                    local.z > cut
                };
            return Ok(NurbsTorusPointQuery {
                location: if inside {
                    PointLocation::Inside
                } else {
                    PointLocation::Outside
                },
                boundary: None,
            });
        }
        if distance + error > band {
            return Err(unresolved());
        }
        let n = self.strip_ranges().len();
        if wall_distance + error <= band {
            if let Some(mut witness) = source_query.boundary {
                let foot = frame.local_point(witness.point);
                let clearance = if self.is_lower() {
                    cut - foot.z
                } else {
                    foot.z - cut
                };
                if clearance > 10. * band + error {
                    let major_quarter = witness.face / 4;
                    let minor_quarter = witness.face % 4;
                    let k = self
                        .strip_ranges()
                        .iter()
                        .position(|(q, [a, b])| {
                            *q == minor_quarter && witness.uv[1] >= *a && witness.uv[1] <= *b
                        })
                        .ok_or_else(unresolved)?;
                    witness.face = major_quarter * n + k;
                    let actual = self.solid().shell.faces[witness.face]
                        .surface
                        .try_evaluate(witness.uv[0], witness.uv[1])?;
                    witness.point = actual;
                    witness.distance = norm(point - actual);
                    witness.normal = self.solid().shell.faces[witness.face]
                        .surface
                        .normal_at(witness.uv[0], witness.uv[1])?;
                    if witness.distance + error <= band {
                        return Ok(NurbsTorusPointQuery {
                            location: PointLocation::Boundary,
                            boundary: Some(witness),
                        });
                    }
                }
            }
        }
        if cap_distance + error <= band {
            let face = 4 * n;
            let cap = &self.solid().shell.faces[face];
            let Surface::Plane { origin, u, v } = cap.surface else {
                return Err(unresolved());
            };
            let normal = cap.surface.normal_at(0., 0.)? * f64::from(cap.orientation);
            let foot = point - normal * (point - origin).dot(normal);
            let foot_local = frame.local_point(foot);
            let radius = foot_local.x.hypot(foot_local.y);
            if radius - inner > 10. * band + error
                && outer - radius > 10. * band + error
                && self.source().classify_point(foot, policy)? == PointLocation::Inside
            {
                let delta = foot - origin;
                let a = u.dot(u);
                let b = u.dot(v);
                let d = v.dot(v);
                let det = a * d - b * b;
                if !det.is_normal() || det <= 0. {
                    return Err(unresolved());
                }
                let x = delta.dot(u);
                let y = delta.dot(v);
                let uv = [(d * x - b * y) / det, (a * y - b * x) / det];
                let actual = cap.surface.try_evaluate(uv[0], uv[1])?;
                let distance = norm(point - actual);
                if actual.finite()
                    && normal.finite()
                    && distance.is_finite()
                    && distance + error <= band
                {
                    return Ok(NurbsTorusPointQuery {
                        location: PointLocation::Boundary,
                        boundary: Some(NurbsTorusBoundaryWitness {
                            face,
                            uv,
                            point: actual,
                            distance,
                            normal,
                        }),
                    });
                }
            }
        }
        Err(unresolved())
    }
}
