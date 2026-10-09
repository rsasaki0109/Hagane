//! Conforming bounded display for one positive-weight rational bilinear polygon face.
use crate::*;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct NurbsPolygonMesh {
    pub mesh: Mesh,
    pub vertex_uv: Vec<[f64; 2]>,
    /// One engineering bound per triangle at its original UV coordinates.
    pub error_bounds: Vec<f64>,
}
impl NurbsPolygonFace {
    /// Display the retained polygon, not a mesh Boolean. Initial support is one
    /// degree-(1,1), positive-weight rational patch, including genuinely curved saddles.
    pub fn tessellate_bilinear_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsPolygonMesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. || !(1..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "polygon display needs positive error and 1..65536 triangles",
            ));
        }
        let s = &self.boundary.surface;
        if s.degrees() != [1, 1] || s.control_points().len() != 4 {
            return Err(Error::Unsupported(
                "bounded polygon display requires one rational bilinear patch",
            ));
        }
        let p = s.control_points();
        let derivatives = rational_derivative_bounds(s)?;
        let ratio = s.weights().iter().copied().fold(0f64, f64::max)
            / s.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let domains = [s.knots(0)?, s.knots(1)?];
        let widths = [domains[0][2] - domains[0][1], domains[1][2] - domains[1][1]];
        let scale = p
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(f64::MIN_POSITIVE, f64::max);
        let mut arithmetic = 65536. * f64::EPSILON * scale * ratio * ratio;
        for axis in 0..2 {
            let parameter_scale = domains[axis]
                .iter()
                .map(|x| x.abs())
                .fold(f64::MIN_POSITIVE, f64::max);
            arithmetic +=
                128. * f64::EPSILON * parameter_scale * (derivatives[3 + axis] / widths[axis]);
        }
        if !arithmetic.is_finite() || arithmetic >= error {
            return Err(Error::Tessellation(
                "bilinear polygon precision cannot resolve requested error",
            ));
        }
        let mut uv = self.boundary.uv_corners().to_vec();
        let mut triangles: Vec<[usize; 3]> = (1..uv.len() - 1).map(|i| [0, i, i + 1]).collect();
        for depth in 0..=10 {
            if triangles.len() > max_triangles {
                return Err(Error::Tessellation(
                    "polygon display exceeds triangle budget",
                ));
            }
            let bounds: Vec<f64> = triangles
                .iter()
                .map(|t| {
                    let mut lo = uv[t[0]];
                    let mut hi = lo;
                    for &id in &t[1..] {
                        for a in 0..2 {
                            lo[a] = lo[a].min(uv[id][a]);
                            hi[a] = hi[a].max(uv[id][a]);
                        }
                    }
                    // Taylor remainder about the interpolated UV point. First
                    // order terms cancel in barycentric interpolation.
                    let du = (hi[0] - lo[0]) / widths[0];
                    let dv = (hi[1] - lo[1]) / widths[1];
                    0.5 * (derivatives[0] * du * du
                        + 2. * derivatives[1] * du * dv
                        + derivatives[2] * dv * dv)
                        + arithmetic
                })
                .collect();
            if bounds.iter().all(|b| b.is_finite() && *b <= error) {
                let mut positions = Vec::with_capacity(uv.len());
                let mut normals = Vec::with_capacity(uv.len());
                for point in &uv {
                    let e = s.evaluate_with_partials(point[0], point[1], [KnotSide::Right; 2])?;
                    positions.push(e.point);
                    normals.push(e.du.cross(e.dv).normalized()? * self.face.orientation as f64);
                }
                for t in &mut triangles {
                    if orient2d(uv[t[0]], uv[t[1]], uv[t[2]])? != Orientation::CounterClockwise {
                        return Err(Error::Tessellation(
                            "polygon UV triangle orientation is unresolved",
                        ));
                    }
                    if self.face.orientation < 0 {
                        t.swap(1, 2);
                    }
                    let cross = (positions[t[1]] - positions[t[0]])
                        .cross(positions[t[2]] - positions[t[0]]);
                    if !cross.finite() || cross.norm() == 0. {
                        return Err(Error::Tessellation(
                            "polygon display has degenerate triangle",
                        ));
                    }
                }
                let face_ids = vec![0; triangles.len()];
                return Ok(NurbsPolygonMesh {
                    mesh: Mesh {
                        positions,
                        normals,
                        triangles,
                        face_ids,
                    },
                    vertex_uv: uv,
                    error_bounds: bounds,
                });
            }
            if depth == 10 || triangles.len() > max_triangles / 4 {
                return Err(Error::Tessellation(
                    "polygon display exceeds depth or triangle budget",
                ));
            }
            let mut midpoints = HashMap::new();
            let mut next = Vec::with_capacity(triangles.len() * 4);
            for [a, b, c] in triangles {
                let mut middle = |a: usize, b: usize| -> Result<usize> {
                    let key = if a < b { [a, b] } else { [b, a] };
                    if let Some(id) = midpoints.get(&key) {
                        return Ok(*id);
                    }
                    let point = [
                        uv[a][0] + (uv[b][0] - uv[a][0]) * 0.5,
                        uv[a][1] + (uv[b][1] - uv[a][1]) * 0.5,
                    ];
                    if point == uv[a] || point == uv[b] {
                        return Err(Error::Tessellation("polygon UV midpoint is unresolved"));
                    }
                    let id = uv.len();
                    uv.push(point);
                    midpoints.insert(key, id);
                    Ok(id)
                };
                let ab = middle(a, b)?;
                let bc = middle(b, c)?;
                let ca = middle(c, a)?;
                next.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            }
            triangles = next;
        }
        unreachable!()
    }
}

// Bounds in normalized patch coordinates for S=H/W, with positive W.
// Differentiate H=W*S twice; H_uu=W_uu=H_vv=W_vv=0.
fn rational_derivative_bounds(s: &NurbsSurface) -> Result<[f64; 5]> {
    let points = s.control_points();
    let maximum = s.weights().iter().copied().fold(0f64, f64::max);
    let w: Vec<f64> = s.weights().iter().map(|x| x / maximum).collect();
    let minimum = w.iter().copied().fold(f64::INFINITY, f64::min);
    let diameter = points
        .iter()
        .flat_map(|a| points.iter().map(move |b| (*a - *b).norm()))
        .fold(0f64, f64::max);
    let mut h = Vec::new();
    for (p, w) in points.iter().zip(&w) {
        let local = *p - points[0];
        let weighted = local * *w;
        if !weighted.finite()
            || [local.x, local.y, local.z]
                .into_iter()
                .zip([weighted.x, weighted.y, weighted.z])
                .any(|(a, b)| a != 0. && b == 0.)
        {
            return Err(Error::Tessellation(
                "rational polygon homogeneous controls exceed numerical range",
            ));
        }
        h.push(weighted);
    }
    let hu = (h[2] - h[0]).norm().max((h[3] - h[1]).norm());
    let hv = (h[1] - h[0]).norm().max((h[3] - h[2]).norm());
    let huv = ((h[3] - h[2]) - (h[1] - h[0])).norm();
    let wu = (w[2] - w[0]).abs().max((w[3] - w[1]).abs());
    let wv = (w[1] - w[0]).abs().max((w[3] - w[2]).abs());
    let wuv = ((w[3] - w[2]) - (w[1] - w[0])).abs();
    let su = (hu + diameter * wu) / minimum;
    let sv = (hv + diameter * wv) / minimum;
    let bounds = [
        2. * wu * su / minimum,
        (huv + diameter * wuv + su * wv + sv * wu) / minimum,
        2. * wv * sv / minimum,
        su,
        sv,
    ];
    if minimum <= 0. || !diameter.is_finite() || bounds.iter().any(|x| !x.is_finite()) {
        return Err(Error::Tessellation(
            "rational polygon derivative bound exceeds numerical range",
        ));
    }
    Ok(bounds)
}
