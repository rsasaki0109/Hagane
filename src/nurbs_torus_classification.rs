//! Mesh-free finite Euclidean classification with actual rational face witnesses.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusBoundaryWitness {
    pub face: usize,
    pub uv: [f64; 2],
    pub point: Point3,
    pub distance: f64,
    pub normal: Vec3,
}
#[derive(Clone, Debug)]
pub struct NurbsTorusPointQuery {
    pub location: PointLocation,
    pub boundary: Option<NurbsTorusBoundaryWitness>,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "torus point distance or actual boundary witness is unresolved at caller tolerance",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn quarter(x: f64, y: f64, allowance: f64) -> Result<(usize, f64)> {
    // A physical seam displacement below the existing coordinate arithmetic
    // reserve has no stable angular side. Choose its literal quarter start;
    // the final actual surface-distance test still certifies the witness.
    if y.abs() <= allowance {
        return Ok((if x >= 0. { 0 } else { 2 }, 0.));
    }
    if x.abs() <= allowance {
        return Ok((if y >= 0. { 1 } else { 3 }, 0.));
    }
    let angle = y.atan2(x);
    let angle = if angle < 0. {
        angle + std::f64::consts::TAU
    } else {
        angle
    };
    let q = ((angle / std::f64::consts::FRAC_PI_2).floor() as usize).min(3);
    let phase = angle - q as f64 * std::f64::consts::FRAC_PI_2;
    let k = (phase / 2.).tan();
    let root = std::f64::consts::SQRT_2;
    let t = root * k / (1. + (root - 1.) * k);
    if !t.is_finite() || !(0. ..=1.).contains(&t) {
        return Err(unresolved());
    }
    Ok((q, t))
}
impl NurbsTorusSolid {
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        Ok(self.point_query(point, policy)?.location)
    }
    /// Euclidean classification of the regular ring body. Boundary success
    /// includes a point and normal evaluated on the retained rational surface.
    /// Arithmetic allowances are engineering guards, not interval certificates.
    pub fn point_query(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusPointQuery> {
        self.validate(policy)?;
        if !point.finite() {
            return Err(Error::InvalidInput("torus query point must be finite"));
        }
        let frame = self.frame();
        let [major, minor] = self.radii();
        let delta = point - frame.origin();
        let distance = norm(delta);
        if !delta.finite() || !distance.is_finite() {
            return Err(unresolved());
        }
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let world = [point, frame.origin()]
            .into_iter()
            .fold(major + minor, |s, p| {
                s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs())
            });
        let allowance = 32768. * f64::EPSILON * world + 4. * gram * distance;
        if !allowance.is_finite() || allowance >= policy.linear() / 8. {
            return Err(Error::Unsupported(
                "torus query world-coordinate or frame precision is unresolved",
            ));
        }
        let local = frame.local_point(point);
        let rho = local.x.hypot(local.y);
        let tube = (rho - major).hypot(local.z);
        let signed = tube - minor;
        if !local.finite() || !rho.is_finite() || !signed.is_finite() {
            return Err(unresolved());
        }
        let band = policy.length_at_scale(2. * (major + minor).hypot(minor))?;
        if signed.abs() - allowance > band {
            return Ok(NurbsTorusPointQuery {
                location: if signed < 0. {
                    PointLocation::Inside
                } else {
                    PointLocation::Outside
                },
                boundary: None,
            });
        }
        if signed.abs() + allowance > band || rho <= allowance || tube <= allowance {
            return Err(unresolved());
        }
        let (i, u) = quarter(local.x, local.y, allowance)?;
        let (j, v) = quarter(rho - major, local.z, allowance)?;
        let face = i * 4 + j;
        let surface = &self.solid().shell.faces[face].surface;
        let boundary = surface.try_evaluate(u, v)?;
        let actual_distance = norm(point - boundary);
        let normal = surface.normal_at(u, v)?;
        if !boundary.finite()
            || !normal.finite()
            || !actual_distance.is_finite()
            || actual_distance + allowance > band
        {
            return Err(unresolved());
        }
        Ok(NurbsTorusPointQuery {
            location: PointLocation::Boundary,
            boundary: Some(NurbsTorusBoundaryWitness {
                face,
                uv: [u, v],
                point: boundary,
                distance: actual_distance,
                normal,
            }),
        })
    }
}
