//! Mesh-free segment intervals in the actual one- or two-cut finite slab.
use crate::*;
fn unresolved() -> Error {
    Error::Unsupported(
        "frustum slab segment contact, retained face parameter mapping or precision is unresolved",
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
struct Cut {
    face_id: usize,
    origin: Point3,
    normal: Vec3,
    gaps: [f64; 2],
    slope: f64,
}
struct Geometry<'a> {
    canonical: &'a NurbsFrustumSolid,
    solid: &'a Solid,
    present: [bool; 2],
}
fn intersect(
    start: Point3,
    end: Point3,
    policy: GeometryTolerance,
    geometry: Geometry<'_>,
    source_query: impl Fn(Point3) -> Result<PointLocation>,
    source_segment: impl Fn() -> Result<NurbsBoredFrustumSegmentIntersection>,
    actual_query: impl Fn(Point3) -> Result<PointLocation>,
) -> Result<NurbsBoredFrustumSegmentIntersection> {
    let Geometry {
        canonical,
        solid,
        present,
    } = geometry;
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
    // Actual endpoint classification applies original source frame/world policy
    // before any clipped-empty result or subset shortcut.
    let locations = [actual_query(start)?, actual_query(end)?];
    if locations.contains(&PointLocation::Boundary) {
        return Err(Error::Unsupported(
            "frustum slab segment endpoint is within the boundary band; move it away",
        ));
    }
    let [a, b] = canonical.radii();
    let scale = (2. * a.max(b)).hypot(canonical.height());
    let band = policy.length_at_scale(scale)?;
    if length <= 10. * band {
        return Err(unresolved());
    }
    let mut world = [start, end, canonical.frame().origin()]
        .into_iter()
        .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
    let mut cuts = Vec::with_capacity(2);
    for (face_id, &is_present) in present.iter().enumerate() {
        if !is_present {
            continue;
        }
        let cap = &solid.shell.faces[face_id];
        let Surface::Plane { origin, u, v } = cap.surface else {
            return Err(unresolved());
        };
        let normal = u.cross(v).normalized()? * f64::from(cap.orientation);
        let gaps = [normal.dot(start - origin), normal.dot(end - origin)];
        let slope = normal.dot(delta);
        if gaps.iter().any(|v| !v.is_finite()) || !slope.is_finite() {
            return Err(unresolved());
        }
        world = world
            .max(origin.x.abs())
            .max(origin.y.abs())
            .max(origin.z.abs());
        cuts.push(Cut {
            face_id,
            origin,
            normal,
            gaps,
            slope,
        });
    }
    // Reserve every present cap's original anchor BEFORE solving even an empty
    // original-source intersection. Cropping never hides a remote plane origin.
    let arithmetic = 65536. * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. {
        return Err(unresolved());
    }
    let source = source_segment()?;
    let mut material_intervals = vec![];
    let mut roots: [Option<f64>; 2] = [None, None];
    for mut retained in source.material_intervals.iter().copied() {
        let mut removed = false;
        for cut in &cuts {
            if cut.slope.abs() <= 2. * arithmetic {
                if cut.gaps.iter().all(|d| *d < -band - arithmetic) {
                } else if cut.gaps.iter().all(|d| *d > band + arithmetic) {
                    removed = true;
                    break;
                } else {
                    return Err(unresolved());
                }
            } else {
                let t = -cut.gaps[0] / cut.slope;
                if !t.is_finite() {
                    return Err(unresolved());
                }
                if t > retained[0] && t < retained[1] {
                    if arithmetic / (cut.slope.abs() - 2. * arithmetic) * length
                        >= policy.linear() / 8.
                        || (t - retained[0]) * length <= 10. * band
                        || (retained[1] - t) * length <= 10. * band
                    {
                        return Err(unresolved());
                    }
                    if source_query(start + delta * t)? != PointLocation::Inside {
                        return Err(unresolved());
                    }
                    if cut.slope > 0. {
                        retained[1] = t;
                    } else {
                        retained[0] = t;
                    }
                    roots[cut.face_id] = Some(t);
                } else {
                    // Root-order rounding cannot turn a tiny retained piece into
                    // a false miss: BOTH current interval endpoints must prove
                    // the same strict retained/removed halfspace membership.
                    let gaps = retained.map(|t| cut.normal.dot(start + delta * t - cut.origin));
                    if gaps.iter().all(|g| *g > band + arithmetic) {
                        removed = true;
                        break;
                    } else if !gaps.iter().all(|g| *g < -band - arithmetic) {
                        return Err(unresolved());
                    }
                }
            }
        }
        if !removed {
            material_intervals.push(retained);
        }
    }
    let mut hits = vec![];
    for retained in material_intervals.iter().copied() {
        if (retained[1] - retained[0]) * length <= 10. * band {
            return Err(unresolved());
        }
        if actual_query(start + delta * (retained[0] + (retained[1] - retained[0]) / 2.))?
            != PointLocation::Inside
        {
            return Err(unresolved());
        }
        for &t in &retained {
            if t == 0. || t == 1. {
                continue;
            }
            let point = start + delta * t;
            // Only FINAL material endpoints create hits. An earlier cap root
            // removed by the other cut never becomes an apparent boundary hit.
            let faces = if let Some(cap_id) = roots.iter().position(|root| *root == Some(t)) {
                vec![FrustumFaceWitness {
                    face_id: cap_id,
                    uv: plane_uv(&solid.shell.faces[cap_id].surface, point)?,
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
                        if present[f.face_id] {
                            return Err(unresolved());
                        }
                        mapped.push(f.clone());
                    } else {
                        let Surface::Nurbs(surface) = &solid.shell.faces[f.face_id].surface else {
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
            if faces.is_empty() {
                return Err(unresolved());
            }
            for f in &faces {
                let face = &solid.shell.faces[f.face_id];
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
            if actual_query(point)? != PointLocation::Boundary {
                return Err(unresolved());
            }
            hits.push(NurbsFrustumSegmentHit {
                parameter: t,
                point,
                faces,
            });
        }
    }
    hits.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
    if hits
        .windows(2)
        .any(|w| (w[1].parameter - w[0].parameter) * length <= 10. * band)
        || material_intervals
            .windows(2)
            .any(|w| (w[1][0] - w[0][1]) * length <= 10. * band)
    {
        return Err(unresolved());
    }
    if (locations[0] == PointLocation::Inside) != material_intervals.iter().any(|v| v[0] == 0.)
        || (locations[1] == PointLocation::Inside) != material_intervals.iter().any(|v| v[1] == 1.)
    {
        return Err(unresolved());
    }
    Ok(NurbsBoredFrustumSegmentIntersection {
        hits,
        material_intervals,
    })
}
impl NurbsFrustumSlabSolid {
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumSegmentIntersection> {
        self.validate(policy)?;
        let result = intersect(
            start,
            end,
            policy,
            Geometry {
                canonical: self.source(),
                solid: self.solid(),
                present: [
                    self.lower_section().is_some(),
                    self.upper_section().is_some(),
                ],
            },
            |p| self.source().classify_point(p, policy),
            || {
                let result = self.source().intersect_segment(start, end, policy)?;
                Ok(NurbsBoredFrustumSegmentIntersection {
                    hits: result.hits,
                    material_intervals: result.material_interval.into_iter().collect(),
                })
            },
            |p| self.classify_point(p, policy),
        )?;
        if result.material_intervals.len() > 1 {
            return Err(unresolved());
        }
        Ok(NurbsFrustumSegmentIntersection {
            hits: result.hits,
            material_interval: result.material_intervals.into_iter().next(),
        })
    }
}
impl NurbsBoredFrustumSlabSolid {
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumSegmentIntersection> {
        self.validate(policy)?;
        intersect(
            start,
            end,
            policy,
            Geometry {
                canonical: self.source().source(),
                solid: self.solid(),
                present: [
                    self.outer().lower_section().is_some(),
                    self.outer().upper_section().is_some(),
                ],
            },
            |p| self.source().classify_point(p, policy),
            || self.source().intersect_segment(start, end, policy),
            |p| self.classify_point(p, policy),
        )
    }
}
