//! Conservative Euclidean-band classification of retained oblique frustum pieces.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported("bored oblique frustum classification is unresolved near the finite cut ellipse, shared rim or boundary threshold")
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsBoredObliqueFrustumSolid {
    /// Classify the actual finite retained body. Boundary success needs a
    /// retained finite-face witness; ambiguous shared-rim distances reject.
    /// Relative tolerance uses the source's local extent, never query distance.
    pub fn classify_point(
        &self,
        query: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        // This checked source query applies its original world/frame precision
        // policy before any subset or halfspace shortcut can return a result.
        let source_location = self.source().classify_point(query, policy)?;
        let cap_index = if self.is_lower() { 1 } else { 0 };
        let Surface::Plane { origin, u, v } = self.solid().shell.faces[cap_index].surface else {
            return Err(unresolved());
        };
        let normal = u.cross(v).normalized()?;
        let delta = query - origin;
        let distance = norm(delta);
        let [r0, r1] = self.source().source().radii();
        let scale = (2. * r0.max(r1)).hypot(self.source().source().height());
        let band = policy.length_at_scale(scale)?;
        let world = [query, origin, self.source().source().frame().origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let allowance = 65536. * f64::EPSILON * world;
        if !delta.finite()
            || !distance.is_finite()
            || !allowance.is_finite()
            || allowance >= policy.linear() / 8.
        {
            return Err(Error::Unsupported(
                "bored oblique frustum query world-coordinate precision is unresolved",
            ));
        }
        let plane_gap = normal.dot(delta);
        if !plane_gap.is_finite() {
            return Err(unresolved());
        }
        let retained_gap = if self.is_lower() {
            -plane_gap
        } else {
            plane_gap
        };
        // The child is a subset of the source, and of its retained halfspace.
        // These are distance lower bounds, not a finite-corner distance formula.
        if source_location == PointLocation::Outside || retained_gap + allowance < -band {
            return Ok(PointLocation::Outside);
        }
        if source_location == PointLocation::Inside && retained_gap - allowance > band {
            return Ok(PointLocation::Inside);
        }
        if source_location == PointLocation::Boundary && retained_gap - allowance > 10. * band {
            // A nearest source boundary point is at most band away; this much
            // halfspace clearance proves that original boundary is retained.
            return Ok(PointLocation::Boundary);
        }
        if plane_gap.abs() + allowance <= band {
            let foot = query - normal * plane_gap;
            if !foot.finite() {
                return Err(unresolved());
            }
            if self.source().classify_point(foot, policy)? == PointLocation::Inside {
                // The orthogonal foot lies inside the original finite annular frustum,
                // hence in the actual annular ellipse cap of this retained planar cap.
                return Ok(PointLocation::Boundary);
            }
        }
        Err(unresolved())
    }
}
