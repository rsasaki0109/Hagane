//! Euclidean classification of retained polygon graph faces, with material-only
//! cap witnesses. Positive Bernstein hull boxes provide distance lower bounds;
//! evaluated points on actual retained faces provide upper bounds. Parameter
//! triangles clip cap search regions; they never approximate 3D geometry.
use crate::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
#[derive(Clone)]
struct Cell {
    surface: NurbsSurface,
    face: usize,
    region: Option<Vec<[f64; 2]>>,
    depth: usize,
    lower: f64,
    serial: usize,
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.serial == b.serial
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Cell {
    fn cmp(&self, b: &Self) -> Ordering {
        b.lower
            .total_cmp(&self.lower)
            .then_with(|| b.serial.cmp(&self.serial))
    }
}
fn unsupported() -> Error {
    Error::Unsupported(
        "polygon graph Euclidean boundary distance or arithmetic precision is unresolved",
    )
}
fn contains(polygon: &[[f64; 2]], point: [f64; 2], strict: bool) -> Result<bool> {
    for i in 0..polygon.len() {
        let o = orient2d(polygon[i], polygon[(i + 1) % polygon.len()], point)?;
        if o == Orientation::Clockwise || (strict && o == Orientation::Collinear) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn material(outer: &[[f64; 2]], hole: Option<&[[f64; 2]]>, point: [f64; 2]) -> Result<bool> {
    Ok(contains(outer, point, false)?
        && !match hole {
            Some(h) => contains(h, point, true)?,
            None => false,
        })
}
fn clip(region: &[[f64; 2]], ranges: [[f64; 2]; 2]) -> Result<Vec<[f64; 2]>> {
    let mut result = region.to_vec();
    for axis in 0..2 {
        for (side, bound) in ranges[axis].iter().copied().enumerate() {
            let inside = |p: [f64; 2]| {
                if side == 0 {
                    p[axis] >= bound
                } else {
                    p[axis] <= bound
                }
            };
            let mut next = Vec::new();
            if result.is_empty() {
                return Ok(result);
            }
            for i in 0..result.len() {
                let a = result[i];
                let b = result[(i + 1) % result.len()];
                if inside(a) {
                    next.push(a);
                }
                if inside(a) != inside(b) {
                    let fraction = (bound - a[axis]) / (b[axis] - a[axis]);
                    if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                        return Err(unsupported());
                    }
                    let other = 1 - axis;
                    let mut point = a;
                    point[axis] = bound;
                    point[other] = (a[other] + fraction * (b[other] - a[other]))
                        .clamp(a[other].min(b[other]), a[other].max(b[other]));
                    if next.last() != Some(&point) {
                        next.push(point);
                    }
                }
            }
            if next.len() > 1 && next.first() == next.last() {
                next.pop();
            }
            result = next;
        }
    }
    if result.len() < 3 {
        return Ok(vec![]);
    }
    let mut area = false;
    for i in 1..result.len() - 1 {
        if orient2d(result[0], result[i], result[i + 1])? != Orientation::Collinear {
            area = true;
        }
    }
    if !area {
        return Ok(vec![]);
    }
    Ok(result)
}
fn project(
    region: Option<&[[f64; 2]]>,
    domain: [[f64; 2]; 2],
    point: [f64; 2],
) -> Result<[f64; 2]> {
    let point = std::array::from_fn::<_, 2, _>(|a| point[a].clamp(domain[a][0], domain[a][1]));
    let Some(region) = region else {
        return Ok(point);
    };
    if contains(region, point, false)? {
        return Ok(point);
    }
    let mut best = region[0];
    let mut distance = f64::INFINITY;
    for i in 0..region.len() {
        let a = region[i];
        let b = region[(i + 1) % region.len()];
        let delta = [b[0] - a[0], b[1] - a[1]];
        let length = delta[0].hypot(delta[1]);
        if length == 0. {
            continue;
        }
        let direction = [delta[0] / length, delta[1] / length];
        let t = (((point[0] - a[0]) * direction[0] + (point[1] - a[1]) * direction[1]) / length)
            .clamp(0., 1.);
        let p = std::array::from_fn::<_, 2, _>(|j| {
            (a[j] + t * delta[j]).clamp(a[j].min(b[j]), a[j].max(b[j]))
        });
        let d = (p[0] - point[0]).hypot(p[1] - point[1]);
        if d < distance {
            distance = d;
            best = p;
        }
    }
    Ok(best)
}
fn lower(surface: &NurbsSurface, query: Point3) -> Result<f64> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in surface.control_points() {
        for (a, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[a] = lo[a].min(v);
            hi[a] = hi[a].max(v);
        }
    }
    let q = [query.x, query.y, query.z];
    let d = std::array::from_fn::<_, 3, _>(|a| {
        if q[a] < lo[a] {
            lo[a] - q[a]
        } else if q[a] > hi[a] {
            q[a] - hi[a]
        } else {
            0.
        }
    });
    let result = d[0].hypot(d[1]).hypot(d[2]);
    if !result.is_finite() {
        return Err(unsupported());
    }
    Ok(result)
}
fn witnesses(
    original: &NurbsSurface,
    cell: &Cell,
    initial: [f64; 2],
    query: Point3,
    outer: &[[f64; 2]],
    hole: Option<&[[f64; 2]]>,
) -> Result<f64> {
    let domain = cell.surface.domain();
    let region = cell.region.as_deref();
    let mut uv = project(region, domain, initial)?;
    let mut upper = f64::INFINITY;
    let candidate = |uv: [f64; 2]| -> Result<Option<f64>> {
        if cell.face < 2 && !material(outer, hole, uv)? {
            return Ok(None);
        }
        let distance = (original.evaluate(uv[0], uv[1])? - query).norm();
        if !distance.is_finite() {
            return Err(unsupported());
        }
        Ok(Some(distance))
    };
    for _ in 0..12 {
        if let Some(distance) = candidate(uv)? {
            upper = upper.min(distance);
        }
        let jet = original.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
        let residual = query - jet.point;
        let scale = jet.du.norm().max(jet.dv.norm());
        if !scale.is_finite() || scale == 0. {
            break;
        }
        let a = Vec3::new(jet.du.x / scale, jet.du.y / scale, jet.du.z / scale);
        let b = Vec3::new(jet.dv.x / scale, jet.dv.y / scale, jet.dv.z / scale);
        let aa = a.dot(a);
        let bb = b.dot(b);
        let ab = a.dot(b);
        let det = aa * bb - ab * ab;
        if !det.is_finite() || det <= 128. * f64::EPSILON * aa * bb {
            break;
        }
        let ra = residual.dot(a) / scale;
        let rb = residual.dot(b) / scale;
        let next = [
            uv[0] + (ra * bb - rb * ab) / det,
            uv[1] + (rb * aa - ra * ab) / det,
        ];
        if next.iter().any(|v| !v.is_finite()) {
            break;
        }
        let next = project(region, domain, next)?;
        if next == uv {
            break;
        }
        uv = next;
    }
    let corners = if let Some(region) = region {
        region.to_vec()
    } else {
        vec![
            [domain[0][0], domain[1][0]],
            [domain[0][1], domain[1][0]],
            [domain[0][1], domain[1][1]],
            [domain[0][0], domain[1][1]],
        ]
    };
    let middle = project(region, domain, domain.map(|[a, b]| a + (b - a) / 2.))?;
    for uv in corners.into_iter().chain([middle]) {
        if let Some(distance) = candidate(uv)? {
            upper = upper.min(distance);
        }
    }
    Ok(upper)
}
fn membership(
    source: &NurbsGraphSolid,
    outer: &[[f64; 2]],
    hole: Option<&[[f64; 2]]>,
    local: Point3,
    guard: f64,
) -> Result<PointLocation> {
    let [l, w, h] = source.dimensions();
    let margins = |polygon: &[[f64; 2]]| -> Result<Vec<f64>> {
        let mut out = vec![];
        for i in 0..polygon.len() {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            let dx = l * (b[0] - a[0]);
            let dy = w * (b[1] - a[1]);
            let length = dx.hypot(dy);
            let gap = (dx / length) * (local.y - w * a[1]) - (dy / length) * (local.x - l * a[0]);
            if !gap.is_finite() {
                return Err(unsupported());
            }
            out.push(gap);
        }
        Ok(out)
    };
    let outer = margins(outer)?;
    if outer.iter().any(|d| *d < -guard) || local.z < -guard {
        return Ok(PointLocation::Outside);
    }
    let mut hole_outside = true;
    if let Some(hole) = hole {
        let margins = margins(hole)?;
        if margins.iter().all(|d| *d > guard) {
            return Ok(PointLocation::Outside);
        }
        hole_outside = margins.iter().any(|d| *d < -guard);
    }
    let u = local.x / l;
    let v = local.y / w;
    let roof = h + source.bulge() * (4. * u * (1. - u) * v * (1. - v));
    let gap = roof - local.z;
    if !gap.is_finite() {
        return Err(unsupported());
    }
    if gap < -guard {
        return Ok(PointLocation::Outside);
    }
    if outer.iter().all(|d| *d > guard) && local.z > guard && gap > guard && hole_outside {
        Ok(PointLocation::Inside)
    } else {
        Err(unsupported())
    }
}
fn classify(
    source: &NurbsGraphSolid,
    solid: &Solid,
    outer: &[[f64; 2]],
    hole: Option<&[[f64; 2]]>,
    triangles: Vec<[[f64; 2]; 3]>,
    query: Point3,
    tol: GeometryTolerance,
) -> Result<PointLocation> {
    if !query.finite() {
        return Err(Error::InvalidInput(
            "polygon classification requires a finite world point",
        ));
    }
    let local = source.placement().local_point(query);
    if !local.finite() {
        return Err(unsupported());
    }
    let identity = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol.absolute())?
        .trimmed_uv(source.source_domain(), tol.absolute())?;
    let bounds = identity.bounds()?;
    let budget = tol.length_at_scale((bounds.max - bounds.min).norm())?;
    let arithmetic = source.arithmetic_budget()?
        + 256. * f64::EPSILON * query.x.abs().max(query.y.abs()).max(query.z.abs());
    if !arithmetic.is_finite() || arithmetic >= budget / 4. {
        return Err(unsupported());
    }
    let originals = solid
        .shell
        .faces
        .iter()
        .map(|f| match &f.surface {
            Surface::Nurbs(s) => Ok(&**s),
            _ => Err(Error::InvalidTopology(
                "polygon classification requires retained NURBS faces",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut queue = BinaryHeap::new();
    let mut serial = 0;
    for (face, original) in originals.iter().enumerate() {
        if face < 2 {
            for tri in &triangles {
                let ranges = std::array::from_fn::<_, 2, _>(|a| {
                    [
                        tri.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min),
                        tri.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max),
                    ]
                });
                let surface = original.restricted(ranges)?;
                let lower = lower(&surface, query)?;
                queue.push(Cell {
                    surface,
                    face,
                    region: Some(tri.to_vec()),
                    depth: 0,
                    lower,
                    serial,
                });
                serial += 1;
            }
        } else {
            for patch in original.bezier_patches()? {
                let lower = lower(&patch.surface, query)?;
                queue.push(Cell {
                    surface: patch.surface,
                    face,
                    region: None,
                    depth: 0,
                    lower,
                    serial,
                });
                serial += 1;
            }
        }
    }
    let [l, w, _] = source.dimensions();
    let mut visits = 0;
    while let Some(cell) = queue.pop() {
        // This accumulated world-space engineering allowance covers homogeneous
        // restriction and rounded UV clipping/projection; clipped vertices are
        // search data, never new B-rep boundary geometry or an interval proof.
        let guard = arithmetic * (cell.depth + 2) as f64;
        if !guard.is_finite() || guard >= budget / 4. {
            return Err(unsupported());
        }
        if cell.lower > budget + guard {
            continue;
        }
        visits += 1;
        if visits > 16384 || cell.depth >= 48 {
            return Err(unsupported());
        }
        let domain = cell.surface.domain();
        let initial = if cell.face < 2 {
            [local.x / l, local.y / w]
        } else {
            let polygon = if cell.face < 2 + outer.len() {
                outer
            } else {
                hole.ok_or(Error::InvalidTopology("missing opening walls"))?
            };
            let i = if cell.face < 2 + outer.len() {
                cell.face - 2
            } else {
                cell.face - 2 - outer.len()
            };
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            let dx = l * (b[0] - a[0]);
            let dy = w * (b[1] - a[1]);
            let length = dx.hypot(dy);
            let t = (((local.x - l * a[0]) * (dx / length) + (local.y - w * a[1]) * (dy / length))
                / length)
                .clamp(0., 1.);
            let u = a[0] + t * (b[0] - a[0]);
            let v = a[1] + t * (b[1] - a[1]);
            let height =
                source.dimensions()[2] + source.bulge() * (4. * u * (1. - u) * v * (1. - v));
            [t, local.z / height]
        };
        if witnesses(originals[cell.face], &cell, initial, query, outer, hole)? + guard <= budget {
            return Ok(PointLocation::Boundary);
        }
        let middle = domain.map(|[a, b]| a + (b - a) / 2.);
        if (0..2).any(|a| middle[a] <= domain[a][0] || middle[a] >= domain[a][1])
            || serial + 4 > 65536
        {
            return Err(unsupported());
        }
        for iu in 0..2 {
            for iv in 0..2 {
                let ranges = [
                    [
                        if iu == 0 { domain[0][0] } else { middle[0] },
                        if iu == 0 { middle[0] } else { domain[0][1] },
                    ],
                    [
                        if iv == 0 { domain[1][0] } else { middle[1] },
                        if iv == 0 { middle[1] } else { domain[1][1] },
                    ],
                ];
                let region = if let Some(region) = &cell.region {
                    let region = clip(region, ranges)?;
                    if region.is_empty() {
                        continue;
                    }
                    Some(region)
                } else {
                    None
                };
                let surface = cell.surface.restricted(ranges)?;
                let lower = lower(&surface, query)?;
                queue.push(Cell {
                    surface,
                    face: cell.face,
                    region,
                    depth: cell.depth + 1,
                    lower,
                    serial,
                });
                serial += 1;
            }
        }
    }
    membership(source, outer, hole, local, arithmetic)
}
impl NurbsGraphPolygonSolid {
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        self.validate(tol.absolute())?;
        let p = self.polygon();
        let triangles = (1..p.len() - 1).map(|i| [p[0], p[i], p[i + 1]]).collect();
        classify(self.source(), self.brep(), p, None, triangles, point, tol)
    }
}
impl NurbsGraphPolygonHoledSolid {
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        self.validate(tol.absolute())?;
        let points: Vec<_> = self
            .outer_polygon()
            .iter()
            .chain(self.opening())
            .copied()
            .collect();
        let triangles = self
            .material_triangles()?
            .into_iter()
            .map(|tri| tri.map(|i| points[i]))
            .collect();
        classify(
            self.source(),
            self.brep(),
            self.outer_polygon(),
            Some(self.opening()),
            triangles,
            point,
            tol,
        )
    }
}
