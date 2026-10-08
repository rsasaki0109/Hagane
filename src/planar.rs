//! Tolerance-aware planar loop predicates. No tessellation is used for validity.
use crate::{
    locate_point_in_polygon, orient2d, segments_intersect2d, Error, Orientation, PointLocation,
    Result, Tolerance,
};
pub(crate) type P2 = [f64; 2];
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn length(a: P2) -> f64 {
    a[0].hypot(a[1])
}
pub(crate) fn distance(a: P2, b: P2) -> f64 {
    length(sub(a, b))
}
pub(crate) fn segment_distance(p: P2, a: P2, b: P2) -> Result<f64> {
    let d = sub(b, a);
    let n = length(d);
    let delta = sub(p, a);
    if !n.is_finite() || delta.iter().any(|x| !x.is_finite()) {
        return Err(Error::InvalidInput(
            "planar separation exceeds finite range",
        ));
    }
    if n == 0.0 {
        return checked_distance(p, a);
    }
    let unit = [d[0] / n, d[1] / n];
    let projection = dot(delta, unit);
    if !projection.is_finite() {
        return Err(Error::InvalidInput(
            "planar projection exceeds finite range",
        ));
    }
    let along = projection.clamp(0.0, n);
    let result = length([delta[0] - along * unit[0], delta[1] - along * unit[1]]);
    if !result.is_finite() {
        return Err(Error::InvalidInput("planar distance exceeds finite range"));
    }
    Ok(result)
}
fn checked_distance(a: P2, b: P2) -> Result<f64> {
    let result = distance(a, b);
    if !result.is_finite() {
        return Err(Error::InvalidInput("planar distance exceeds finite range"));
    }
    Ok(result)
}
fn segments_distance(a: P2, b: P2, c: P2, d: P2) -> Result<f64> {
    if segments_intersect2d(a, b, c, d)? {
        return Ok(0.0);
    }
    Ok(segment_distance(a, c, d)?
        .min(segment_distance(b, c, d)?)
        .min(segment_distance(c, a, b)?)
        .min(segment_distance(d, a, b)?))
}
pub(crate) fn polygon_area(p: &[P2]) -> f64 {
    // Translate to the first point to avoid cancellation at far origins.
    if p.is_empty() {
        return 0.0;
    }
    (0..p.len())
        .map(|i| cross(sub(p[i], p[0]), sub(p[(i + 1) % p.len()], p[0])) * 0.5)
        .sum()
}
/// Returns a signed simple loop. Rejects redundant/near-collinear corners so
/// cap triangulation and side boundaries cannot acquire different subdivisions.
pub(crate) fn validate_polygon(p: &[P2], tol: Tolerance) -> Result<()> {
    if p.len() < 3 || p.len() > 4096 {
        return Err(Error::InvalidInput(
            "polygon requires 3..4096 distinct corners",
        ));
    }
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Error::InvalidInput("nonfinite polygon corner"));
    }
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        let c = p[(i + 2) % p.len()];
        if checked_distance(a, b)? <= 10.0 * tol.linear {
            return Err(Error::InvalidInput(
                "polygon edge must exceed ten linear tolerances",
            ));
        }
        let d = sub(b, a);
        let e = sub(c, b);
        let (nd, ne) = (length(d), length(e));
        if !nd.is_finite() || !ne.is_finite() {
            return Err(Error::InvalidInput("polygon span exceeds finite range"));
        }
        let sine = cross([d[0] / nd, d[1] / nd], [e[0] / ne, e[1] / ne]).abs();
        if orient2d(a, b, c)? == Orientation::Collinear || sine * nd.min(ne) <= tol.linear {
            return Err(Error::InvalidInput(
                "redundant or near-collinear polygon corner",
            ));
        }
    }
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            if j == i + 1 || (i == 0 && j == p.len() - 1) {
                continue;
            }
            if boxes_separated(
                p[i],
                p[(i + 1) % p.len()],
                p[j],
                p[(j + 1) % p.len()],
                tol.linear,
            ) {
                continue;
            }
            if segments_distance(p[i], p[(i + 1) % p.len()], p[j], p[(j + 1) % p.len()])?
                <= tol.linear
            {
                return Err(Error::InvalidInput(
                    "polygon self-intersects or nearly touches",
                ));
            }
        }
    }
    let area = polygon_area(p);
    if !area.is_finite() || area.abs() <= tol.linear * tol.linear {
        return Err(Error::InvalidInput("degenerate polygon area"));
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub(crate) enum PlanarLoop {
    Polygon(Vec<P2>),
    Circle { center: P2, radius: f64 },
}
impl PlanarLoop {
    fn witness(&self) -> P2 {
        match self {
            Self::Polygon(p) => p[0],
            Self::Circle { center, radius } => [center[0] + radius, center[1]],
        }
    }
    fn boundary_distance(&self, p: P2) -> Result<f64> {
        match self {
            Self::Polygon(poly) => {
                let mut minimum = f64::INFINITY;
                for i in 0..poly.len() {
                    minimum =
                        minimum.min(segment_distance(p, poly[i], poly[(i + 1) % poly.len()])?);
                }
                Ok(minimum)
            }
            Self::Circle { center, radius } => Ok((checked_distance(p, *center)? - radius).abs()),
        }
    }
    fn contains(&self, p: P2) -> Result<bool> {
        match self {
            Self::Polygon(poly) => Ok(locate_point_in_polygon(p, poly)? == PointLocation::Inside),
            Self::Circle { center, radius } => Ok(checked_distance(p, *center)? < *radius),
        }
    }
}
fn boundaries_distance(a: &PlanarLoop, b: &PlanarLoop) -> Result<f64> {
    match (a, b) {
        (PlanarLoop::Polygon(p), PlanarLoop::Polygon(q)) => {
            let mut minimum = f64::INFINITY;
            for i in 0..p.len() {
                for j in 0..q.len() {
                    minimum = minimum.min(segments_distance(
                        p[i],
                        p[(i + 1) % p.len()],
                        q[j],
                        q[(j + 1) % q.len()],
                    )?);
                }
            }
            Ok(minimum)
        }
        (
            PlanarLoop::Circle {
                center: a,
                radius: r,
            },
            PlanarLoop::Circle {
                center: b,
                radius: s,
            },
        ) => {
            let d = checked_distance(*a, *b)?;
            if !(r + s).is_finite() {
                return Err(Error::InvalidInput(
                    "combined circle radii exceed finite range",
                ));
            }
            Ok(if d > r + s {
                d - r - s
            } else if d < (r - s).abs() {
                (r - s).abs() - d
            } else {
                0.0
            })
        }
        (PlanarLoop::Polygon(p), PlanarLoop::Circle { center, radius })
        | (PlanarLoop::Circle { center, radius }, PlanarLoop::Polygon(p)) => {
            let mut minimum = f64::INFINITY;
            for i in 0..p.len() {
                let a = p[i];
                let b = p[(i + 1) % p.len()];
                let near = segment_distance(*center, a, b)?;
                let far = checked_distance(*center, a)?.max(checked_distance(*center, b)?);
                minimum = minimum.min(if *radius < near {
                    near - radius
                } else if *radius > far {
                    radius - far
                } else {
                    0.0
                });
            }
            Ok(minimum)
        }
    }
}
pub(crate) fn validate_region(
    outer: &PlanarLoop,
    holes: &[PlanarLoop],
    tol: Tolerance,
) -> Result<()> {
    for h in holes {
        if !outer.contains(h.witness())?
            || boundaries_distance(outer, h)? <= tol.linear
            || outer.boundary_distance(h.witness())? <= tol.linear
        {
            return Err(Error::InvalidInput(
                "hole touches or leaves its outer boundary",
            ));
        }
    }
    for i in 0..holes.len() {
        for j in i + 1..holes.len() {
            let a = &holes[i];
            let b = &holes[j];
            if boundaries_distance(a, b)? <= tol.linear
                || a.contains(b.witness())?
                || b.contains(a.witness())?
            {
                return Err(Error::InvalidInput(
                    "holes overlap, nest, touch or nearly touch",
                ));
            }
        }
    }
    Ok(())
}

fn boxes_separated(a: P2, b: P2, c: P2, d: P2, tolerance: f64) -> bool {
    (0..2).any(|i| {
        a[i].max(b[i]) + tolerance < c[i].min(d[i]) || c[i].max(d[i]) + tolerance < a[i].min(b[i])
    })
}
/// Validate the sampled display boundary independently of analytic modeling.
/// A coarse approximation must not move a hole outside or create a crossing.
pub(crate) fn validate_sampled_region(loops: &[Vec<P2>], tol: Tolerance) -> Result<()> {
    for ring in loops {
        validate_polygon(ring, tol)?;
    }
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            for k in 0..loops[i].len() {
                for l in 0..loops[j].len() {
                    let (a, b, c, d) = (
                        loops[i][k],
                        loops[i][(k + 1) % loops[i].len()],
                        loops[j][l],
                        loops[j][(l + 1) % loops[j].len()],
                    );
                    if !boxes_separated(a, b, c, d, tol.linear)
                        && segments_distance(a, b, c, d)? <= tol.linear
                    {
                        return Err(Error::Tessellation(
                            "sampled trim boundaries intersect or nearly touch",
                        ));
                    }
                }
            }
            let a = locate_point_in_polygon(loops[j][0], &loops[i])?;
            let b = locate_point_in_polygon(loops[i][0], &loops[j])?;
            if (i == 0 && a != PointLocation::Inside)
                || (i > 0 && (a != PointLocation::Outside || b != PointLocation::Outside))
            {
                return Err(Error::Tessellation(
                    "sampled holes leave their boundary or nest",
                ));
            }
        }
    }
    Ok(())
}
