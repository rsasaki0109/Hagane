//! Euclidean retained-face classification, excluding every circular-void cap witness.
use crate::nurbs_graph_classification::{box_distance, witnesses_filtered};
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
fn unresolved() -> Error {
    Error::Unsupported("circular graph Euclidean distance or arithmetic precision is unresolved")
}
fn radial(body: &NurbsGraphCircularHoledSolid, uv: [f64; 2]) -> Result<f64> {
    let [l, w, _] = body.source().dimensions();
    let c = body.center();
    let d = (l * uv[0] - c[0]).hypot(w * uv[1] - c[1]);
    if d.is_finite() {
        Ok(d)
    } else {
        Err(unresolved())
    }
}
fn removed(body: &NurbsGraphCircularHoledSolid, domain: [[f64; 2]; 2], guard: f64) -> Result<bool> {
    for u in domain[0] {
        for v in domain[1] {
            if radial(body, [u, v])? + guard >= body.radius() {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn height(source: &NurbsGraphSolid, uv: [f64; 2]) -> f64 {
    source.dimensions()[2] + 4. * source.bulge() * uv[0] * (1. - uv[0]) * uv[1] * (1. - uv[1])
}
fn hint(body: &NurbsGraphCircularHoledSolid, face: usize, local: Point3) -> Result<[f64; 2]> {
    let source = body.source();
    let [l, w, _] = source.dimensions();
    let uv = [local.x / l, local.y / w];
    if face < 2 {
        return Ok(uv);
    }
    let domain = source.source_domain();
    if face < 6 {
        let (varying, fixed, axis) = match face {
            2 => (uv[1], domain[0][0], 1),
            3 => (uv[1], domain[0][1], 1),
            4 => (uv[0], domain[1][0], 0),
            _ => (uv[0], domain[1][1], 0),
        };
        let parameter = if axis == 0 {
            [varying.clamp(domain[0][0], domain[0][1]), fixed]
        } else {
            [fixed, varying.clamp(domain[1][0], domain[1][1])]
        };
        return Ok([varying, local.z / height(source, parameter)]);
    }
    let c = body.center();
    let dx = local.x - c[0];
    let dy = local.y - c[1];
    let q = face - 6;
    let (x, y) = match q {
        0 => (dx, dy),
        1 => (dy, -dx),
        2 => (-dx, -dy),
        _ => (-dy, dx),
    };
    // A seed only: inverse rational-quarter coordinates do not certify distance.
    let t = if x == 0. && y == 0. {
        0.5
    } else if x >= 0. && y >= 0. {
        let scale = x.max(y);
        let (x, y) = (x / scale, y / scale);
        let k = y / (x.hypot(y) + x);
        std::f64::consts::SQRT_2 * k / (1. + (std::f64::consts::SQRT_2 - 1.) * k)
    } else if x >= y {
        0.
    } else {
        1.
    };
    let boundary = body.brep().shell.faces[1].wires[1]
        .coedges
        .iter()
        .find(|c| c.edge == 16 + q)
        .ok_or_else(unresolved)?;
    let uv = boundary.pcurve.try_evaluate(t)?;
    Ok([t, local.z / height(source, uv)])
}
impl NurbsGraphCircularHoledSolid {
    /// Classify using a Euclidean boundary band on the actual retained faces.
    /// Relative tolerance uses the local source enclosure. Removed disk regions
    /// cannot provide cap witnesses. Unresolved precision/resources return errors.
    pub fn classify_point(&self, query: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        self.validate(tol.absolute())?;
        if !query.finite() {
            return Err(Error::InvalidInput(
                "circular classification requires a finite point",
            ));
        }
        let source = self.source();
        let local = source.placement().local_point(query);
        if !local.finite() {
            return Err(unresolved());
        }
        let local_source =
            NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol.absolute())?
                .trimmed_uv(source.source_domain(), tol.absolute())?;
        let bounds = local_source.bounds()?;
        let budget = tol.length_at_scale((bounds.max - bounds.min).norm())?;
        let arithmetic = source.arithmetic_budget()?
            + 256. * f64::EPSILON * query.x.abs().max(query.y.abs()).max(query.z.abs());
        if !arithmetic.is_finite() || arithmetic >= budget / 4. {
            return Err(unresolved());
        }
        let [l, w, _] = source.dimensions();
        let domain = source.source_domain();
        let xy = [
            local.x - l * domain[0][0],
            l * domain[0][1] - local.x,
            local.y - w * domain[1][0],
            w * domain[1][1] - local.y,
        ];
        if xy.iter().any(|x| !x.is_finite()) {
            return Err(unresolved());
        }
        let c = self.center();
        let rho = (local.x - c[0]).hypot(local.y - c[1]);
        if !rho.is_finite() {
            return Err(unresolved());
        }
        // Supporting planes and the entire removed vertical cylinder give true
        // Euclidean separation. This shortcut does not use a vertical roof gap.
        if xy.iter().any(|x| *x < -budget - 4. * arithmetic)
            || local.z < -budget - 4. * arithmetic
            || self.radius() - rho > budget + 4. * arithmetic
        {
            return Ok(PointLocation::Outside);
        }
        let originals = self
            .brep()
            .shell
            .faces
            .iter()
            .map(|f| match &f.surface {
                Surface::Nurbs(s) => Ok(&**s),
                _ => Err(unresolved()),
            })
            .collect::<Result<Vec<_>>>()?;
        let mut queue = BinaryHeap::new();
        let mut serial = 0;
        for (face, original) in originals.iter().enumerate() {
            for patch in original.bezier_patches()? {
                if face < 2 && removed(self, patch.parameter_ranges, 4. * arithmetic)? {
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
        let mut visits = 0;
        while let Some(cell) = queue.pop() {
            let guard = arithmetic * (cell.depth + 2) as f64;
            if !guard.is_finite() || guard >= budget / 4. {
                return Err(unresolved());
            }
            if cell.lower > budget + guard {
                continue;
            }
            visits += 1;
            if visits > 16384 || cell.depth >= 48 {
                return Err(unresolved());
            }
            let domain = cell.surface.domain();
            if cell.face < 2 && removed(self, domain, 4. * guard)? {
                continue;
            }
            let accept = |uv| -> Result<bool> {
                if cell.face < 2 {
                    Ok(radial(self, uv)? >= self.radius() + 4. * guard)
                } else {
                    Ok(true)
                }
            };
            let upper = witnesses_filtered(
                originals[cell.face],
                domain,
                hint(self, cell.face, local)?,
                query,
                &accept,
            )?;
            if upper + guard <= budget {
                return Ok(PointLocation::Boundary);
            }
            let middle = domain.map(|[a, b]| a + (b - a) / 2.);
            if (0..2).any(|a| middle[a] <= domain[a][0] || middle[a] >= domain[a][1]) {
                return Err(unresolved());
            }
            if serial + 4 > 65536 {
                return Err(unresolved());
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
                    if cell.face < 2 && removed(self, ranges, 4. * guard)? {
                        continue;
                    }
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
        if xy.iter().any(|x| *x < -arithmetic)
            || local.z < -arithmetic
            || self.radius() - rho > arithmetic
        {
            return Ok(PointLocation::Outside);
        }
        let roof_margin = height(source, [local.x / l, local.y / w]) - local.z;
        if !roof_margin.is_finite() {
            return Err(unresolved());
        }
        if roof_margin < -arithmetic {
            return Ok(PointLocation::Outside);
        }
        if xy.iter().all(|x| *x > arithmetic)
            && rho - self.radius() > arithmetic
            && local.z > arithmetic
            && roof_margin > arithmetic
        {
            return Ok(PointLocation::Inside);
        }
        Err(unresolved())
    }
}
