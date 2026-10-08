//! Tolerance-aware planar loop predicates. No tessellation is used for validity.
use crate::{Error, Result, Tolerance};
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
pub(crate) fn segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let d = sub(b, a);
    let n = dot(d, d);
    let t = if n == 0.0 {
        0.0
    } else {
        (dot(sub(p, a), d) / n).clamp(0.0, 1.0)
    };
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}
fn segments_distance(a: P2, b: P2, c: P2, d: P2) -> f64 {
    let ab = sub(b, a);
    let cd = sub(d, c);
    let s1 = cross(ab, sub(c, a));
    let s2 = cross(ab, sub(d, a));
    let s3 = cross(cd, sub(a, c));
    let s4 = cross(cd, sub(b, c));
    if s1.signum() != s2.signum() && s3.signum() != s4.signum() {
        return 0.0;
    }
    segment_distance(a, c, d)
        .min(segment_distance(b, c, d))
        .min(segment_distance(c, a, b))
        .min(segment_distance(d, a, b))
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
        if distance(a, b) <= 10.0 * tol.linear {
            return Err(Error::InvalidInput(
                "polygon edge must exceed ten linear tolerances",
            ));
        }
        let d = sub(b, a);
        let e = sub(c, b);
        if cross(d, e).abs() <= tol.linear * length(d).max(length(e)) {
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
            if segments_distance(p[i], p[(i + 1) % p.len()], p[j], p[(j + 1) % p.len()])
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
pub(crate) fn inside_polygon(p: P2, poly: &[P2]) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
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
    fn boundary_distance(&self, p: P2) -> f64 {
        match self {
            Self::Polygon(poly) => (0..poly.len())
                .map(|i| segment_distance(p, poly[i], poly[(i + 1) % poly.len()]))
                .fold(f64::INFINITY, f64::min),
            Self::Circle { center, radius } => (distance(p, *center) - radius).abs(),
        }
    }
    fn contains(&self, p: P2) -> bool {
        match self {
            Self::Polygon(poly) => inside_polygon(p, poly),
            Self::Circle { center, radius } => distance(p, *center) < *radius,
        }
    }
}
fn boundaries_distance(a: &PlanarLoop, b: &PlanarLoop) -> f64 {
    match (a, b) {
        (PlanarLoop::Polygon(p), PlanarLoop::Polygon(q)) => (0..p.len())
            .flat_map(|i| {
                (0..q.len()).map(move |j| {
                    segments_distance(p[i], p[(i + 1) % p.len()], q[j], q[(j + 1) % q.len()])
                })
            })
            .fold(f64::INFINITY, f64::min),
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
            let d = distance(*a, *b);
            if d > *r + *s {
                d - r - s
            } else if d < (*r - *s).abs() {
                (*r - *s).abs() - d
            } else {
                0.0
            }
        }
        (PlanarLoop::Polygon(p), PlanarLoop::Circle { center, radius })
        | (PlanarLoop::Circle { center, radius }, PlanarLoop::Polygon(p)) => (0..p.len())
            .map(|i| {
                let min = segment_distance(*center, p[i], p[(i + 1) % p.len()]);
                let max = distance(*center, p[i]).max(distance(*center, p[(i + 1) % p.len()]));
                if *radius < min {
                    min - radius
                } else if *radius > max {
                    radius - max
                } else {
                    0.0
                }
            })
            .fold(f64::INFINITY, f64::min),
    }
}
pub(crate) fn validate_region(
    outer: &PlanarLoop,
    holes: &[PlanarLoop],
    tol: Tolerance,
) -> Result<()> {
    for h in holes {
        if !outer.contains(h.witness())
            || boundaries_distance(outer, h) <= tol.linear
            || outer.boundary_distance(h.witness()) <= tol.linear
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
            if boundaries_distance(a, b) <= tol.linear
                || a.contains(b.witness())
                || b.contains(a.witness())
            {
                return Err(Error::InvalidInput(
                    "holes overlap, nest, touch or nearly touch",
                ));
            }
        }
    }
    Ok(())
}
