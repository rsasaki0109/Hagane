//! Euclidean point classification of the completely validated NURBS frustum.
use crate::*;

fn unresolved() -> Error {
    Error::Unsupported(
        "frustum point distance or query precision is unresolved at caller tolerance",
    )
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn boundary_distance(rho: f64, z: f64, radii: [f64; 2], height: f64) -> Result<f64> {
    let dr = radii[1] - radii[0];
    let side_length = dr.hypot(height);
    // Divide before multiplying: no squared dimensions or squared distances
    // are needed, including for microscopic and large accepted models.
    let fraction = ((rho - radii[0]) / side_length * (dr / side_length)
        + z / side_length * (height / side_length))
        .clamp(0., 1.);
    let side_radius = radii[0] * (1. - fraction) + radii[1] * fraction;
    let side = (rho - side_radius).hypot(z - height * fraction);
    let bottom = (rho - radii[0]).max(0.).hypot(z);
    let top = (rho - radii[1]).max(0.).hypot(z - height);
    let distance = side.min(bottom).min(top);
    if !fraction.is_finite() || !distance.is_finite() {
        return Err(unresolved());
    }
    Ok(distance)
}
impl NurbsFrustumSolid {
    /// Classify against the finite material body using Euclidean distance to
    /// the side and the two finite disk caps, including their circular rims.
    /// The relative length policy uses body dimensions, never query distance.
    /// Unresolved threshold comparisons reject rather than guessing a location.
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        if !point.finite() {
            return Err(Error::InvalidInput("frustum query point must be finite"));
        }
        let frame = self.frame();
        let radii = self.radii();
        let height = self.height();
        let delta = point - frame.origin();
        let query_distance = length(delta);
        if !delta.finite() || !query_distance.is_finite() {
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
            .fold(radii[0].max(radii[1]).max(height), |scale, p| {
                scale.max(p.x.abs()).max(p.y.abs()).max(p.z.abs())
            });
        let allowance = 32768. * f64::EPSILON * world + 4. * metric_error * query_distance;
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
        let distance = boundary_distance(rho, local.z, radii, height)?;
        if distance + allowance <= band {
            return Ok(PointLocation::Boundary);
        }
        if distance - allowance <= band {
            return Err(unresolved());
        }
        if local.z > 0. && local.z < height {
            let fraction = local.z / height;
            let radius = radii[0] * (1. - fraction) + radii[1] * fraction;
            if rho < radius {
                return Ok(PointLocation::Inside);
            }
        }
        Ok(PointLocation::Outside)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy(scale: f64) -> GeometryTolerance {
        GeometryTolerance::new(1e-6 * scale, 1e-10, 0.).unwrap()
    }
    #[test]
    fn finite_disk_rims_use_diagonal_distance_not_plane_extensions() {
        let p = policy(1.);
        let body = NurbsFrustumSolid::new(Frame3::IDENTITY, [2., 2.], 5., p).unwrap();
        for point in [Point3::new(0., 0., 2.), Point3::new(1., 0., 4.)] {
            assert_eq!(
                body.classify_point(point, p).unwrap(),
                PointLocation::Inside
            );
        }
        assert_eq!(
            body.classify_point(Point3::new(2., 0., 2.), p).unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            body.classify_point(Point3::new(2. + 0.6e-6, 0., 5. + 0.6e-6), p)
                .unwrap(),
            PointLocation::Boundary
        );
        assert_eq!(
            body.classify_point(Point3::new(2. + 0.8e-6, 0., 5. + 0.8e-6), p)
                .unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            body.classify_point(Point3::new(4., 0., 5.), p).unwrap(),
            PointLocation::Outside
        );
        assert!(body
            .classify_point(Point3::new(2. + 1e-6, 0., 2.), p)
            .is_err());
    }
    #[test]
    fn sloping_side_distance_is_normal_and_scales_with_the_model() {
        for scale in [1e-60, 1., 1e60] {
            let p = policy(scale);
            let body =
                NurbsFrustumSolid::new(Frame3::IDENTITY, [2. * scale, 4. * scale], 5. * scale, p)
                    .unwrap();
            let normal = Vec3::new(5., 0., -2.).normalized().unwrap();
            let middle = Point3::new(3. * scale, 0., 2.5 * scale);
            assert_eq!(
                body.classify_point(middle + normal * (0.6e-6 * scale), p)
                    .unwrap(),
                PointLocation::Boundary
            );
            assert_eq!(
                body.classify_point(middle + normal * (1.4e-6 * scale), p)
                    .unwrap(),
                PointLocation::Outside
            );
            assert_eq!(
                body.classify_point(middle - normal * (1.4e-6 * scale), p)
                    .unwrap(),
                PointLocation::Inside
            );
        }
    }
    #[test]
    fn pose_relative_policy_and_invalid_query_guards() {
        let rotation = Transform::rotation(Vec3::new(0., 1., 0.), 0.7).unwrap();
        let p = policy(1.);
        let frame = Frame3::new(Point3::new(10., -3., 7.), rotation.axes(), p.absolute()).unwrap();
        let body = NurbsFrustumSolid::new(frame, [4., 2.], 5., p).unwrap();
        assert_eq!(
            body.classify_point(frame.point(Vec3::new(0., 0., 2.5)), p)
                .unwrap(),
            PointLocation::Inside
        );
        let relative = GeometryTolerance::new(1e-6, 1e-10, 1e-4).unwrap();
        assert_eq!(
            body.classify_point(frame.point(Vec3::new(2. + 0.0003, 0., 5.)), relative)
                .unwrap(),
            PointLocation::Boundary
        );
        assert!(body.classify_point(Point3::new(1e12, 0., 0.), p).is_err());
        assert!(body
            .classify_point(Point3::new(f64::NAN, 0., 0.), p)
            .is_err());
    }
}
