//! Euclidean boundary-band classification for canonical graph-solid wrappers.
//!
//! Positive rational Bernstein coefficients sum to one after normalization:
//! every exact patch lies in its control-point convex hull and therefore its
//! control AABB. Distance to that box is a lower bound. Evaluated points on the
//! retained original face are upper-bound witnesses, including tangent-plane
//! iteration candidates whose convergence is never presumed. Material-only
//! patches cover holed caps; excluded patches cannot create a false boundary.
//! Repeated restriction and world arithmetic use separate engineering guards,
//! not formal interval certificates. Unresolved bands/resources return errors.
//! This is an independent application of the Bernstein convex-hull property;
//! no point fitting, mesh classification, or external CAD implementation is used.
use crate::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
#[derive(Clone)]
struct Cell {
    surface: NurbsSurface,
    face: usize,
    depth: usize,
    lower: f64,
    serial: usize,
}
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cell {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .total_cmp(&self.lower)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}
fn box_distance(surface: &NurbsSurface, query: Point3) -> Result<f64> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in surface.control_points() {
        for (i, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            min[i] = min[i].min(v);
            max[i] = max[i].max(v);
        }
    }
    let delta: [f64; 3] = std::array::from_fn(|i| {
        let q = [query.x, query.y, query.z][i];
        if q < min[i] {
            min[i] - q
        } else if q > max[i] {
            q - max[i]
        } else {
            0.
        }
    });
    let result = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !result.is_finite() {
        return Err(Error::InvalidInput(
            "graph classification distance exceeds finite range",
        ));
    }
    Ok(result)
}
fn roof(source: &NurbsGraphSolid, u: f64, v: f64) -> f64 {
    source.dimensions()[2] + 4. * source.bulge() * u * (1. - u) * v * (1. - v)
}
fn hint(
    source: &NurbsGraphSolid,
    hole: Option<[[f64; 2]; 2]>,
    face: usize,
    local: Point3,
) -> [f64; 2] {
    let [l, w, _] = source.dimensions();
    let u = local.x / l;
    let v = local.y / w;
    if face < 2 {
        return [u, v];
    }
    let outer = source.source_domain();
    let (varying, fixed, axis) = if let Some(hole) = hole {
        match face {
            2 => (u, outer[1][0], 0),
            3 => (v, outer[0][1], 1),
            4 => (u, outer[1][1], 0),
            5 => (v, outer[0][0], 1),
            6 => (v, hole[0][0], 1),
            7 => (u, hole[1][1], 0),
            8 => (v, hole[0][1], 1),
            _ => (u, hole[1][0], 0),
        }
    } else {
        match face {
            2 => (v, outer[0][0], 1),
            3 => (v, outer[0][1], 1),
            4 => (u, outer[1][0], 0),
            _ => (u, outer[1][1], 0),
        }
    };
    let height = if axis == 0 {
        roof(source, varying.clamp(outer[0][0], outer[0][1]), fixed)
    } else {
        roof(source, fixed, varying.clamp(outer[1][0], outer[1][1]))
    };
    [varying, local.z / height]
}
fn witnesses(
    original: &NurbsSurface,
    domain: [[f64; 2]; 2],
    initial: [f64; 2],
    query: Point3,
) -> Result<f64> {
    let clamp = |uv: [f64; 2]| -> [f64; 2] {
        std::array::from_fn(|i| uv[i].clamp(domain[i][0], domain[i][1]))
    };
    let mut uv = clamp(initial);
    let mut upper = f64::INFINITY;
    // Newton-like tangent-plane candidates are only witnesses. Their convergence
    // is never used as a distance lower bound or as proof of a closest point.
    for _ in 0..12 {
        let jet = original.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
        let residual = query - jet.point;
        let distance = residual.norm();
        if !distance.is_finite() {
            return Err(Error::InvalidInput(
                "graph classification witness distance overflows",
            ));
        }
        upper = upper.min(distance);
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
        let next = clamp([
            uv[0] + (ra * bb - rb * ab) / det,
            uv[1] + (rb * aa - ra * ab) / det,
        ]);
        if next.iter().any(|x| !x.is_finite()) || next == uv {
            break;
        }
        uv = next;
    }
    let middle = domain.map(|[a, b]| a + (b - a) / 2.);
    for uv in [
        middle,
        [domain[0][0], domain[1][0]],
        [domain[0][0], domain[1][1]],
        [domain[0][1], domain[1][0]],
        [domain[0][1], domain[1][1]],
    ] {
        let distance = (original.evaluate(uv[0], uv[1])? - query).norm();
        if !distance.is_finite() {
            return Err(Error::InvalidInput(
                "graph classification witness distance overflows",
            ));
        }
        upper = upper.min(distance);
    }
    Ok(upper)
}
fn classify(
    source: &NurbsGraphSolid,
    solid: &Solid,
    hole: Option<[[f64; 2]; 2]>,
    query: Point3,
    tol: GeometryTolerance,
) -> Result<PointLocation> {
    if !query.finite() {
        return Err(Error::InvalidInput(
            "graph classification requires a finite world point",
        ));
    }
    let local = source.placement().local_point(query);
    if !local.finite() {
        return Err(Error::InvalidInput(
            "graph classification inverse placement overflows",
        ));
    }
    // The local canonical source enclosure controls relative tolerance. World
    // placement and query distance influence arithmetic guards, never this band.
    let local_source = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol.absolute())?
        .trimmed_uv(source.source_domain(), tol.absolute())?;
    let bounds = local_source.bounds()?;
    let diagonal = (bounds.max - bounds.min).norm();
    let budget = tol.length_at_scale(diagonal)?;
    let query_scale = query.x.abs().max(query.y.abs()).max(query.z.abs());
    let arithmetic = source.arithmetic_budget()? + 256. * f64::EPSILON * query_scale;
    if !arithmetic.is_finite() || arithmetic >= budget / 4. {
        return Err(Error::Unsupported(
            "graph classification coordinate precision cannot resolve its Euclidean tolerance band",
        ));
    }
    let originals = solid
        .shell
        .faces
        .iter()
        .map(|f| {
            if let Surface::Nurbs(s) = &f.surface {
                Ok(&**s)
            } else {
                Err(Error::InvalidTopology(
                    "graph classification requires retained rational faces",
                ))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let mut queue = BinaryHeap::new();
    let mut serial = 0usize;
    for (face, original) in originals.iter().enumerate() {
        for patch in original.bezier_patches()? {
            let domain = patch.parameter_ranges;
            if face < 2
                && hole.is_some_and(|h| {
                    domain[0][0] >= h[0][0]
                        && domain[0][1] <= h[0][1]
                        && domain[1][0] >= h[1][0]
                        && domain[1][1] <= h[1][1]
                })
            {
                continue;
            }
            let lower = box_distance(&patch.surface, query)?;
            queue.push(Cell {
                surface: patch.surface,
                face,
                depth: 0,
                lower,
                serial,
            });
            serial += 1;
        }
    }
    let mut visits = 0usize;
    while let Some(cell) = queue.pop() {
        let guard = arithmetic * (cell.depth + 2) as f64;
        if !guard.is_finite() || guard >= budget / 4. {
            return Err(Error::Unsupported(
                "graph classification subdivision precision cannot resolve boundary distance",
            ));
        }
        if cell.lower > budget + guard {
            continue;
        }
        visits += 1;
        if visits > 16384 || cell.depth >= 48 {
            return Err(Error::Unsupported(
                "graph classification boundary distance exceeds subdivision resources",
            ));
        }
        let domain = cell.surface.domain();
        let upper = witnesses(
            originals[cell.face],
            domain,
            hint(source, hole, cell.face, local),
            query,
        )?;
        if upper + guard <= budget {
            return Ok(PointLocation::Boundary);
        }
        let middle = domain.map(|[a, b]| a + (b - a) / 2.);
        if (0..2).any(|axis| middle[axis] <= domain[axis][0] || middle[axis] >= domain[axis][1]) {
            return Err(Error::Unsupported(
                "graph classification UV subdivision is unrepresentable",
            ));
        }
        if serial + 4 > 65536 {
            return Err(Error::Unsupported(
                "graph classification patch work budget exceeded",
            ));
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
                let surface = cell.surface.restricted(ranges)?;
                let lower = box_distance(&surface, query)?;
                queue.push(Cell {
                    surface,
                    face: cell.face,
                    depth: cell.depth + 1,
                    lower,
                    serial,
                });
                serial += 1;
            }
        }
    }
    let [l, w, _] = source.dimensions();
    let [u, v] = source.source_domain();
    let xy = [
        local.x - l * u[0],
        l * u[1] - local.x,
        local.y - w * v[0],
        w * v[1] - local.y,
    ];
    if xy.iter().any(|x| !x.is_finite()) {
        return Err(Error::Unsupported(
            "graph classification analytic membership overflows",
        ));
    }
    if xy.iter().any(|x| *x < -arithmetic) || local.z < -arithmetic {
        return Ok(PointLocation::Outside);
    }
    let mut hole_outside = true;
    if let Some(h) = hole {
        let gaps = [
            local.x - l * h[0][0],
            l * h[0][1] - local.x,
            local.y - w * h[1][0],
            w * h[1][1] - local.y,
        ];
        if gaps.iter().any(|x| !x.is_finite()) {
            return Err(Error::Unsupported(
                "graph opening analytic membership overflows",
            ));
        }
        if gaps.iter().all(|x| *x > arithmetic) {
            return Ok(PointLocation::Outside);
        }
        hole_outside = gaps.iter().any(|x| *x < -arithmetic);
    }
    let height = roof(source, local.x / l, local.y / w);
    let roof_margin = height - local.z;
    if !roof_margin.is_finite() {
        return Err(Error::Unsupported(
            "graph classification analytic height overflows",
        ));
    }
    if roof_margin < -arithmetic {
        return Ok(PointLocation::Outside);
    }
    if xy.iter().all(|x| *x > arithmetic)
        && local.z > arithmetic
        && roof_margin > arithmetic
        && hole_outside
    {
        return Ok(PointLocation::Inside);
    }
    Err(Error::Unsupported(
        "graph classification analytic membership is unresolved",
    ))
}
impl NurbsGraphSolid {
    /// Classify against a Euclidean boundary band after canonical B-rep validation.
    /// Relative tolerance uses the local source enclosure; unresolved distance
    /// or arithmetic conditioning returns an explicit error.
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        self.validate(tol.absolute())?;
        classify(self, self.brep(), None, point, tol)
    }
}
impl NurbsGraphHoledSolid {
    /// Classify against a Euclidean boundary band after canonical B-rep validation.
    /// Relative tolerance uses the local source enclosure; unresolved distance
    /// or arithmetic conditioning returns an explicit error.
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        self.validate(tol.absolute())?;
        classify(self.source(), self.brep(), Some(self.hole()), point, tol)
    }
}
