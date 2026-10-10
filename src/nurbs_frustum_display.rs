//! Conforming display of retained rational frustum faces and their shared edges.
use crate::*;
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn failure() -> Error {
    Error::Tessellation(
        "rational frustum display cannot resolve requested error within its grid budget",
    )
}
fn split(h: &[[f64; 4]; 3], t: f64) -> ([[f64; 4]; 3], [[f64; 4]; 3]) {
    let mix = |a: [f64; 4], b: [f64; 4]| std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t);
    let a = mix(h[0], h[1]);
    let b = mix(h[1], h[2]);
    let c = mix(a, b);
    ([h[0], a, c], [c, b, h[2]])
}
/// Homogeneous Bernstein residual to the endpoint chord, not a circle recipe.
fn rim_bound(curve: &NurbsCurve, a: f64, b: f64, arithmetic: f64) -> Result<f64> {
    if curve.degree() != 2 || curve.control_points().len() != 3 || curve.domain() != [0., 1.] {
        return Err(failure());
    }
    let origin = curve.control_points()[0];
    let h: [[f64; 4]; 3] = std::array::from_fn(|i| {
        let p = curve.control_points()[i] - origin;
        let w = curve.weights()[i];
        [p.x * w, p.y * w, p.z * w, w]
    });
    let (left, _) = split(&h, b);
    let (_, restricted) = split(&left, a / b);
    let project = |h: [f64; 4]| Vec3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3]);
    let corners = [project(restricted[0]), project(restricted[2])];
    let minimum = restricted
        .iter()
        .map(|h| h[3])
        .fold(f64::INFINITY, f64::min);
    if !minimum.is_finite() || minimum <= 0. {
        return Err(failure());
    }
    let mut bound = 0f64;
    for k in 0..=3 {
        let mut residual = Vec3::new(0., 0., 0.);
        for (j, corner) in corners.iter().enumerate() {
            if k < j || k - j > 2 {
                continue;
            }
            let h = restricted[k - j];
            let factor = if j == 0 {
                (3 - k) as f64 / 3.
            } else {
                k as f64 / 3.
            };
            residual = residual + (Vec3::new(h[0], h[1], h[2]) - *corner * h[3]) * factor;
        }
        bound = bound.max(norm(residual) / minimum);
    }
    let mismatch = norm(curve.evaluate(a)? - origin - corners[0])
        .max(norm(curve.evaluate(b)? - origin - corners[1]));
    Ok(bound + mismatch + arithmetic)
}
fn emit(mesh: &mut Mesh, p: [Point3; 3], n: [Vec3; 3], face: usize, orientation: i8) -> Result<()> {
    let mut p = p;
    let mut n = n;
    if orientation < 0 {
        p.swap(1, 2);
        n.swap(1, 2);
    }
    n = n.map(|v| v * orientation as f64);
    let cross = (p[1] - p[0]).cross(p[2] - p[0]);
    if !p.iter().all(|p| p.finite())
        || !n.iter().all(|n| n.finite())
        || norm(cross) <= 0.
        || !norm(cross).is_finite()
        || cross.dot(n[0] + n[1] + n[2]) <= 0.
    {
        return Err(failure());
    }
    let offset = mesh.positions.len();
    mesh.positions.extend(p);
    mesh.normals.extend(n);
    mesh.triangles.push([offset, offset + 1, offset + 2]);
    mesh.face_ids.push(face);
    Ok(())
}
impl NurbsFrustumSolid {
    /// Tessellate actual rational faces with one common dyadic grid. Bernstein
    /// cell bounds include the mixed U/V twist; canonical retained edge samples
    /// supply every shared boundary position. Shading duplicates copy these
    /// positions exactly while keeping the cap/side normal discontinuity.
    pub fn tessellate(&self, chord_error: f64, policy: GeometryTolerance) -> Result<Mesh> {
        self.validate(policy)?;
        tessellate_ruled_four_quarters(self.solid(), chord_error, policy, None)
    }
}
pub(crate) fn tessellate_ruled_four_quarters(
    body: &Solid,
    chord_error: f64,
    _policy: GeometryTolerance,
    generator_weights: Option<[[f64; 2]; 4]>,
) -> Result<Mesh> {
    // Private helper: callers must certify their retained typed B-rep first.
    // Generic Solid validation does not admit these rational trim domains.
    if body.vertices.len() != 8 || body.edges.len() != 12 || body.shell.faces.len() != 6 {
        return Err(failure());
    }
    if let Some(weights) = generator_weights {
        if weights.iter().flatten().any(|w| !w.is_finite() || *w <= 0.) {
            return Err(failure());
        }
    }
    if !chord_error.is_finite() || chord_error <= 0. {
        return Err(Error::InvalidInput(
            "frustum display needs positive finite chord error",
        ));
    }
    let mut scale = 0f64;
    for face in &body.shell.faces {
        match &face.surface {
            Surface::Nurbs(s) => {
                for p in s.control_points() {
                    scale = scale.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
                }
            }
            Surface::Plane { origin, .. } => {
                scale = scale
                    .max(origin.x.abs())
                    .max(origin.y.abs())
                    .max(origin.z.abs())
            }
            _ => return Err(failure()),
        }
    }
    let mut weight_ratio = 1.;
    if generator_weights.is_some() {
        for face in &body.shell.faces {
            if let Surface::Nurbs(surface) = &face.surface {
                let min = surface
                    .weights()
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max = surface.weights().iter().copied().fold(0., f64::max);
                weight_ratio = f64::max(weight_ratio, max / min);
            }
        }
        for edge in &body.edges {
            if let Curve::Nurbs(curve) = &edge.curve {
                let min = curve
                    .weights()
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max = curve.weights().iter().copied().fold(0., f64::max);
                weight_ratio = f64::max(weight_ratio, max / min);
            }
        }
    }
    let arithmetic = 65536. * f64::EPSILON * scale.max(f64::MIN_POSITIVE) * weight_ratio;
    if !arithmetic.is_finite() || arithmetic >= chord_error / 16. {
        return Err(failure());
    }
    let surfaces = body.shell.faces[2..]
        .iter()
        .map(|f| match &f.surface {
            Surface::Nurbs(s) => Ok(s.as_ref()),
            _ => Err(failure()),
        })
        .collect::<Result<Vec<_>>>()?;
    let rims = body.edges[..8]
        .iter()
        .map(|e| match &e.curve {
            Curve::Nurbs(c) => Ok(c.as_ref()),
            _ => Err(failure()),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut count = 1usize;
    for surface in &surfaces {
        let bounded = surface.tessellate_bounded(chord_error * 0.5, 4096)?;
        let n = (bounded.uv_ranges.len() as f64).sqrt() as usize;
        if n * n != bounded.uv_ranges.len() || !n.is_power_of_two() {
            return Err(failure());
        }
        count = count.max(n);
    }
    loop {
        if count > 64 {
            return Err(failure());
        }
        let mut all = true;
        for curve in &rims {
            for i in 0..count {
                all &= rim_bound(
                    curve,
                    i as f64 / count as f64,
                    (i + 1) as f64 / count as f64,
                    arithmetic,
                )? <= chord_error * 0.75;
            }
        }
        if all {
            break;
        }
        count *= 2;
    }
    // Cache by topology IDs/parameters, never by approximate XYZ proximity.
    let edge_points = body
        .edges
        .iter()
        .enumerate()
        .map(|(edge, e)| {
            (0..=count)
                .map(|i| {
                    if i == 0 {
                        Ok(body.vertices[e.vertices[0]].point)
                    } else if i == count {
                        Ok(body.vertices[e.vertices[1]].point)
                    } else {
                        let mut t = i as f64 / count as f64;
                        if edge >= 8 {
                            if let Some(weights) = generator_weights {
                                let [w0, w1] = weights[edge - 8];
                                let scale = w0.max(w1);
                                let (w0, w1) = (w0 / scale, w1 / scale);
                                t = t * w1 / ((1. - t) * w0 + t * w1);
                                if !t.is_finite() || t <= 0. || t >= 1. {
                                    return Err(failure());
                                }
                            }
                        }
                        let p = e.curve.try_evaluate(t)?;
                        if !p.finite() {
                            Err(failure())
                        } else {
                            Ok(p)
                        }
                    }
                })
                .collect::<Result<Vec<_>>>()
                .map(|p| (edge, p))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut mesh = Mesh::default();
    for (q, surface) in surfaces.iter().enumerate() {
        let mut points = Vec::with_capacity((count + 1) * (count + 1));
        let mut normals = Vec::with_capacity(points.capacity());
        let mut mismatch = 0f64;
        for i in 0..=count {
            for j in 0..=count {
                let u = i as f64 / count as f64;
                let v = j as f64 / count as f64;
                let exact = surface.evaluate(u, v)?;
                let canonical = if j == 0 {
                    edge_points[q].1[i]
                } else if j == count {
                    edge_points[4 + q].1[i]
                } else if i == 0 {
                    edge_points[8 + q].1[j]
                } else if i == count {
                    edge_points[8 + (q + 1) % 4].1[j]
                } else {
                    exact
                };
                mismatch = mismatch.max(norm(canonical - exact));
                points.push(canonical);
                normals.push(surface.normal(u, v)?);
            }
        }
        if !mismatch.is_finite() || mismatch + arithmetic > chord_error * 0.25 {
            return Err(failure());
        }
        for i in 0..count {
            for j in 0..count {
                let ranges = [
                    [i as f64 / count as f64, (i + 1) as f64 / count as f64],
                    [j as f64 / count as f64, (j + 1) as f64 / count as f64],
                ];
                let restricted = surface.restricted(ranges)?;
                let bounded =
                    restricted.tessellate_bounded(chord_error * 0.75 - mismatch - arithmetic, 1)?;
                if bounded.error_bounds.len() != 1
                    || bounded.error_bounds[0] + mismatch + arithmetic > chord_error
                {
                    return Err(failure());
                }
                let a = i * (count + 1) + j;
                let b = (i + 1) * (count + 1) + j;
                let c = b + 1;
                let d = a + 1;
                for indices in [[a, b, c], [a, c, d]] {
                    emit(
                        &mut mesh,
                        indices.map(|k| points[k]),
                        indices.map(|k| normals[k]),
                        q + 2,
                        body.shell.faces[q + 2].orientation,
                    )?;
                }
            }
        }
    }
    for cap in 0..2 {
        let face = &body.shell.faces[cap];
        let center = match face.surface {
            Surface::Plane { origin, .. } => origin,
            _ => return Err(failure()),
        };
        let normal = face.surface.normal_at(0., 0.)?;
        for q in 0..4 {
            for i in 0..count {
                let curve = rims[cap * 4 + q];
                let bound = rim_bound(
                    curve,
                    i as f64 / count as f64,
                    (i + 1) as f64 / count as f64,
                    arithmetic,
                )?;
                let a = i as f64 / count as f64;
                let b = (i + 1) as f64 / count as f64;
                let replacement = norm(edge_points[cap * 4 + q].1[i] - curve.evaluate(a)?)
                    .max(norm(edge_points[cap * 4 + q].1[i + 1] - curve.evaluate(b)?));
                if !replacement.is_finite() || bound + replacement > chord_error {
                    return Err(failure());
                }
                emit(
                    &mut mesh,
                    [
                        center,
                        edge_points[cap * 4 + q].1[i],
                        edge_points[cap * 4 + q].1[i + 1],
                    ],
                    [normal; 3],
                    cap,
                    face.orientation,
                )?;
            }
        }
    }
    if mesh.signed_volume() <= 0. || !mesh.signed_volume().is_finite() {
        return Err(failure());
    }
    Ok(mesh)
}
