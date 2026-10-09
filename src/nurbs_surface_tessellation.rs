//! Bounded tensor-Bernstein display with explicit one-sided crease normals.
use crate::nurbs::{project, weighted_controls};
use crate::{Error, KnotSide, Mesh, NurbsSurface, Point3, Result, Vec3};

#[derive(Clone, Debug)]
pub struct NurbsSurfaceMesh {
    pub mesh: Mesh,
    /// Original UV coordinate for each display vertex.
    pub vertex_uv: Vec<[f64; 2]>,
    /// Canonical geometric node per display vertex; crease duplicates share it.
    pub vertex_nodes: Vec<usize>,
    /// Explicit source derivative limits used for each display normal.
    pub normal_sides: Vec<[KnotSide; 2]>,
    /// One original-parameter rectangle per cell; each cell emits two triangles.
    pub uv_ranges: Vec<[[f64; 2]; 2]>,
    /// One bound per rectangle, shared by its two emitted triangles.
    pub error_bounds: Vec<f64>,
}
#[derive(Clone)]
struct Cell {
    h: Vec<[f64; 4]>,
    uv: [[f64; 2]; 2],
}
impl NurbsSurface {
    /// Conforming bounded tessellation of positive rational Bezier spans.
    /// C0 knot lines retain separate one-sided display normals. All spans share one dyadic level.
    pub fn tessellate_bounded(
        &self,
        chord_error: f64,
        max_cells: usize,
    ) -> Result<NurbsSurfaceMesh> {
        if !chord_error.is_finite() || chord_error <= 0. || !(1..=65536).contains(&max_cells) {
            return Err(Error::InvalidInput(
                "surface tessellation needs finite positive error and 1..65536 cells",
            ));
        }
        let [p, q] = self.degrees();
        let [nu, nv] = [p + 1, q + 1];
        let mut spans = [0usize; 2];
        for (axis, span_count) in spans.iter_mut().enumerate() {
            let knots = self.knots(axis)?;
            *span_count = knots.windows(2).filter(|pair| pair[0] < pair[1]).count();
        }
        if spans[0].saturating_mul(spans[1]) > max_cells {
            return Err(Error::Tessellation(
                "surface knot span count exceeds cell limit",
            ));
        }
        let scale = self
            .control_points()
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(0., f64::max)
            .max(f64::MIN_POSITIVE);
        let min = self.weights().iter().copied().fold(f64::INFINITY, f64::min);
        let max = self.weights().iter().copied().fold(0., f64::max);
        let arithmetic = 4096. * f64::EPSILON * scale * (max / min) * (p + q + 2) as f64;
        if !arithmetic.is_finite() || arithmetic >= chord_error * 0.25 {
            return Err(Error::Tessellation(
                "surface coordinate/weight precision cannot resolve requested error",
            ));
        }
        let origin = self.control_points()[0];
        let patches = self.bezier_patches()?;
        let mut cells = Vec::with_capacity(patches.len());
        for patch in patches {
            let points = patch
                .surface
                .control_points()
                .iter()
                .map(|point| *point - origin)
                .collect::<Vec<_>>();
            cells.push(Cell {
                h: weighted_controls(&points, patch.surface.weights())?,
                uv: patch.parameter_ranges,
            });
        }
        for depth in 0..=8 {
            let mut bounds = Vec::with_capacity(cells.len());
            let mut all = true;
            for cell in &cells {
                let bound = cell_bound(self, cell, p, q, origin)? + arithmetic;
                if !bound.is_finite() {
                    return Err(Error::Tessellation("surface bound exceeds finite range"));
                }
                all &= bound <= chord_error;
                bounds.push(bound);
            }
            if all {
                return emit(self, cells, bounds);
            }
            if depth == 8 || cells.len() > max_cells / 4 {
                return Err(Error::Tessellation(
                    "surface tessellation exceeds depth or cell limit",
                ));
            }
            if cells
                .len()
                .saturating_mul(4)
                .saturating_mul(nu)
                .saturating_mul(nv)
                > 16_000_000
            {
                return Err(Error::Unsupported(
                    "surface tessellation exceeds control work limit",
                ));
            }
            let mut next = Vec::with_capacity(cells.len() * 4);
            for cell in cells {
                let [[u0, u1], [v0, v1]] = cell.uv;
                let um = u0 + (u1 - u0) * 0.5;
                let vm = v0 + (v1 - v0) * 0.5;
                if um == u0 || um == u1 || vm == v0 || vm == v1 {
                    return Err(Error::Tessellation(
                        "surface parameter midpoint cannot be represented",
                    ));
                }
                let (left, right) = split_axis(&cell.h, nu, nv, 0, (um - u0) / (u1 - u0));
                for (h, urange) in [(left, [u0, um]), (right, [um, u1])] {
                    let (bottom, top) = split_axis(&h, nu, nv, 1, (vm - v0) / (v1 - v0));
                    next.push(Cell {
                        h: bottom,
                        uv: [urange, [v0, vm]],
                    });
                    next.push(Cell {
                        h: top,
                        uv: [urange, [vm, v1]],
                    });
                }
            }
            cells = next;
        }
        unreachable!()
    }
}
fn cell_bound(
    surface: &NurbsSurface,
    cell: &Cell,
    p: usize,
    q: usize,
    origin: Point3,
) -> Result<f64> {
    let nv = q + 1;
    let corners = [
        project(cell.h[0])?,
        project(cell.h[p * nv])?,
        project(cell.h[p * nv + q])?,
        project(cell.h[q])?,
    ];
    let [[u0, u1], [v0, v1]] = cell.uv;
    let uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
    let mut mismatch = 0f64;
    for (corner, uv) in corners.iter().zip(uv) {
        let delta = (surface.evaluate(uv[0], uv[1])? - origin - *corner).norm();
        if !delta.is_finite() {
            return Err(Error::Tessellation(
                "surface endpoint agreement exceeds finite range",
            ));
        }
        mismatch = mismatch.max(delta);
    }
    let min_weight = cell.h.iter().map(|v| v[3]).fold(f64::INFINITY, f64::min);
    if min_weight <= 0. || !min_weight.is_finite() {
        return Err(Error::Tessellation(
            "surface homogeneous denominator is unusable",
        ));
    }
    let mut max = 0f64;
    for k in 0..=p + 1 {
        for l in 0..=q + 1 {
            let mut d = Vec3::new(0., 0., 0.);
            for a in 0..=1 {
                for b in 0..=1 {
                    if k < a || l < b || k - a > p || l - b > q {
                        continue;
                    }
                    let fu = if a == 0 {
                        (p + 1 - k) as f64 / (p + 1) as f64
                    } else {
                        k as f64 / (p + 1) as f64
                    };
                    let fv = if b == 0 {
                        (q + 1 - l) as f64 / (q + 1) as f64
                    } else {
                        l as f64 / (q + 1) as f64
                    };
                    let h = cell.h[(k - a) * nv + l - b];
                    let corner = corners[match (a, b) {
                        (0, 0) => 0,
                        (1, 0) => 1,
                        (1, 1) => 2,
                        _ => 3,
                    }];
                    d = d + (Vec3::new(h[0], h[1], h[2]) - corner * h[3]) * (fu * fv);
                }
            }
            let norm = d.norm();
            if !norm.is_finite() {
                return Err(Error::Tessellation(
                    "surface Bernstein coefficient exceeds finite range",
                ));
            }
            max = max.max(norm);
        }
    }
    let twist = (corners[0] - corners[1] + corners[2] - corners[3]).norm() * 0.25;
    Ok(max / min_weight + twist + mismatch)
}
fn split_axis(
    h: &[[f64; 4]],
    nu: usize,
    nv: usize,
    axis: usize,
    fraction: f64,
) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut left = vec![[0.; 4]; h.len()];
    let mut right = left.clone();
    let (count, rows) = if axis == 0 { (nu, nv) } else { (nv, nu) };
    let index = |i, j| if axis == 0 { i * nv + j } else { j * nv + i };
    for row in 0..rows {
        let mut work = (0..count).map(|i| h[index(i, row)]).collect::<Vec<_>>();
        left[index(0, row)] = work[0];
        right[index(count - 1, row)] = work[count - 1];
        for r in 1..count {
            for i in 0..count - r {
                let next = work[i + 1];
                for (value, next) in work[i].iter_mut().zip(next) {
                    *value = (1. - fraction) * *value + fraction * next;
                }
            }
            left[index(r, row)] = work[0];
            right[index(count - 1 - r, row)] = work[count - 1 - r];
        }
    }
    (left, right)
}
fn emit(surface: &NurbsSurface, cells: Vec<Cell>, bounds: Vec<f64>) -> Result<NurbsSurfaceMesh> {
    let mut mesh = Mesh::default();
    let mut uv_ranges = Vec::with_capacity(cells.len());
    let mut shared = std::collections::BTreeMap::new();
    let mut geometric = std::collections::BTreeMap::new();
    let mut vertex_uv = Vec::new();
    let mut vertex_nodes = Vec::new();
    let mut normal_sides = Vec::new();
    let mut creases: [std::collections::BTreeSet<u64>; 2] =
        std::array::from_fn(|_| std::collections::BTreeSet::new());
    for (axis, set) in creases.iter_mut().enumerate() {
        let degree = surface.degrees()[axis];
        let knots = surface.knots(axis)?;
        let count = surface.control_counts()[axis];
        let mut i = degree + 1;
        while i < count {
            let next = knots.partition_point(|v| *v <= knots[i]);
            if next - i == degree {
                set.insert(canonical_parameter(knots[i]).to_bits());
            }
            i = next;
        }
    }
    for cell in cells {
        let [[u0, u1], [v0, v1]] = cell.uv;
        let mut ids = [0usize; 4];
        for (i, [u, v]) in [[u0, v0], [u1, v0], [u1, v1], [u0, v1]]
            .into_iter()
            .enumerate()
        {
            // Signed zero is one numerical parameter, regardless of knot spelling.
            let u = canonical_parameter(u);
            let v = canonical_parameter(v);
            let uv_key = (u.to_bits(), v.to_bits());
            let sides = [
                if creases[0].contains(&u.to_bits()) && u == u1 {
                    KnotSide::Left
                } else {
                    KnotSide::Right
                },
                if creases[1].contains(&v.to_bits()) && v == v1 {
                    KnotSide::Left
                } else {
                    KnotSide::Right
                },
            ];
            let key = (
                uv_key,
                sides[0] == KnotSide::Left,
                sides[1] == KnotSide::Left,
            );
            ids[i] = if let Some(id) = shared.get(&key) {
                *id
            } else {
                let (node, point) = if let Some(value) = geometric.get(&uv_key) {
                    *value
                } else {
                    let value = (geometric.len(), surface.evaluate(u, v)?);
                    geometric.insert(uv_key, value);
                    value
                };
                let normal = surface.evaluate_with_partials(u, v, sides)?.normal()?;
                let id = mesh.positions.len();
                mesh.positions.push(point);
                mesh.normals.push(normal);
                vertex_uv.push([u, v]);
                vertex_nodes.push(node);
                normal_sides.push(sides);
                shared.insert(key, id);
                id
            };
        }
        for tri in [[ids[0], ids[1], ids[2]], [ids[0], ids[2], ids[3]]] {
            let cross = (mesh.positions[tri[1]] - mesh.positions[tri[0]])
                .cross(mesh.positions[tri[2]] - mesh.positions[tri[0]]);
            let normal = cross
                .normalized()
                .map_err(|_| Error::Tessellation("bounded surface triangle is degenerate"))?;
            if tri.iter().any(|i| normal.dot(mesh.normals[*i]) <= 0.) {
                return Err(Error::Tessellation(
                    "bounded surface triangle orientation disagrees with analytic normal",
                ));
            }
            mesh.triangles.push(tri);
            mesh.face_ids.push(0);
        }
        uv_ranges.push(cell.uv);
    }
    Ok(NurbsSurfaceMesh {
        mesh,
        uv_ranges,
        error_bounds: bounds,
        vertex_uv,
        vertex_nodes,
        normal_sides,
    })
}

fn canonical_parameter(parameter: f64) -> f64 {
    if parameter == 0.0 {
        0.0
    } else {
        parameter
    }
}
