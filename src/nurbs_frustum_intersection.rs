//! Checked finite segment intersection with the retained rational frustum.
use crate::*;
#[derive(Clone, Debug)]
pub struct FrustumFaceWitness {
    pub face_id: usize,
    pub uv: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumSegmentHit {
    pub parameter: f64,
    pub point: Point3,
    pub faces: Vec<FrustumFaceWitness>,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumSegmentIntersection {
    pub hits: Vec<NurbsFrustumSegmentHit>,
    pub material_interval: Option<[f64; 2]>,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "frustum segment intersection is tangent, coincident or numerically unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsFrustumSolid {
    /// Intersect a finite world segment with the convex finite material body.
    /// Tangency, rim contacts and unresolved endpoint/threshold cases reject.
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumSegmentIntersection> {
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
        let [r0, r1] = self.radii();
        let height = self.height();
        let scale = (2. * r0.max(r1)).hypot(height);
        let band = policy.length_at_scale(scale)?;
        let world = [start, end, self.frame().origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let axes = self.frame().axes();
        let mut metric_error = 0f64;
        for (i, u) in axes.iter().enumerate() {
            for (j, v) in axes.iter().enumerate() {
                metric_error = metric_error.max((u.dot(*v) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let anchor = norm(start - self.frame().origin()).max(norm(end - self.frame().origin()));
        let arithmetic = 65536. * f64::EPSILON * world + 4. * metric_error * anchor;
        if !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. || length <= 10. * band {
            return Err(unresolved());
        }
        let endpoint_locations = [
            self.classify_point(start, policy)?,
            self.classify_point(end, policy)?,
        ];
        if endpoint_locations.contains(&PointLocation::Boundary) {
            return Err(unresolved());
        }
        let a = self.frame().local_point(start);
        let b = self.frame().local_point(end);
        let d = b - a;
        let normalization = scale.max(norm(a)).max(norm(b));
        if !normalization.is_normal() {
            return Err(unresolved());
        }
        let a = Vec3::new(
            a.x / normalization,
            a.y / normalization,
            a.z / normalization,
        );
        let d = Vec3::new(
            d.x / normalization,
            d.y / normalization,
            d.z / normalization,
        );
        let h = height / normalization;
        let rb = r0 / normalization;
        let slope = (r1 - r0) / height;
        let ra = rb + slope * a.z;
        let rd = slope * d.z;
        let aa = d.x * d.x + d.y * d.y - rd * rd;
        let bb = 2. * (a.x * d.x + a.y * d.y - ra * rd);
        let cc = a.x * a.x + a.y * a.y - ra * ra;
        let coeff_scale = aa.abs().max(bb.abs()).max(cc.abs());
        let operand_scale = d.x * d.x
            + d.y * d.y
            + rd * rd
            + 2. * (a.x * d.x).abs()
            + 2. * (a.y * d.y).abs()
            + 2. * (ra * rd).abs()
            + a.x * a.x
            + a.y * a.y
            + ra * ra;
        let perturbation = (arithmetic / normalization) * (1. + slope.abs());
        let guard = 8192. * f64::EPSILON * operand_scale
            + 8. * (arithmetic / normalization)
                * (1. + slope.abs())
                * (1. + a.x.abs() + a.y.abs() + ra.abs() + d.x.abs() + d.y.abs() + rd.abs())
            + 32. * perturbation * perturbation;
        if !guard.is_finite() {
            return Err(unresolved());
        }
        if !coeff_scale.is_normal() || ![aa, bb, cc].iter().all(|x| x.is_finite()) {
            return Err(unresolved());
        }
        let mut roots = vec![];
        let cone_guard = (band + arithmetic) * (1. + slope.abs()) / normalization
            + 128.
                * f64::EPSILON
                * (1. + a.x.abs() + a.y.abs() + ra.abs() + d.x.abs() + d.y.abs() + rd.abs());
        if !cone_guard.is_finite() {
            return Err(unresolved());
        }
        let cone_inside = ra > cone_guard
            && ra + rd > cone_guard
            && a.x.hypot(a.y) - ra < -cone_guard
            && (a.x + d.x).hypot(a.y + d.y) - (ra + rd) < -cone_guard;
        if cone_inside {
            // Norm minus a positive affine radius is convex: strict endpoint
            // interior proves there is no lateral boundary along this segment.
        } else if aa.abs() <= guard {
            if aa != 0. {
                return Err(unresolved());
            }
            if bb == 0. {
                if cc.abs() <= guard {
                    return Err(unresolved());
                }
            } else {
                if bb.abs() <= 3. * guard + 2. * aa.abs() {
                    return Err(unresolved());
                }
                roots.push(-cc / bb);
            }
        } else {
            let discriminant = bb * bb - 4. * aa * cc;
            let error = 8192. * f64::EPSILON * (bb * bb + 4. * (aa * cc).abs())
                + guard * (2. * bb.abs() + 4. * aa.abs() + 4. * cc.abs() + 8. * guard);
            if discriminant.abs() <= error {
                return Err(unresolved());
            }
            if discriminant > 0. {
                let q = -0.5 * (bb + discriminant.sqrt().copysign(bb));
                if q == 0. {
                    return Err(unresolved());
                }
                roots.extend([q / aa, cc / q]);
            }
        }
        let mut candidates = vec![];
        for t in roots {
            if !t.is_finite() {
                return Err(unresolved());
            }
            if t > 0. && t < 1. {
                let derivative = (2. * aa * t + bb).abs();
                let uncertainty =
                    guard * (1. + t.abs() + t * t) / (derivative - guard * (1. + 2. * t.abs()));
                if derivative <= guard * (1. + 2. * t.abs())
                    || !uncertainty.is_finite()
                    || uncertainty * length >= policy.linear() / 4.
                {
                    return Err(unresolved());
                }
                let p = a + d * t;
                if p.z > band / normalization && p.z < h - band / normalization {
                    candidates.push((t, None));
                } else if p.z.abs() <= band / normalization
                    || (p.z - h).abs() <= band / normalization
                {
                    return Err(unresolved());
                }
            }
        }
        if d.z != 0. {
            for (face, z, r) in [(0, 0., r0), (1, h, r1)] {
                let t = (z - a.z) / d.z;
                if t > 0. && t < 1. {
                    if arithmetic / (d.z.abs() * normalization) * length >= policy.linear() / 16. {
                        return Err(unresolved());
                    }
                    let p = a + d * t;
                    let rho = p.x.hypot(p.y) * normalization;
                    if (rho - r).abs() <= band + arithmetic {
                        return Err(unresolved());
                    }
                    if rho < r {
                        candidates.push((t, Some(face)));
                    }
                }
            }
        } else if a.z.abs() <= band / normalization || (a.z - h).abs() <= band / normalization {
            return Err(unresolved());
        }
        candidates.sort_by(|x, y| x.0.total_cmp(&y.0));
        if candidates
            .windows(2)
            .any(|w| (w[1].0 - w[0].0) * length <= 10. * band)
        {
            return Err(unresolved());
        }
        let mut hits = vec![];
        for (t, cap) in candidates {
            if !t.is_finite() || t * length <= 10. * band || (1. - t) * length <= 10. * band {
                return Err(unresolved());
            }
            let point = start + delta * t;
            let p = self.frame().local_point(point);
            let mut faces = vec![];
            if let Some(face) = cap {
                faces.push(FrustumFaceWitness {
                    face_id: face,
                    uv: [p.x, p.y],
                });
            } else {
                let rho = p.x.hypot(p.y);
                if !rho.is_normal() {
                    return Err(unresolved());
                }
                let dirs = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
                for (q, dir) in dirs.iter().enumerate() {
                    let x = p.x * dir[0] + p.y * dir[1];
                    let y = -p.x * dir[1] + p.y * dir[0];
                    if x >= 0. && y >= 0. {
                        let k = y / (rho + x);
                        let u = std::f64::consts::SQRT_2 * k
                            / (1. + (std::f64::consts::SQRT_2 - 1.) * k);
                        faces.push(FrustumFaceWitness {
                            face_id: 2 + q,
                            uv: [u, p.z / height],
                        });
                    }
                }
            }
            if faces.is_empty() {
                return Err(unresolved());
            }
            for witness in &faces {
                let evaluated = self.solid().shell.faces[witness.face_id]
                    .surface
                    .try_evaluate(witness.uv[0], witness.uv[1])?;
                if norm(evaluated - point) + arithmetic > policy.linear() / 4. {
                    return Err(unresolved());
                }
            }
            if self.classify_point(point, policy)? != PointLocation::Boundary {
                return Err(unresolved());
            }
            hits.push(NurbsFrustumSegmentHit {
                parameter: t,
                point,
                faces,
            });
        }
        let mut cuts = vec![0.];
        cuts.extend(hits.iter().map(|h| h.parameter));
        cuts.push(1.);
        let mut material = None;
        for w in cuts.windows(2) {
            let middle = (w[0] + w[1]) / 2.;
            let location = self.classify_point(start + delta * middle, policy)?;
            match location {
                PointLocation::Inside => {
                    if material.is_some() {
                        return Err(unresolved());
                    }
                    material = Some([w[0], w[1]]);
                }
                PointLocation::Outside => (),
                PointLocation::Boundary => return Err(unresolved()),
            }
        }
        if hits.len() > 2
            || (endpoint_locations[0] == PointLocation::Inside)
                != material.is_some_and(|v| v[0] == 0.)
            || (endpoint_locations[1] == PointLocation::Inside)
                != material.is_some_and(|v| v[1] == 1.)
        {
            return Err(unresolved());
        }
        for hit in &hits {
            let Some(interval) = material else {
                return Err(unresolved());
            };
            let entering = hit.parameter == interval[0];
            if !entering && hit.parameter != interval[1] {
                return Err(unresolved());
            }
            for witness in &hit.faces {
                let face = &self.solid().shell.faces[witness.face_id];
                let normal =
                    face.surface.normal_at(witness.uv[0], witness.uv[1])? * face.orientation as f64;
                let crossing = normal.dot(delta);
                if !crossing.is_finite()
                    || crossing.abs() <= 256. * f64::EPSILON * length + arithmetic
                    || (crossing < 0.) != entering
                {
                    return Err(unresolved());
                }
            }
        }
        Ok(NurbsFrustumSegmentIntersection {
            hits,
            material_interval: material,
        })
    }
}
