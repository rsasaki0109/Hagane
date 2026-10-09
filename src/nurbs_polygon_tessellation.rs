//! Conforming bounded display for C1 positive-weight rational polygon faces.
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
        if self.boundary.surface.degrees() != [1, 1] {
            return Err(Error::Unsupported("bilinear display requires degree (1,1)"));
        }
        self.tessellate_bounded(error, max_triangles, tol)
    }
    /// Bounded convex polygon display across structurally C1 rational spans.
    /// C0 knots remain unsupported; boundary/extraction/work limits apply.
    pub fn tessellate_bounded(
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
        let degrees = s.degrees();
        let p = s.control_points();
        let derivatives = surface_derivative_bounds(s)?;
        let ratio = s.weights().iter().copied().fold(0f64, f64::max)
            / s.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let domains = [s.knots(0)?, s.knots(1)?];
        let ranges = s.domain();
        let widths = [ranges[0][1] - ranges[0][0], ranges[1][1] - ranges[1][0]];
        let scale = p
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(f64::MIN_POSITIVE, f64::max);
        let mut arithmetic =
            65536. * f64::EPSILON * scale * ratio * ratio * (degrees[0] + degrees[1] + 2) as f64
                / 4.;
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
                "rational polygon precision cannot resolve requested error",
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
// Differentiate H=W*S twice and bound each Bernstein derivative control net.
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
    let counts = [s.degrees()[0] + 1, s.degrees()[1] + 1];
    let (hu_net, wu_net, cu) = differentiate(&h, &w, counts, 0);
    let (hv_net, wv_net, cv) = differentiate(&h, &w, counts, 1);
    let (huu_net, wuu_net, _) = differentiate(&hu_net, &wu_net, cu, 0);
    let (hvv_net, wvv_net, _) = differentiate(&hv_net, &wv_net, cv, 1);
    let (huv_net, wuv_net, _) = differentiate(&hu_net, &wu_net, cu, 1);
    if [&hu_net, &hv_net, &huu_net, &hvv_net, &huv_net]
        .into_iter()
        .flatten()
        .any(|v| !v.finite())
    {
        return Err(Error::Tessellation(
            "rational polygon derivative controls exceed numerical range",
        ));
    }
    let norm = |net: &[Vec3]| net.iter().map(|v| v.norm()).fold(0f64, f64::max);
    let abs = |net: &[f64]| net.iter().map(|v| v.abs()).fold(0f64, f64::max);
    let (hu, hv, huu, huv, hvv) = (
        norm(&hu_net),
        norm(&hv_net),
        norm(&huu_net),
        norm(&huv_net),
        norm(&hvv_net),
    );
    let (wu, wv, wuu, wuv, wvv) = (
        abs(&wu_net),
        abs(&wv_net),
        abs(&wuu_net),
        abs(&wuv_net),
        abs(&wvv_net),
    );
    let su = (hu + diameter * wu) / minimum;
    let sv = (hv + diameter * wv) / minimum;
    let bounds = [
        (huu + diameter * wuu + 2. * wu * su) / minimum,
        (huv + diameter * wuv + su * wv + sv * wu) / minimum,
        (hvv + diameter * wvv + 2. * wv * sv) / minimum,
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

fn differentiate(
    h: &[Vec3],
    w: &[f64],
    counts: [usize; 2],
    axis: usize,
) -> (Vec<Vec3>, Vec<f64>, [usize; 2]) {
    if counts[axis] <= 1 {
        return (Vec::new(), Vec::new(), [0, 0]);
    }
    let degree = (counts[axis] - 1) as f64;
    let mut result = counts;
    result[axis] -= 1;
    let step = if axis == 0 { counts[1] } else { 1 };
    let mut dh = Vec::with_capacity(result[0] * result[1]);
    let mut dw = Vec::with_capacity(result[0] * result[1]);
    for i in 0..result[0] {
        for j in 0..result[1] {
            let index = i * counts[1] + j;
            dh.push((h[index + step] - h[index]) * degree);
            dw.push((w[index + step] - w[index]) * degree);
        }
    }
    (dh, dw, result)
}

// A C1 function has an absolutely continuous first derivative along each
// triangle segment. Piecewise Hessian bounds therefore bound its Taylor
// remainder across knot crossings, even where second derivatives jump.
fn surface_derivative_bounds(s: &NurbsSurface) -> Result<[f64; 5]> {
    let degrees = s.degrees();
    let domains = s.domain();
    let mut patch_count = 1usize;
    for axis in 0..2 {
        let knots = s.knots(axis)?;
        let mut index = 0;
        while index < knots.len() {
            let value = knots[index];
            let next = knots.partition_point(|v| *v <= value);
            if value > domains[axis][0] && value < domains[axis][1] && next - index >= degrees[axis]
            {
                return Err(Error::Unsupported(
                    "polygon display requires C1 source knots; C0 creases need explicit splitting",
                ));
            }
            index = next;
        }
        patch_count = patch_count.saturating_mul(knots.windows(2).filter(|v| v[0] < v[1]).count());
    }
    let net = (degrees[0] + 1) * (degrees[1] + 1);
    if patch_count.saturating_mul(net).saturating_mul(net) > 16_000_000 {
        return Err(Error::Unsupported(
            "polygon derivative bounds exceed control work limit",
        ));
    }
    if patch_count == 1 {
        return rational_derivative_bounds(s);
    }
    let mut result = [0f64; 5];
    for patch in s.bezier_patches()? {
        let local = rational_derivative_bounds(&patch.surface)?;
        let factors = [
            (domains[0][1] - domains[0][0])
                / (patch.parameter_ranges[0][1] - patch.parameter_ranges[0][0]),
            (domains[1][1] - domains[1][0])
                / (patch.parameter_ranges[1][1] - patch.parameter_ranges[1][0]),
        ];
        let scaling = [
            factors[0] * factors[0],
            factors[0] * factors[1],
            factors[1] * factors[1],
            factors[0],
            factors[1],
        ];
        for i in 0..5 {
            let bound = local[i] * scaling[i];
            if !bound.is_finite() {
                return Err(Error::Tessellation(
                    "polygon span derivative scaling exceeds numerical range",
                ));
            }
            result[i] = result[i].max(bound);
        }
    }
    Ok(result)
}
