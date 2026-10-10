//! Euclidean finite-boundary classification of exact coaxial annular frusta.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported("bored frustum query distance or world/frame precision is unresolved")
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn boundary_distance(rho: f64, z: f64, radii: [f64; 2], height: f64, hole: f64) -> Result<f64> {
    let dr = radii[1] - radii[0];
    let span = dr.hypot(height);
    let t = ((rho - radii[0]) / span * (dr / span) + z / span * (height / span)).clamp(0., 1.);
    let outer_radius = radii[0] * (1. - t) + radii[1] * t;
    let outer = (rho - outer_radius).hypot(z - height * t);
    let inner = (rho - hole).hypot(z - z.clamp(0., height));
    let bottom = (hole - rho).max(rho - radii[0]).max(0.).hypot(z);
    let top = (hole - rho).max(rho - radii[1]).max(0.).hypot(z - height);
    let distance = outer.min(inner).min(bottom).min(top);
    if !t.is_finite() || !distance.is_finite() {
        return Err(unresolved());
    }
    Ok(distance)
}
impl NurbsBoredFrustumSolid {
    /// Classify against the actual finite outer wall, inner wall and annular
    /// caps. Distances include diagonal rim distances; no mesh or plane-extension
    /// approximation determines the location. Relative policy uses source extent.
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        if !point.finite() {
            return Err(Error::InvalidInput(
                "bored frustum query point must be finite",
            ));
        }
        let frame = self.source().frame();
        let radii = self.source().radii();
        let height = self.source().height();
        let delta = point - frame.origin();
        let distance = length(delta);
        if !delta.finite() || !distance.is_finite() {
            return Err(unresolved());
        }
        let axes = frame.axes();
        let mut metric_error = 0f64;
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1. } else { 0. };
                metric_error = metric_error.max((axes[i].dot(axes[j]) - expected).abs());
            }
        }
        let world = [point, frame.origin()]
            .into_iter()
            .fold(radii[0].max(radii[1]).max(height), |s, p| {
                s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs())
            });
        let allowance = 32768. * f64::EPSILON * world + 4. * metric_error * distance;
        if !allowance.is_finite() || allowance >= policy.linear() / 8. {
            return Err(unresolved());
        }
        let local = frame.local_point(point);
        let rho = local.x.hypot(local.y);
        if !local.finite() || !rho.is_finite() {
            return Err(unresolved());
        }
        let scale = (2. * radii[0].max(radii[1])).hypot(height);
        let band = policy.length_at_scale(scale)?;
        let boundary = boundary_distance(rho, local.z, radii, height, self.radius())?;
        if boundary + allowance <= band {
            return Ok(PointLocation::Boundary);
        }
        if boundary - allowance <= band {
            return Err(unresolved());
        }
        if local.z > 0. && local.z < height {
            let t = local.z / height;
            let outer = radii[0] * (1. - t) + radii[1] * t;
            if rho > self.radius() && rho < outer {
                return Ok(PointLocation::Inside);
            }
        }
        Ok(PointLocation::Outside)
    }
}
