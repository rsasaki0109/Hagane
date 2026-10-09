//! Conforming bounded display for one equal-weight bilinear polygon face.
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
    /// degree-(1,1), equal-weight patch, including genuinely curved saddles.
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
        if s.degrees() != [1, 1]
            || s.control_points().len() != 4
            || s.weights().iter().any(|w| *w != s.weights()[0])
        {
            return Err(Error::Unsupported(
                "bounded polygon display requires one equal-weight bilinear patch",
            ));
        }
        let p = s.control_points();
        let twist = ((p[3] - p[2]) - (p[1] - p[0])).norm();
        let domains = [s.knots(0)?, s.knots(1)?];
        let widths = [domains[0][2] - domains[0][1], domains[1][2] - domains[1][1]];
        let scale = p
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(f64::MIN_POSITIVE, f64::max);
        let diameter = p
            .iter()
            .flat_map(|a| p.iter().map(move |b| (*a - *b).norm()))
            .fold(0f64, f64::max);
        let mut arithmetic = 65536. * f64::EPSILON * scale;
        for axis in 0..2 {
            let parameter_scale = domains[axis]
                .iter()
                .map(|x| x.abs())
                .fold(f64::MIN_POSITIVE, f64::max);
            arithmetic += 128. * f64::EPSILON * parameter_scale * (diameter / widths[axis]);
        }
        if !twist.is_finite() || !arithmetic.is_finite() || arithmetic >= error {
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
                    // Constant and affine terms cancel under barycentric interpolation.
                    // Both the normalized UV product and its interpolation lie in
                    // [0, du*dv] after shifting to this triangle's rectangle origin.
                    twist * ((hi[0] - lo[0]) / widths[0]) * ((hi[1] - lo[1]) / widths[1])
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
