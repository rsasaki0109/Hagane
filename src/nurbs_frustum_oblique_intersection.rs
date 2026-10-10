//! Checked segment intervals in the actual retained oblique frustum body.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "oblique frustum segment contact, parameter mapping or precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn plane_uv(surface: &Surface, p: Point3) -> Result<[f64; 2]> {
    let Surface::Plane { origin, u, v } = surface else {
        return Err(unresolved());
    };
    let d = p - *origin;
    let aa = u.dot(*u);
    let ab = u.dot(*v);
    let bb = v.dot(*v);
    let determinant = aa * bb - ab * ab;
    if !determinant.is_normal() || determinant <= 0. {
        return Err(unresolved());
    }
    Ok([
        (bb * u.dot(d) - ab * v.dot(d)) / determinant,
        (aa * v.dot(d) - ab * u.dot(d)) / determinant,
    ])
}
impl NurbsObliqueFrustumSolid {
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
        let locations = [
            self.classify_point(start, policy)?,
            self.classify_point(end, policy)?,
        ];
        if locations.contains(&PointLocation::Boundary) {
            return Err(Error::Unsupported(
                "oblique segment endpoint is within the boundary band; move it away",
            ));
        }
        let [a, b] = self.source().radii();
        let scale = (2. * a.max(b)).hypot(self.source().height());
        let band = policy.length_at_scale(scale)?;
        if length <= 10. * band {
            return Err(unresolved());
        }
        let cap_id = if self.is_lower() { 1 } else { 0 };
        let cap = &self.solid().shell.faces[cap_id];
        let Surface::Plane { origin, u, v } = cap.surface else {
            return Err(unresolved());
        };
        let normal = u.cross(v).normalized()? * f64::from(cap.orientation);
        let world = [start, end, origin]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let arithmetic = 65536. * f64::EPSILON * world;
        if !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. {
            return Err(unresolved());
        }
        let gaps = [normal.dot(start - origin), normal.dot(end - origin)];
        let slope = normal.dot(delta);
        if gaps.iter().any(|v| !v.is_finite()) || !slope.is_finite() {
            return Err(unresolved());
        }
        let source = self.source().intersect_segment(start, end, policy)?;
        let mut interval = source.material_interval;
        let mut cut_root = None;
        if let Some(mut retained) = interval {
            if slope.abs() <= 2. * arithmetic {
                if gaps.iter().all(|d| *d < -band - arithmetic) {
                } else if gaps.iter().all(|d| *d > band + arithmetic) {
                    interval = None;
                } else {
                    return Err(unresolved());
                }
            } else {
                let t = -gaps[0] / slope;
                if !t.is_finite() {
                    return Err(unresolved());
                }
                if t > retained[0] && t < retained[1] {
                    if arithmetic / (slope.abs() - 2. * arithmetic) * length >= policy.linear() / 8.
                    {
                        return Err(unresolved());
                    }
                    if (t - retained[0]) * length <= 10. * band
                        || (retained[1] - t) * length <= 10. * band
                    {
                        return Err(unresolved());
                    }
                    let point = start + delta * t;
                    if self.source().classify_point(point, policy)? != PointLocation::Inside {
                        return Err(unresolved());
                    }
                    if slope > 0. {
                        retained[1] = t;
                    } else {
                        retained[0] = t;
                    }
                    interval = Some(retained);
                    cut_root = Some(t);
                } else {
                    let endpoint_gaps = retained.map(|t| normal.dot(start + delta * t - origin));
                    if endpoint_gaps.iter().all(|g| *g > band + arithmetic) {
                        interval = None;
                    } else if !endpoint_gaps.iter().all(|g| *g < -band - arithmetic) {
                        return Err(unresolved());
                    }
                }
            }
        }
        let mut hits = vec![];
        if let Some(retained) = interval {
            if (retained[1] - retained[0]) * length <= 10. * band {
                return Err(unresolved());
            }
            if self.classify_point(
                start + delta * (retained[0] + (retained[1] - retained[0]) / 2.),
                policy,
            )? != PointLocation::Inside
            {
                return Err(unresolved());
            }
            for &t in &retained {
                if t == 0. || t == 1. {
                    continue;
                }
                let point = start + delta * t;
                let faces = if cut_root == Some(t) {
                    vec![FrustumFaceWitness {
                        face_id: cap_id,
                        uv: plane_uv(&cap.surface, point)?,
                    }]
                } else {
                    let original = source
                        .hits
                        .iter()
                        .find(|h| h.parameter == t)
                        .ok_or_else(unresolved)?;
                    let mut mapped = vec![];
                    for f in &original.faces {
                        if f.face_id < 2 {
                            if f.face_id == cap_id {
                                return Err(unresolved());
                            }
                            mapped.push(f.clone());
                        } else {
                            let Surface::Nurbs(surface) =
                                &self.solid().shell.faces[f.face_id].surface
                            else {
                                return Err(unresolved());
                            };
                            let u = f.uv[0];
                            let p0 = surface.evaluate(u, 0.)?;
                            let p1 = surface.evaluate(u, 1.)?;
                            let generator = p1 - p0;
                            let gscale = norm(generator);
                            if !gscale.is_normal() {
                                return Err(unresolved());
                            }
                            let axis = generator * (1. / gscale);
                            let q = axis.dot(point - p0) / gscale;
                            let basis = [(1. - u).powi(2), 2. * u * (1. - u), u * u];
                            let w = surface.weights();
                            let wb = (0..3).map(|i| basis[i] * w[2 * i]).sum::<f64>();
                            let wt = (0..3).map(|i| basis[i] * w[2 * i + 1]).sum::<f64>();
                            let v = q * wb / ((1. - q) * wt + q * wb);
                            if !(0. ..=1.).contains(&q) || !(0. ..=1.).contains(&v) {
                                return Err(unresolved());
                            }
                            mapped.push(FrustumFaceWitness {
                                face_id: f.face_id,
                                uv: [u, v],
                            });
                        }
                    }
                    mapped
                };
                for f in &faces {
                    let face = &self.solid().shell.faces[f.face_id];
                    let represented = face.surface.try_evaluate(f.uv[0], f.uv[1])?;
                    if norm(represented - point) + arithmetic >= policy.linear() / 4. {
                        return Err(unresolved());
                    }
                    let crossing = (face.surface.normal_at(f.uv[0], f.uv[1])?
                        * f64::from(face.orientation))
                    .dot(delta);
                    if !crossing.is_finite()
                        || crossing.abs() <= 256. * f64::EPSILON * length + arithmetic
                        || (crossing < 0.) != (t == retained[0])
                    {
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
        }
        if (locations[0] == PointLocation::Inside) != interval.is_some_and(|v| v[0] == 0.)
            || (locations[1] == PointLocation::Inside) != interval.is_some_and(|v| v[1] == 1.)
        {
            return Err(unresolved());
        }
        Ok(NurbsFrustumSegmentIntersection {
            hits,
            material_interval: interval,
        })
    }
}
