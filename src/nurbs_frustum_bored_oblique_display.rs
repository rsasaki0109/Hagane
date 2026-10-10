//! Bounded display of actual rational annular oblique children.
use crate::nurbs_frustum_display::{emit, rim_bound};
use crate::*;
fn error() -> Error {
    Error::Tessellation("bored frustum display exceeds its precision or common grid budget")
}
fn norm(p: Vec3) -> f64 {
    p.x.hypot(p.y).hypot(p.z)
}
fn indices(q: usize) -> (usize, usize, usize, usize) {
    if q < 4 {
        (2 + q, q, 4 + q, 8 + q)
    } else {
        let q = q - 4;
        (6 + q, 12 + q, 16 + q, 20 + q)
    }
}
fn triangle_radius(p: [[f64; 2]; 3]) -> f64 {
    let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    let signs = [cross(p[0], p[1]), cross(p[1], p[2]), cross(p[2], p[0])];
    if signs.iter().all(|v| *v >= 0.) || signs.iter().all(|v| *v <= 0.) {
        return 0.;
    }
    let mut minimum = f64::INFINITY;
    for i in 0..3 {
        let a = p[i];
        let b = p[(i + 1) % 3];
        let d = [b[0] - a[0], b[1] - a[1]];
        let scale = d[0].abs().max(d[1].abs());
        if scale == 0. {
            minimum = minimum.min(a[0].hypot(a[1]));
            continue;
        }
        let e = [d[0] / scale, d[1] / scale];
        let t = (-(a[0] / scale * e[0] + a[1] / scale * e[1]) / (e[0] * e[0] + e[1] * e[1]))
            .clamp(0., 1.);
        minimum = minimum.min((a[0] + t * d[0]).hypot(a[1] + t * d[1]));
    }
    minimum
}
impl NurbsBoredObliqueFrustumSolid {
    /// Display the actual eight rational walls and both annular cap faces.
    /// One common dyadic grid and topology-keyed retained-edge samples close all
    /// seams exactly. Cap strips preserve the opening; no center fans are emitted.
    pub fn tessellate(&self, chord_error: f64, policy: GeometryTolerance) -> Result<Mesh> {
        tessellate_annular_ruled(
            self.solid(),
            self.source().source().frame(),
            self.source().radius(),
            chord_error,
            || self.validate(policy),
        )
    }
}
pub(crate) fn tessellate_annular_ruled(
    body: &Solid,
    source_frame: Frame3,
    hole_radius: f64,
    chord_error: f64,
    validate: impl FnOnce() -> Result<()>,
) -> Result<Mesh> {
    validate()?;
    if !chord_error.is_finite() || chord_error <= 0. {
        return Err(Error::InvalidInput(
            "bored frustum display needs positive finite chord tolerance",
        ));
    }
    if body.vertices.len() != 16 || body.edges.len() != 24 || body.shell.faces.len() != 10 {
        return Err(error());
    }
    let surfaces = (0..8)
        .map(|q| match &body.shell.faces[indices(q).0].surface {
            Surface::Nurbs(surface)
                if surface.degrees() == [2, 1]
                    && surface.control_counts() == [3, 2]
                    && surface.domain() == [[0., 1.], [0., 1.]] =>
            {
                Ok(surface.as_ref())
            }
            _ => Err(error()),
        })
        .collect::<Result<Vec<_>>>()?;
    let rim_ids: Vec<_> = (0..8).chain(12..20).collect();
    let rims = rim_ids
        .iter()
        .map(|&edge| match &body.edges[edge].curve {
            Curve::Nurbs(curve) => Ok(curve.as_ref()),
            _ => Err(error()),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut world = 0f64;
    let mut ratio = 1f64;
    for surface in &surfaces {
        for p in surface.control_points() {
            world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
        let min = surface
            .weights()
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max = surface.weights().iter().copied().fold(0., f64::max);
        ratio = ratio.max(max / min);
    }
    let arithmetic = 65536. * f64::EPSILON * world.max(f64::MIN_POSITIVE) * ratio;
    if !arithmetic.is_finite() || arithmetic >= chord_error / 16. {
        return Err(error());
    }
    let mut n = 1usize;
    for surface in &surfaces {
        let cells = surface
            .tessellate_bounded(chord_error * 0.5, 4096)?
            .uv_ranges
            .len();
        let side = (cells as f64).sqrt() as usize;
        if side * side != cells || !side.is_power_of_two() {
            return Err(error());
        }
        n = n.max(side);
    }
    loop {
        if n > 64 {
            return Err(error());
        }
        let mut accepted = true;
        for rim in &rims {
            for i in 0..n {
                accepted &= rim_bound(
                    rim,
                    i as f64 / n as f64,
                    (i + 1) as f64 / n as f64,
                    arithmetic,
                )? <= chord_error * 0.75;
            }
        }
        if accepted {
            break;
        }
        n *= 2;
    }
    // Only actual topology identity chooses shared samples. There is no XYZ weld.
    let cache = body
        .edges
        .iter()
        .enumerate()
        .map(|(edge_id, edge)| {
            (0..=n)
                .map(|i| {
                    if i == 0 {
                        Ok(body.vertices[edge.vertices[0]].point)
                    } else if i == n {
                        Ok(body.vertices[edge.vertices[1]].point)
                    } else {
                        let mut t = i as f64 / n as f64;
                        if (8..12).contains(&edge_id) || (20..24).contains(&edge_id) {
                            let q = if edge_id < 12 {
                                edge_id - 8
                            } else {
                                edge_id - 20 + 4
                            };
                            let weights = surfaces[q].weights();
                            let scale = weights[0].max(weights[1]);
                            let a = weights[0] / scale;
                            let b = weights[1] / scale;
                            t = t * b / ((1. - t) * a + t * b);
                            if !t.is_finite() || t <= 0. || t >= 1. {
                                return Err(error());
                            }
                        }
                        edge.curve.try_evaluate(t)
                    }
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut mesh = Mesh::default();
    for (q, surface) in surfaces.iter().enumerate() {
        let (face, bottom, top, generator) = indices(q);
        let next = if q < 4 {
            8 + (q + 1) % 4
        } else {
            20 + (q - 4 + 1) % 4
        };
        let mut positions = Vec::with_capacity((n + 1) * (n + 1));
        let mut normals = Vec::with_capacity(positions.capacity());
        let mut mismatch = 0f64;
        for i in 0..=n {
            for j in 0..=n {
                let u = i as f64 / n as f64;
                let v = j as f64 / n as f64;
                let actual = surface.evaluate(u, v)?;
                let point = if j == 0 {
                    cache[bottom][i]
                } else if j == n {
                    cache[top][i]
                } else if i == 0 {
                    cache[generator][j]
                } else if i == n {
                    cache[next][j]
                } else {
                    actual
                };
                mismatch = mismatch.max(norm(actual - point));
                positions.push(point);
                normals.push(surface.normal(u, v)?);
            }
        }
        if !mismatch.is_finite() || mismatch + arithmetic >= chord_error * 0.25 {
            return Err(error());
        }
        for i in 0..n {
            for j in 0..n {
                let cell = surface.restricted([
                    [i as f64 / n as f64, (i + 1) as f64 / n as f64],
                    [j as f64 / n as f64, (j + 1) as f64 / n as f64],
                ])?;
                let bounded =
                    cell.tessellate_bounded(chord_error * 0.75 - mismatch - arithmetic, 1)?;
                if bounded.error_bounds.len() != 1
                    || bounded.error_bounds[0] + mismatch + arithmetic > chord_error
                {
                    return Err(error());
                }
                let a = i * (n + 1) + j;
                let b = (i + 1) * (n + 1) + j;
                let c = b + 1;
                let d = a + 1;
                for triangle in [[a, b, c], [a, c, d]] {
                    emit(
                        &mut mesh,
                        triangle.map(|k| positions[k]),
                        triangle.map(|k| normals[k]),
                        face,
                        body.shell.faces[face].orientation,
                    )?;
                }
            }
        }
    }
    for cap in 0..2 {
        let face = &body.shell.faces[cap];
        let normal = face.surface.normal_at(0., 0.)?;
        for quarter in 0..4 {
            let outer = 4 * cap + quarter;
            let inner = 12 + 4 * cap + quarter;
            for i in 0..n {
                let mut inner_allowance = 0.;
                for edge in [outer, inner] {
                    let Curve::Nurbs(curve) = &body.edges[edge].curve else {
                        return Err(error());
                    };
                    let a = i as f64 / n as f64;
                    let b = (i + 1) as f64 / n as f64;
                    let replacement = norm(cache[edge][i] - curve.evaluate(a)?)
                        .max(norm(cache[edge][i + 1] - curve.evaluate(b)?));
                    let bound = rim_bound(curve, a, b, arithmetic)? + replacement;
                    if edge == inner {
                        inner_allowance = bound;
                    }
                    if bound > chord_error {
                        return Err(error());
                    }
                }
                for points in [
                    [cache[inner][i], cache[outer][i], cache[outer][i + 1]],
                    [cache[inner][i], cache[outer][i + 1], cache[inner][i + 1]],
                ] {
                    let frame = source_frame;
                    let local = points.map(|p| {
                        let d = p - frame.origin();
                        [d.dot(frame.axes()[0]), d.dot(frame.axes()[1])]
                    });
                    if triangle_radius(local) + inner_allowance < hole_radius {
                        return Err(error());
                    }
                    emit(&mut mesh, points, [normal; 3], cap, face.orientation)?;
                }
            }
        }
    }
    if !mesh.signed_volume().is_finite() || mesh.signed_volume() <= 0. {
        return Err(error());
    }
    Ok(mesh)
}
