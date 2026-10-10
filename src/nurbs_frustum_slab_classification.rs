//! Conservative finite-face Euclidean-band queries for retained parallel slabs.
use crate::*;

fn unresolved() -> Error {
    Error::Unsupported("frustum slab classification is unresolved near a finite cut rim, retained cap witness or boundary threshold")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
struct Cut {
    origin: Point3,
    outward: Vec3,
    gap: f64,
}
fn classify(
    query: Point3,
    policy: GeometryTolerance,
    source: &NurbsFrustumSolid,
    solid: &Solid,
    present: [bool; 2],
    source_query: impl Fn(Point3) -> Result<PointLocation>,
) -> Result<PointLocation> {
    // Applies the ORIGINAL finite source's frame, world and relative policy
    // before either subset-distance lower bound is used.
    let source_location = source_query(query)?;
    let [r0, r1] = source.radii();
    let scale = (2. * r0.max(r1)).hypot(source.height());
    let band = policy.length_at_scale(scale)?;
    let mut world = [query, source.frame().origin()]
        .into_iter()
        .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
    let mut cuts = Vec::with_capacity(2);
    for (face_id, &is_present) in present.iter().enumerate() {
        if !is_present {
            continue;
        }
        let face = &solid.shell.faces[face_id];
        let Surface::Plane { origin, u, v } = face.surface else {
            return Err(unresolved());
        };
        let outward = u.cross(v).normalized()? * f64::from(face.orientation);
        let delta = query - origin;
        let gap = -outward.dot(delta);
        if !delta.finite() || !norm(delta).is_finite() || !gap.is_finite() {
            return Err(unresolved());
        }
        world = world
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        cuts.push(Cut {
            origin,
            outward,
            gap,
        });
    }
    let allowance = 65536. * f64::EPSILON * world;
    if !allowance.is_finite() || allowance >= policy.linear() / 8. {
        return Err(Error::Unsupported(
            "frustum slab query world-coordinate precision is unresolved",
        ));
    }
    // Both lower bounds apply to the actual subset. They are not a formula
    // for a Euclidean corner distance or an intersection of band labels.
    if source_location == PointLocation::Outside
        || cuts.iter().any(|cut| cut.gap + allowance < -band)
    {
        return Ok(PointLocation::Outside);
    }
    if source_location == PointLocation::Inside && cuts.iter().all(|cut| cut.gap - allowance > band)
    {
        return Ok(PointLocation::Inside);
    }
    if source_location == PointLocation::Boundary
        && cuts.iter().all(|cut| cut.gap - allowance > 10. * band)
    {
        // A finite source witness within band cannot cross either cut plane.
        return Ok(PointLocation::Boundary);
    }
    for (index, cut) in cuts.iter().enumerate() {
        if cut.gap.abs() + allowance > band {
            continue;
        }
        let foot = query + cut.outward * cut.gap;
        if !foot.finite() {
            return Err(unresolved());
        }
        let foot_world = world.max(foot.x.abs()).max(foot.y.abs()).max(foot.z.abs());
        let foot_allowance = 65536. * f64::EPSILON * foot_world;
        if !foot_allowance.is_finite() || foot_allowance >= policy.linear() / 8. {
            return Err(unresolved());
        }
        // The plane foot is an actual finite disk/annulus witness only when
        // the other cut also retains it. A finite witness in a thin slab
        // cannot be decided from an axial gap or from the query's own signs.
        let other_cuts_retain = cuts.iter().enumerate().all(|(other_index, other)| {
            other_index == index || -other.outward.dot(foot - other.origin) > foot_allowance
        });
        if other_cuts_retain && source_query(foot)? == PointLocation::Inside {
            return Ok(PointLocation::Boundary);
        }
    }
    Err(unresolved())
}
impl NurbsFrustumSlabSolid {
    /// Query the actual finite retained faces. Unresolved cut rims and finite
    /// cap witnesses refuse; relative tolerance uses the original source extent.
    pub fn classify_point(
        &self,
        query: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        classify(
            query,
            policy,
            self.source(),
            self.solid(),
            [
                self.lower_section().is_some(),
                self.upper_section().is_some(),
            ],
            |point| self.source().classify_point(point, policy),
        )
    }
}
impl NurbsBoredFrustumSlabSolid {
    /// Query actual annular caps and inward bore walls, without filling the void.
    pub fn classify_point(
        &self,
        query: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        classify(
            query,
            policy,
            self.source().source(),
            self.solid(),
            [
                self.outer().lower_section().is_some(),
                self.outer().upper_section().is_some(),
            ],
            |point| self.source().classify_point(point, policy),
        )
    }
}
