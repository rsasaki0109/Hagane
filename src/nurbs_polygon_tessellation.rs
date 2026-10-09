//! Conforming bounded display for C1 positive-weight rational polygon faces.
use crate::*;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct NurbsPolygonMesh {
    pub mesh: Mesh,
    pub vertex_uv: Vec<[f64; 2]>,
    /// Canonical UV node; normal duplicates at creases share this identity.
    pub vertex_nodes: Vec<usize>,
    pub normal_sides: Vec<[KnotSide; 2]>,
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
        self.tessellate_polygon(error, max_triangles, tol, false, &[])
    }
    /// Split every structural C0 knot before display; duplicate only normals.
    pub fn tessellate_crease_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsPolygonMesh> {
        self.tessellate_polygon(error, max_triangles, tol, true, &[])
    }
    /// Display a previously validated B-rep region with rectangular UV holes.
    /// Cut at every hole boundary before masking and sampling retained triangles.
    pub(crate) fn tessellate_holed_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
        holes: &[[[f64; 2]; 2]],
    ) -> Result<NurbsPolygonMesh> {
        self.tessellate_polygon(error, max_triangles, tol, true, holes)
    }
    fn tessellate_polygon(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
        split_creases: bool,
        holes: &[[[f64; 2]; 2]],
    ) -> Result<NurbsPolygonMesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. || !(1..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "polygon display needs positive error and 1..65536 triangles",
            ));
        }
        if holes.len() > 16
            || holes.iter().any(|hole| {
                (0..2).any(|axis| {
                    !hole[axis][0].is_finite()
                        || !hole[axis][1].is_finite()
                        || hole[axis][0] >= hole[axis][1]
                })
            })
        {
            return Err(Error::InvalidInput(
                "polygon display hole ranges are invalid",
            ));
        }
        let s = &self.boundary.surface;
        let degrees = s.degrees();
        let p = s.control_points();
        let derivatives = surface_derivative_bounds(s, split_creases)?;
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
        let creases = if split_creases {
            source_creases(s)?
        } else {
            [Vec::new(), Vec::new()]
        };
        if holes.is_empty() {
            let mut clipping_work = 0usize;
            for (axis, values) in creases.iter().enumerate() {
                for &value in values {
                    clipping_work = clipping_work.saturating_add(triangles.len());
                    if clipping_work > 16_000_000 {
                        return Err(Error::Unsupported(
                            "polygon crease clipping exceeds work limit",
                        ));
                    }
                    triangles = split_triangles(&mut uv, triangles, axis, value, max_triangles)?;
                }
            }
        } else {
            triangles = partition_material_cells(&mut uv, holes, &creases, max_triangles)?;
        }
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
                return emit_polygon(s, uv, triangles, bounds, creases, self.face.orientation);
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
fn surface_derivative_bounds(s: &NurbsSurface, allow_creases: bool) -> Result<[f64; 5]> {
    let degrees = s.degrees();
    let domains = s.domain();
    let mut patch_count = 1usize;
    for axis in 0..2 {
        let knots = s.knots(axis)?;
        let mut index = 0;
        while index < knots.len() {
            let value = knots[index];
            let next = knots.partition_point(|v| *v <= value);
            if !allow_creases
                && value > domains[axis][0]
                && value < domains[axis][1]
                && next - index >= degrees[axis]
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

fn source_creases(s: &NurbsSurface) -> Result<[Vec<f64>; 2]> {
    let mut result = [Vec::new(), Vec::new()];
    for (axis, values) in result.iter_mut().enumerate() {
        let knots = s.knots(axis)?;
        let domain = s.domain()[axis];
        let mut i = 0;
        while i < knots.len() {
            let value = knots[i];
            let next = knots.partition_point(|v| *v <= value);
            if value > domain[0] && value < domain[1] && next - i == s.degrees()[axis] {
                values.push(value);
            }
            i = next;
        }
    }
    Ok(result)
}
// Partition the exact outer UV polygon before creating artificial diagonals.
// Every coordinate-line intersection is shared by adjacent convex cells.
fn partition_material_cells(
    uv: &mut Vec<[f64; 2]>,
    holes: &[[[f64; 2]; 2]],
    creases: &[Vec<f64>; 2],
    max_triangles: usize,
) -> Result<Vec<[usize; 3]>> {
    let mut cells = vec![(0..uv.len()).collect::<Vec<_>>()];
    let mut lines = creases.clone();
    for hole in holes {
        for axis in 0..2 {
            lines[axis].extend(hole[axis]);
        }
    }
    for values in &mut lines {
        values.sort_by(f64::total_cmp);
        values.dedup();
    }
    let mut work = 0usize;
    for (axis, values) in lines.iter().enumerate() {
        for &value in values {
            let mut crossings = HashMap::new();
            let mut next = Vec::new();
            let mut count = 0usize;
            for cell in cells {
                work = work.saturating_add(cell.len());
                if work > 16_000_000 {
                    return Err(Error::Unsupported(
                        "polygon region clipping exceeds work limit",
                    ));
                }
                let lo = cell
                    .iter()
                    .map(|id| uv[*id][axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cell
                    .iter()
                    .map(|id| uv[*id][axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                if lo >= value || hi <= value {
                    count += cell.len() - 2;
                    next.push(cell);
                } else {
                    for left in [true, false] {
                        let mut polygon = Vec::new();
                        for index in 0..cell.len() {
                            let a = cell[index];
                            let b = cell[(index + 1) % cell.len()];
                            let inside = |x: f64| if left { x <= value } else { x >= value };
                            let ia = inside(uv[a][axis]);
                            let ib = inside(uv[b][axis]);
                            if ia {
                                polygon.push(a);
                            }
                            if ia != ib {
                                let id = if uv[a][axis] == value {
                                    a
                                } else if uv[b][axis] == value {
                                    b
                                } else {
                                    let key = if a < b { [a, b] } else { [b, a] };
                                    if let Some(id) = crossings.get(&key) {
                                        *id
                                    } else {
                                        let t = (value - uv[a][axis]) / (uv[b][axis] - uv[a][axis]);
                                        if !t.is_finite() || t <= 0. || t >= 1. {
                                            return Err(Error::Tessellation(
                                                "polygon region intersection is unresolved",
                                            ));
                                        }
                                        let other = 1 - axis;
                                        let mut point = uv[a];
                                        point[axis] = value;
                                        point[other] =
                                            uv[a][other] + t * (uv[b][other] - uv[a][other]);
                                        if !point[other].is_finite()
                                            || point == uv[a]
                                            || point == uv[b]
                                        {
                                            return Err(Error::Tessellation(
                                                "polygon region UV intersection collapses",
                                            ));
                                        }
                                        let id = uv.len();
                                        uv.push(point);
                                        crossings.insert(key, id);
                                        id
                                    }
                                };
                                polygon.push(id);
                            }
                        }
                        polygon.dedup();
                        if polygon.len() > 1 && polygon.first() == polygon.last() {
                            polygon.pop();
                        }
                        if polygon.len() < 3 {
                            return Err(Error::Tessellation("polygon region cell collapses"));
                        }
                        count += polygon.len() - 2;
                        next.push(polygon);
                    }
                }
                if count > max_triangles {
                    return Err(Error::Tessellation(
                        "polygon region clipping exceeds triangle budget",
                    ));
                }
            }
            cells = next;
        }
    }
    let mut triangles = Vec::new();
    for cell in cells {
        let mut lo = uv[cell[0]];
        let mut hi = lo;
        for &id in &cell[1..] {
            for axis in 0..2 {
                lo[axis] = lo[axis].min(uv[id][axis]);
                hi[axis] = hi[axis].max(uv[id][axis]);
            }
        }
        if holes
            .iter()
            .any(|h| (0..2).all(|axis| lo[axis] >= h[axis][0] && hi[axis] <= h[axis][1]))
        {
            continue;
        }
        if holes
            .iter()
            .any(|h| (0..2).all(|axis| lo[axis] < h[axis][1] && hi[axis] > h[axis][0]))
        {
            return Err(Error::Tessellation("polygon hole clipping is unresolved"));
        }
        for i in 1..cell.len() - 1 {
            triangles.push([cell[0], cell[i], cell[i + 1]]);
        }
    }
    if triangles.is_empty() {
        return Err(Error::Tessellation(
            "polygon holes leave no display material",
        ));
    }
    Ok(triangles)
}
fn split_triangles(
    uv: &mut Vec<[f64; 2]>,
    triangles: Vec<[usize; 3]>,
    axis: usize,
    value: f64,
    max_triangles: usize,
) -> Result<Vec<[usize; 3]>> {
    let mut output = Vec::new();
    let mut crossings = HashMap::new();
    for triangle in triangles {
        let lo = triangle
            .iter()
            .map(|id| uv[*id][axis])
            .fold(f64::INFINITY, f64::min);
        let hi = triangle
            .iter()
            .map(|id| uv[*id][axis])
            .fold(f64::NEG_INFINITY, f64::max);
        if lo >= value || hi <= value {
            output.push(triangle);
        } else {
            for left in [true, false] {
                let mut polygon = Vec::new();
                for index in 0..3 {
                    let a = triangle[index];
                    let b = triangle[(index + 1) % 3];
                    let inside = |x: f64| if left { x <= value } else { x >= value };
                    let ia = inside(uv[a][axis]);
                    let ib = inside(uv[b][axis]);
                    if ia {
                        polygon.push(a);
                    }
                    if ia != ib {
                        let id = if uv[a][axis] == value {
                            a
                        } else if uv[b][axis] == value {
                            b
                        } else {
                            let key = if a < b { [a, b] } else { [b, a] };
                            if let Some(id) = crossings.get(&key) {
                                *id
                            } else {
                                let t = (value - uv[a][axis]) / (uv[b][axis] - uv[a][axis]);
                                if !t.is_finite() || t <= 0. || t >= 1. {
                                    return Err(Error::Tessellation(
                                        "polygon crease intersection is unresolved",
                                    ));
                                }
                                let other = 1 - axis;
                                let mut point = uv[a];
                                point[axis] = value;
                                point[other] = uv[a][other] + t * (uv[b][other] - uv[a][other]);
                                if !point[other].is_finite() || point == uv[a] || point == uv[b] {
                                    return Err(Error::Tessellation(
                                        "polygon crease UV intersection collapses",
                                    ));
                                }
                                let id = uv.len();
                                uv.push(point);
                                crossings.insert(key, id);
                                id
                            }
                        };
                        polygon.push(id);
                    }
                }
                polygon.dedup();
                if polygon.len() > 1 && polygon.first() == polygon.last() {
                    polygon.pop();
                }
                for i in 1..polygon.len().saturating_sub(1) {
                    output.push([polygon[0], polygon[i], polygon[i + 1]]);
                }
            }
        }
        if output.len() > max_triangles {
            return Err(Error::Tessellation(
                "polygon crease clipping exceeds triangle budget",
            ));
        }
    }
    Ok(output)
}
fn emit_polygon(
    s: &NurbsSurface,
    uv: Vec<[f64; 2]>,
    triangles: Vec<[usize; 3]>,
    bounds: Vec<f64>,
    creases: [Vec<f64>; 2],
    orientation: i8,
) -> Result<NurbsPolygonMesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut vertex_uv = Vec::new();
    let mut vertex_nodes = Vec::new();
    let mut normal_sides = Vec::new();
    let mut emitted = Vec::with_capacity(triangles.len());
    let mut display_nodes = HashMap::new();
    let mut geometry = HashMap::new();
    for triangle in triangles {
        if orient2d(uv[triangle[0]], uv[triangle[1]], uv[triangle[2]])?
            != Orientation::CounterClockwise
        {
            return Err(Error::Tessellation(
                "polygon UV triangle orientation is unresolved",
            ));
        }
        let mut ids = [0; 3];
        for (index, node) in triangle.into_iter().enumerate() {
            let mut sides = [KnotSide::Right; 2];
            let mut flags = 0usize;
            for axis in 0..2 {
                if creases[axis].contains(&uv[node][axis])
                    && triangle.iter().all(|id| uv[*id][axis] <= uv[node][axis])
                {
                    sides[axis] = KnotSide::Left;
                    flags |= 1 << axis;
                }
            }
            let key = (node, flags);
            ids[index] = if let Some(id) = display_nodes.get(&key) {
                *id
            } else {
                let point = if let Some(point) = geometry.get(&node) {
                    *point
                } else {
                    let point = s.evaluate(uv[node][0], uv[node][1])?;
                    geometry.insert(node, point);
                    point
                };
                let e = s.evaluate_with_partials(uv[node][0], uv[node][1], sides)?;
                let normal = e.du.cross(e.dv).normalized()? * orientation as f64;
                let id = positions.len();
                positions.push(point);
                normals.push(normal);
                vertex_uv.push(uv[node]);
                vertex_nodes.push(node);
                normal_sides.push(sides);
                display_nodes.insert(key, id);
                id
            };
        }
        if orientation < 0 {
            ids.swap(1, 2);
        }
        let cross =
            (positions[ids[1]] - positions[ids[0]]).cross(positions[ids[2]] - positions[ids[0]]);
        if !cross.finite() || cross.norm() == 0. {
            return Err(Error::Tessellation(
                "polygon display has degenerate triangle",
            ));
        }
        emitted.push(ids);
    }
    let face_ids = vec![0; emitted.len()];
    Ok(NurbsPolygonMesh {
        mesh: Mesh {
            positions,
            normals,
            triangles: emitted,
            face_ids,
        },
        vertex_uv,
        vertex_nodes,
        normal_sides,
        error_bounds: bounds,
    })
}
