//! Finite two-arc and two-annular-cap Euclidean queries on actual torus bands.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "torus slab point distance, physical trim rim or query precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn annulus([major, minor]: [f64; 2], height: f64) -> [f64; 2] {
    let half = minor * (((minor - height) / minor) * ((minor + height) / minor)).sqrt();
    [major - half, major + half]
}
impl NurbsTorusHorizontalSlab {
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        Ok(self.point_query(point, policy)?.location)
    }
    /// Boundary witnesses exclude unresolved physical rims; quarter patch seams
    /// retain deterministic original UVs and actual child face provenance.
    pub fn point_query(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusPointQuery> {
        self.validate(policy)?;
        let original = self.source().point_query(point, policy)?;
        let frame = self.source().frame();
        let radii = self.source().radii();
        let [major, minor] = radii;
        let [lo, hi] = self.heights();
        let chains = self.chain_ranges();
        let caps = 4 * (chains[0].len() + chains[1].len());
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let scale = 2. * (major + minor).hypot(minor);
        let band = policy.length_at_scale(scale)?;
        let mut world = [point, frame.origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        for face in &self.solid().shell.faces[caps..] {
            let Surface::Plane { origin, .. } = face.surface else {
                return Err(unresolved());
            };
            world = world
                .max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs());
        }
        let error =
            65536. * f64::EPSILON * world + 8. * gram * (norm(point - frame.origin()) + scale);
        if !error.is_finite() || error >= policy.linear() / 8. {
            return Err(unresolved());
        }
        let local = frame.local_point(point);
        let rho = local.x.hypot(local.y);
        let tube = (rho - major).hypot(local.z);
        if !local.finite() || !rho.is_finite() || !tube.is_finite() {
            return Err(unresolved());
        }
        let rings = [annulus(radii, lo), annulus(radii, hi)];
        let cap_distances = [
            (rho - rho.clamp(rings[0][0], rings[0][1])).hypot(local.z - lo),
            (rho - rho.clamp(rings[1][0], rings[1][1])).hypot(local.z - hi),
        ];
        let foot_z = if tube > 0. {
            minor * (local.z / tube)
        } else {
            lo + (hi - lo) * 0.5
        };
        let wall_distance = if foot_z >= lo && foot_z <= hi {
            (tube - minor).abs()
        } else {
            rings
                .iter()
                .zip([lo, hi])
                .flat_map(|(r, z)| r.map(|radius| (rho - radius).hypot(local.z - z)))
                .fold(f64::INFINITY, f64::min)
        };
        let distance = wall_distance.min(cap_distances[0]).min(cap_distances[1]);
        if !distance.is_finite() {
            return Err(unresolved());
        }
        if distance - error > band {
            let inside = tube < minor && local.z > lo && local.z < hi;
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
        if wall_distance + error <= band {
            if let Some(mut witness) = original.boundary {
                let foot = frame.local_point(witness.point);
                if foot.z - lo > 10. * band + error && hi - foot.z > 10. * band + error {
                    let major_quarter = witness.face / 4;
                    let minor_quarter = witness.face % 4;
                    let mut mapped = None;
                    let mut offset = 0;
                    for chain in chains {
                        if let Some(k) = chain.iter().position(|(q, [a, b])| {
                            *q == minor_quarter && witness.uv[1] >= *a && witness.uv[1] <= *b
                        }) {
                            mapped = Some(offset + major_quarter * chain.len() + k);
                            break;
                        }
                        offset += 4 * chain.len();
                    }
                    witness.face = mapped.ok_or_else(unresolved)?;
                    let face = &self.solid().shell.faces[witness.face];
                    witness.point = face.surface.try_evaluate(witness.uv[0], witness.uv[1])?;
                    witness.normal = face.surface.normal_at(witness.uv[0], witness.uv[1])?
                        * f64::from(face.orientation);
                    witness.distance = norm(point - witness.point);
                    if witness.distance.is_finite() && witness.distance + error <= band {
                        return Ok(NurbsTorusPointQuery {
                            location: PointLocation::Boundary,
                            boundary: Some(witness),
                        });
                    }
                }
            }
        }
        for cap in 0..2 {
            if cap_distances[cap] + error > band {
                continue;
            }
            let face_id = caps + cap;
            let face = &self.solid().shell.faces[face_id];
            let Surface::Plane { origin, u, v } = face.surface else {
                return Err(unresolved());
            };
            let normal = face.surface.normal_at(0., 0.)? * f64::from(face.orientation);
            let foot = point - normal * (point - origin).dot(normal);
            let local = frame.local_point(foot);
            let radius = local.x.hypot(local.y);
            let other = &self.solid().shell.faces[caps + 1 - cap];
            let Surface::Plane {
                origin: other_origin,
                ..
            } = other.surface
            else {
                return Err(unresolved());
            };
            let other_normal = other.surface.normal_at(0., 0.)? * f64::from(other.orientation);
            let other_gap = other_normal.dot(foot - other_origin);
            if radius - rings[cap][0] <= 10. * band + error
                || rings[cap][1] - radius <= 10. * band + error
                || !other_gap.is_finite()
                || other_gap + error >= 0.
                || self.source().classify_point(foot, policy)? != PointLocation::Inside
            {
                continue;
            }
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
            let actual = face.surface.try_evaluate(uv[0], uv[1])?;
            let distance = norm(point - actual);
            if actual.finite()
                && normal.finite()
                && distance.is_finite()
                && distance + error <= band
            {
                return Ok(NurbsTorusPointQuery {
                    location: PointLocation::Boundary,
                    boundary: Some(NurbsTorusBoundaryWitness {
                        face: face_id,
                        uv,
                        point: actual,
                        distance,
                        normal,
                    }),
                });
            }
        }
        Err(unresolved())
    }
}
