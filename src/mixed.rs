//! Simple signed line/arc regions, analytic trim validation and exact vector extrusion.
use crate::*;
use std::f64::consts::{PI, TAU};
type P2 = [f64; 2];
#[derive(Clone, Copy, Debug)]
pub enum PlanarSegment {
    Line {
        a: P2,
        b: P2,
    },
    /// Signed circular arc: positive CCW, negative CW; angles in radians.
    Arc {
        center: P2,
        radius: f64,
        start_angle: f64,
        sweep: f64,
    },
}
impl PlanarSegment {
    pub fn evaluate(&self, fraction: f64) -> P2 {
        match *self {
            Self::Line { a, b } => [
                a[0] + (b[0] - a[0]) * fraction,
                a[1] + (b[1] - a[1]) * fraction,
            ],
            Self::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => {
                let angle = start_angle + sweep * fraction;
                [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin(),
                ]
            }
        }
    }
    fn tangent(&self, end: bool) -> Result<Vec3> {
        match *self {
            Self::Line { a, b } => Vec3::new(b[0] - a[0], b[1] - a[1], 0.0).normalized(),
            Self::Arc {
                start_angle, sweep, ..
            } => {
                let a = start_angle + if end { sweep } else { 0.0 };
                Ok(Vec3::new(-a.sin(), a.cos(), 0.0) * sweep.signum())
            }
        }
    }
    /// Reverses traversal without replacing the circular geometry by segments.
    pub fn reversed(self) -> Self {
        match self {
            Self::Line { a, b } => Self::Line { a: b, b: a },
            Self::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => Self::Arc {
                center,
                radius,
                start_angle: (start_angle + sweep).rem_euclid(TAU),
                sweep: -sweep,
            },
        }
    }
    fn forward(self) -> bool {
        !matches!(self,Self::Arc { sweep,.. } if sweep<0.0)
    }
    fn canonical(self) -> Self {
        if self.forward() {
            self
        } else {
            self.reversed()
        }
    }
    fn pcurve(&self) -> PCurve {
        match self.canonical() {
            Self::Line { a, b } => PCurve::Affine {
                origin: a,
                direction: [b[0] - a[0], b[1] - a[1]],
            },
            Self::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => PCurve::Arc {
                center,
                radius,
                start_angle,
                sweep,
            },
        }
    }
}
#[derive(Clone, Debug)]
pub struct ArcLineProfile {
    /// Local XY origin; ring coordinates are offsets. See ArcLineRegion for holes.
    pub origin: Point3,
    pub segments: Vec<PlanarSegment>,
}
/// One simple line/arc outer boundary with disjoint, strictly interior holes.
#[derive(Clone, Debug)]
pub struct ArcLineRegion {
    pub origin: Point3,
    pub outer: Vec<PlanarSegment>,
    pub holes: Vec<Vec<PlanarSegment>>,
}
fn p3(p: P2) -> Point3 {
    Point3::new(p[0], p[1], 0.0)
}
fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn has_angle(segment: PlanarSegment, angle: f64) -> bool {
    let PlanarSegment::Arc {
        start_angle, sweep, ..
    } = segment
    else {
        return false;
    };
    let t = ((angle - start_angle) * sweep.signum()).rem_euclid(TAU);
    t <= sweep.abs() + 1e-10 || TAU - t <= 1e-10
}
fn radial(segment: PlanarSegment, angle: f64) -> P2 {
    let PlanarSegment::Arc { center, radius, .. } = segment else {
        unreachable!()
    };
    [
        center[0] + radius * angle.cos(),
        center[1] + radius * angle.sin(),
    ]
}
pub(crate) fn point_distance(p: P2, segment: PlanarSegment) -> Result<f64> {
    match segment {
        PlanarSegment::Line { a, b } => crate::planar::segment_distance(p, a, b),
        PlanarSegment::Arc {
            center,
            radius,
            start_angle,
            sweep,
        } => {
            let delta = [p[0] - center[0], p[1] - center[1]];
            let mut result =
                distance(p, segment.evaluate(0.0)).min(distance(p, segment.evaluate(1.0)));
            if ((delta[1].atan2(delta[0]) - start_angle) * sweep.signum()).rem_euclid(TAU)
                <= sweep.abs()
            {
                result = result.min((delta[0].hypot(delta[1]) - radius).abs());
            }
            if !result.is_finite() {
                return Err(Error::InvalidInput("arc distance exceeds finite range"));
            }
            Ok(result)
        }
    }
}
fn nearest_point(p: P2, segment: PlanarSegment) -> Result<P2> {
    let result = match segment {
        PlanarSegment::Line { a, b } => {
            let len = distance(a, b);
            let u = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
            let along = ((p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1]).clamp(0.0, len);
            [a[0] + along * u[0], a[1] + along * u[1]]
        }
        PlanarSegment::Arc { center, .. } => {
            let angle = (p[1] - center[1]).atan2(p[0] - center[0]);
            if has_angle(segment, angle) {
                radial(segment, angle)
            } else if distance(p, segment.evaluate(0.0)) < distance(p, segment.evaluate(1.0)) {
                segment.evaluate(0.0)
            } else {
                segment.evaluate(1.0)
            }
        }
    };
    if result.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "closest trim point exceeds finite range",
        ));
    }
    Ok(result)
}
// Endpoints, stationary-distance pairs and intersections of the actual curves.
// Candidate calculations are checked f64 geometry, not display sampling.
fn pair_features(a: PlanarSegment, b: PlanarSegment) -> Result<Vec<(P2, P2)>> {
    let mut pairs = Vec::new();
    for t in [0.0, 0.5, 1.0] {
        let p = a.evaluate(t);
        pairs.push((p, nearest_point(p, b)?));
        let q = b.evaluate(t);
        pairs.push((nearest_point(q, a)?, q));
    }
    match (a, b) {
        (PlanarSegment::Line { a: p, b: q }, PlanarSegment::Line { a: r, b: s }) => {
            if segments_intersect2d(p, q, r, s)? {
                let ua = (p3(q) - p3(p)).normalized()?;
                let ub = (p3(s) - p3(r)).normalized()?;
                let den = ua.cross(ub).z;
                if den != 0.0 {
                    let travel = (p3(r) - p3(p)).cross(ub).z / den;
                    let hit = p3(p) + ua * travel;
                    pairs.push(([hit.x, hit.y], [hit.x, hit.y]));
                }
                // Collinear overlap is covered by endpoint and midpoint pairs.
            }
        }
        (
            line @ PlanarSegment::Line { a: p, b: q },
            arc @ PlanarSegment::Arc { center, radius, .. },
        )
        | (
            arc @ PlanarSegment::Arc { center, radius, .. },
            line @ PlanarSegment::Line { a: p, b: q },
        ) => {
            let len = distance(p, q);
            let u = [(q[0] - p[0]) / len, (q[1] - p[1]) / len];
            let delta = [center[0] - p[0], center[1] - p[1]];
            let along = delta[0] * u[0] + delta[1] * u[1];
            let signed = delta[0] * (-u[1]) + delta[1] * u[0];
            if !along.is_finite() || !signed.is_finite() {
                return Err(Error::InvalidInput(
                    "line/arc projection exceeds finite range",
                ));
            }
            for angle in [u[0].atan2(-u[1]), u[0].atan2(-u[1]) + PI] {
                if has_angle(arc, angle) {
                    let hit = radial(arc, angle);
                    pairs.push((hit, nearest_point(hit, line)?));
                }
            }
            if signed.abs() <= radius {
                let half = radius * (1.0 - (signed / radius).powi(2)).max(0.0).sqrt();
                for t in [along - half, along + half] {
                    if t >= 0.0 && t <= len {
                        let hit = [p[0] + t * u[0], p[1] + t * u[1]];
                        if has_angle(arc, (hit[1] - center[1]).atan2(hit[0] - center[0])) {
                            pairs.push((hit, hit));
                        }
                    }
                }
            }
        }
        (
            PlanarSegment::Arc {
                center: c,
                radius: r,
                ..
            },
            PlanarSegment::Arc {
                center: d,
                radius: t,
                ..
            },
        ) => {
            let span = distance(c, d);
            if !span.is_finite() || !(r + t).is_finite() {
                return Err(Error::InvalidInput(
                    "arc/arc separation exceeds finite range",
                ));
            }
            if span > 0.0 {
                let angle = (d[1] - c[1]).atan2(d[0] - c[0]);
                for x in [angle, angle + PI] {
                    for y in [angle, angle + PI] {
                        if has_angle(a, x) && has_angle(b, y) {
                            pairs.push((radial(a, x), radial(b, y)));
                        }
                    }
                }
                if span >= (r - t).abs() && span <= r + t {
                    let x = span * 0.5 + ((r - t) / span) * ((r + t) * 0.5);
                    let h = r * (1.0 - (x / r).powi(2)).max(0.0).sqrt();
                    let u = [(d[0] - c[0]) / span, (d[1] - c[1]) / span];
                    for sign in [-1.0, 1.0] {
                        let hit = [
                            c[0] + u[0] * x - u[1] * h * sign,
                            c[1] + u[1] * x + u[0] * h * sign,
                        ];
                        if has_angle(a, (hit[1] - c[1]).atan2(hit[0] - c[0]))
                            && has_angle(b, (hit[1] - d[1]).atan2(hit[0] - d[0]))
                        {
                            pairs.push((hit, hit));
                        }
                    }
                }
            }
        }
    }
    if pairs
        .iter()
        .any(|(a, b)| a.iter().chain(b).any(|v| !v.is_finite()) || !distance(*a, *b).is_finite())
    {
        return Err(Error::InvalidInput(
            "trim candidate exceeds finite coordinate range",
        ));
    }
    Ok(pairs)
}
fn boundary_distance(a: PlanarSegment, b: PlanarSegment) -> Result<f64> {
    if let (PlanarSegment::Line { a: p, b: q }, PlanarSegment::Line { a: r, b: s }) = (a, b) {
        if segments_intersect2d(p, q, r, s)? {
            return Ok(0.0);
        }
    }
    Ok(pair_features(a, b)?
        .into_iter()
        .map(|(p, q)| distance(p, q))
        .fold(f64::INFINITY, f64::min))
}
fn loop_area(segments: &[PlanarSegment]) -> f64 {
    crate::topology::wire_area(&Wire {
        coedges: segments
            .iter()
            .enumerate()
            .map(|(edge, s)| Coedge {
                edge,
                forward: s.forward(),
                pcurve: s.pcurve(),
            })
            .collect(),
    })
}
pub(crate) fn validate_mixed(segments: &[PlanarSegment], tol: Tolerance) -> Result<()> {
    validate_mixed_impl(segments, tol, false)
}
fn validate_mixed_impl(
    segments: &[PlanarSegment],
    tol: Tolerance,
    subdivisions: bool,
) -> Result<()> {
    Tolerance::new(tol.linear)?;
    if segments.len() < 2 || segments.len() > 1024 {
        return Err(Error::Unsupported(
            "mixed profiles require 2..1024 segments",
        ));
    }

    for segment in segments {
        match *segment {
            PlanarSegment::Line { a, b } => {
                if a.into_iter().chain(b).any(|x| !x.is_finite())
                    || !distance(a, b).is_finite()
                    || distance(a, b) <= 10.0 * tol.linear
                {
                    return Err(Error::InvalidInput(
                        "mixed line must have finite length exceeding ten tolerances",
                    ));
                }
            }
            PlanarSegment::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => {
                if center.iter().any(|x| !x.is_finite())
                    || !radius.is_finite()
                    || radius <= 10.0 * tol.linear
                    || !start_angle.is_finite()
                    || start_angle.abs() > TAU
                    || !sweep.is_finite()
                    || sweep == 0.0
                    || sweep.abs() > PI
                    || !((radius * sweep.abs()).is_finite())
                    || radius * sweep.abs() <= 10.0 * tol.linear
                {
                    return Err(Error::InvalidInput("arc requires finite center/radius, start in [-2pi,2pi], signed nonzero sweep with abs(sweep)<=pi, and size exceeding ten tolerances"));
                }
                if !((center[0].abs() + radius).is_finite()
                    && (center[1].abs() + radius).is_finite())
                {
                    return Err(Error::InvalidInput("arc exceeds finite coordinates"));
                }
                if distance(segment.evaluate(0.0), segment.evaluate(1.0)) <= 10.0 * tol.linear {
                    return Err(Error::InvalidInput(
                        "arc endpoints are unresolved at this tolerance",
                    ));
                }
            }
        }
    }
    for i in 0..segments.len() {
        let a = segments[i];
        let b = segments[(i + 1) % segments.len()];
        if distance(a.evaluate(1.0), b.evaluate(0.0)) > tol.linear {
            return Err(Error::InvalidInput("mixed profile is not closed"));
        }
        let incoming = a.tangent(true)?;
        let outgoing = b.tangent(false)?;
        if (incoming + outgoing).norm() <= 1e-10 {
            return Err(Error::InvalidInput(
                "mixed profile has an unresolved cusp/backtracking join",
            ));
        }
        if matches!(
            (a, b),
            (PlanarSegment::Line { .. }, PlanarSegment::Line { .. })
        ) {
            let length = distance(a.evaluate(0.0), a.evaluate(1.0))
                .min(distance(b.evaluate(0.0), b.evaluate(1.0)));
            if incoming.cross(outgoing).norm() * length <= tol.linear
                && (!subdivisions || incoming.dot(outgoing) <= 0.0)
            {
                return Err(Error::InvalidInput(
                    "merge redundant or near-collinear straight corners",
                ));
            }
        }
        for j in i + 1..segments.len() {
            let adjacent = j == i + 1 || (i == 0 && j == segments.len() - 1);
            if adjacent {
                let mut joins = Vec::new();
                if j == i + 1 {
                    joins.push(a.evaluate(1.0));
                }
                if i == 0 && j == segments.len() - 1 {
                    joins.push(a.evaluate(0.0));
                }
                for (p, q) in pair_features(a, segments[j])? {
                    if distance(p, q) <= tol.linear
                        && !joins.iter().any(|&joint| {
                            distance(p, joint) <= tol.linear && distance(q, joint) <= tol.linear
                        })
                    {
                        return Err(Error::InvalidInput("adjacent mixed segments overlap, recross or nearly touch away from their join"));
                    }
                }
            } else if boundary_distance(a, segments[j])?
                <= separation_budget(&[a], &[segments[j]], tol)
            {
                return Err(Error::InvalidInput(
                    "mixed profile crosses, touches or nearly touches itself",
                ));
            }
        }
    }
    let area = loop_area(segments);
    if !area.is_finite() || area.abs() <= tol.linear * tol.linear {
        return Err(Error::InvalidInput(
            "mixed profile has no finite nonzero area",
        ));
    }
    Ok(())
}
fn arc_frame(origin: Point3, center: P2, start: f64, tol: Tolerance) -> Result<Frame3> {
    Frame3::new(
        origin + p3(center),
        [
            Vec3::new(start.cos(), start.sin(), 0.0),
            Vec3::new(-start.sin(), start.cos(), 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        tol,
    )
}
pub(crate) fn point_location(
    p: P2,
    segments: &[PlanarSegment],
    tol: Tolerance,
) -> Result<PointLocation> {
    if p.iter().any(|x| !x.is_finite()) {
        return Err(Error::InvalidInput(
            "mixed point classification requires finite coordinates",
        ));
    }
    for &segment in segments {
        if point_distance(p, segment)? <= tol.linear {
            return Ok(PointLocation::Boundary);
        }
    }
    // The boundary check supplies a boundary-free ball of radius `linear`.
    // Shift the analytic ray within that ball to avoid floating endpoint seams.
    let mut p = p;
    let critical: Vec<f64> = segments
        .iter()
        .flat_map(|s| {
            let mut ys = vec![s.evaluate(0.0)[1], s.evaluate(1.0)[1]];
            if let PlanarSegment::Arc { center, radius, .. } = *s {
                ys.extend([center[1] - radius, center[1] + radius]);
            }
            ys
        })
        .collect();
    let seams: Vec<_> = segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let a = s.evaluate(1.0)[1];
            let b = segments[(i + 1) % segments.len()].evaluate(0.0)[1];
            [a.min(b), a.max(b)]
        })
        .collect();
    let y = [0.25, -0.25, 0.5, -0.5, 0.75, -0.75]
        .into_iter()
        .map(|f| p[1] + f * tol.linear)
        .find(|y| {
            y.is_finite()
                && seams.iter().all(|s| *y < s[0] || *y > s[1])
                && critical.iter().all(|c| {
                    (y - c).abs() > 64.0 * f64::EPSILON * y.abs().max(c.abs()).max(tol.linear)
                })
        })
        .ok_or(Error::InvalidInput(
            "unresolved ray at endpoint levels; increase model tolerance",
        ))?;
    p[1] = y;
    let mut inside = false;
    for &segment in segments {
        match segment {
            PlanarSegment::Line { a, b } => {
                if (a[1] > p[1]) != (b[1] > p[1]) {
                    let side = orient2d(a, b, p)?;
                    if (b[1] > a[1] && side == Orientation::CounterClockwise)
                        || (b[1] < a[1] && side == Orientation::Clockwise)
                    {
                        inside = !inside;
                    }
                }
            }
            PlanarSegment::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => {
                let mut fractions = vec![0.0, 1.0];
                for angle in [PI * 0.5, PI * 1.5] {
                    let step = ((angle - start_angle) * sweep.signum()).rem_euclid(TAU);
                    if step > 0.0 && step < sweep.abs() {
                        fractions.push(step / sweep.abs());
                    }
                }
                fractions.sort_by(f64::total_cmp);
                for pair in fractions.windows(2) {
                    let a = segment.evaluate(pair[0]);
                    let b = segment.evaluate(pair[1]);
                    if (a[1] > p[1]) != (b[1] > p[1]) {
                        let ratio = (p[1] - center[1]) / radius;
                        if !ratio.is_finite() || ratio.abs() > 1.0 + 64.0 * f64::EPSILON {
                            return Err(Error::InvalidInput(
                                "unresolved horizontal ray/arc classification",
                            ));
                        }
                        let middle = start_angle + sweep * (pair[0] + pair[1]) * 0.5;
                        let x = center[0]
                            + middle.cos().signum()
                                * radius
                                * (1.0 - ratio * ratio).max(0.0).sqrt();
                        if !x.is_finite() {
                            return Err(Error::InvalidInput("ray/arc root exceeds finite range"));
                        }
                        if p[0] < x {
                            inside = !inside;
                        }
                    }
                }
            }
        }
    }
    Ok(if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    })
}
/// Checks a simple ring, then classifies a point using analytic ray/circle roots
/// and exact line signs. Boundary means within the absolute linear tolerance.
pub fn classify_arc_line_point(
    p: P2,
    segments: &[PlanarSegment],
    tol: Tolerance,
) -> Result<PointLocation> {
    validate_mixed(segments, tol)?;
    point_location(p, segments, tol)
}
fn loops_distance(a: &[PlanarSegment], b: &[PlanarSegment]) -> Result<f64> {
    let mut distance = f64::INFINITY;
    for &x in a {
        for &y in b {
            distance = distance.min(boundary_distance(x, y)?);
        }
    }
    Ok(distance)
}
fn separation_budget(a: &[PlanarSegment], b: &[PlanarSegment], tol: Tolerance) -> f64 {
    let scale = a
        .iter()
        .chain(b)
        .map(|s| match *s {
            PlanarSegment::Line { a, b } => distance(a, b),
            PlanarSegment::Arc { radius, .. } => radius,
        })
        .fold(tol.linear, f64::max);
    tol.linear + 64.0 * f64::EPSILON * scale
}
pub(crate) fn validate_mixed_region(loops: &[Vec<PlanarSegment>], tol: Tolerance) -> Result<()> {
    validate_mixed_region_impl(loops, tol, false)
}
pub(crate) fn validate_mixed_region_trim(
    loops: &[Vec<PlanarSegment>],
    tol: Tolerance,
) -> Result<()> {
    validate_mixed_region_impl(loops, tol, true)
}
fn validate_mixed_region_impl(
    loops: &[Vec<PlanarSegment>],
    tol: Tolerance,
    subdivisions: bool,
) -> Result<()> {
    Tolerance::new(tol.linear)?;
    if loops.is_empty() || loops.len() > 257 || loops.iter().map(Vec::len).sum::<usize>() > 4096 {
        return Err(Error::Unsupported(
            "mixed regions support one outer ring, at most 256 holes and 4096 total segments",
        ));
    }
    for ring in loops {
        validate_mixed_impl(ring, tol, subdivisions)?;
    }
    for hole in &loops[1..] {
        if loops_distance(&loops[0], hole)? <= separation_budget(&loops[0], hole, tol)
            || point_location(hole[0].evaluate(0.0), &loops[0], tol)? != PointLocation::Inside
        {
            return Err(Error::InvalidInput(
                "mixed hole touches, crosses or leaves the outer ring",
            ));
        }
    }
    for i in 1..loops.len() {
        for j in i + 1..loops.len() {
            if loops_distance(&loops[i], &loops[j])? <= separation_budget(&loops[i], &loops[j], tol)
                || point_location(loops[i][0].evaluate(0.0), &loops[j], tol)?
                    != PointLocation::Outside
                || point_location(loops[j][0].evaluate(0.0), &loops[i], tol)?
                    != PointLocation::Outside
            {
                return Err(Error::InvalidInput(
                    "mixed holes overlap, touch, nearly touch or nest",
                ));
            }
        }
    }
    Ok(())
}
fn normalized_ring(ring: &[PlanarSegment], positive: bool) -> Vec<PlanarSegment> {
    if (loop_area(ring) > 0.0) == positive {
        ring.to_vec()
    } else {
        ring.iter().rev().map(|s| s.reversed()).collect()
    }
}
/// Exact positive-Z extrusion of a simple, possibly concave line/arc ring.
/// Either input winding is accepted. For holes use `extrude_arc_line_region`.
pub fn extrude_arc_line(profile: &ArcLineProfile, height: f64, tol: Tolerance) -> Result<Solid> {
    extrude_arc_line_region(
        &ArcLineRegion {
            origin: profile.origin,
            outer: profile.segments.clone(),
            holes: Vec::new(),
        },
        height,
        tol,
    )
}
/// Extrude a line/arc profile in a rigid frame along a world-space vector.
/// Coordinates, including `profile.origin`, are frame-local. Either normal sign
/// is supported, including skew extrusion with exact circular translation walls.
pub fn extrude_arc_line_in_frame(
    profile: &ArcLineProfile,
    world_direction: Vec3,
    frame: Frame3,
    tol: Tolerance,
) -> Result<Solid> {
    extrude_arc_line_region_in_frame(
        &ArcLineRegion {
            origin: profile.origin,
            outer: profile.segments.clone(),
            holes: Vec::new(),
        },
        world_direction,
        frame,
        tol,
    )
}
/// Vector extrusion of a framed region with bounded arcs and disjoint holes.
/// Negative direction shifts the construction base to the terminal plane; face
/// indices have no start/end ordering guarantee. Normal height must exceed ten
/// linear tolerances. Skew vectors retain exact tangential displacement beyond
/// the existing normal conversion allowance (64 EPSILON times vector length).
pub fn extrude_arc_line_region_in_frame(
    profile: &ArcLineRegion,
    world_direction: Vec3,
    frame: Frame3,
    tol: Tolerance,
) -> Result<Solid> {
    Tolerance::new(tol.linear)?;
    if !world_direction.finite() {
        return Err(Error::InvalidInput(
            "mixed framed extrusion requires a finite direction",
        ));
    }
    let local = frame.local_vector(world_direction);
    let length = world_direction.norm();
    if !local.finite() || !length.is_finite() {
        return Err(Error::InvalidInput(
            "mixed framed direction exceeds finite range",
        ));
    }
    // Preserve the existing normal-direction roundoff contract.
    let local = if local.x.hypot(local.y) <= 64. * f64::EPSILON * length {
        Vec3::new(0., 0., local.z)
    } else {
        local
    };
    extrude_arc_line_region_along(profile, local, tol)?.transformed(frame, tol)
}
/// Tilted, negative-normal arc-notch fixture, shared by native and WASM demos.
pub fn framed_arc_extrusion_demo(radius: f64) -> Result<Solid> {
    let tol = Tolerance::default();
    let frame = Transform::translation(Vec3::new(8., -4., 6.))?
        .compose(Transform::rotation(Vec3::new(1., 2., 0.5), 0.8)?)?;
    let mut profile = notched_demo_profile(radius, tol)?;
    profile.origin = Point3::new(0., 0., 12.);
    extrude_arc_line_region_in_frame(&profile, frame.vector(Vec3::new(0., 0., -24.)), frame, tol)
}
/// Tilted, signed skew extrusion fixture used by native/WASM and browser checks.
pub fn skew_arc_extrusion_demo(radius: f64, offset: f64, height: f64) -> Result<Solid> {
    let tol = Tolerance::default();
    let frame = Transform::translation(Vec3::new(8., -4., 6.))?
        .compose(Transform::rotation(Vec3::new(1., 2., 0.5), 0.8)?)?;
    let mut profile = notched_demo_profile(radius, tol)?;
    profile.origin = Point3::new(0., 0., 12.);
    extrude_arc_line_region_in_frame(
        &profile,
        frame.vector(Vec3::new(offset, -0.5 * offset, height)),
        frame,
        tol,
    )
}
pub fn skew_arc_extrusion_demo_json(radius: f64, offset: f64, height: f64) -> Result<String> {
    skew_arc_extrusion_demo(radius, offset, height)?.mesh_json(0.05, Tolerance::default())
}
/// Exact normal extrusion of a simple region with signed circular arcs and
/// disjoint holes. Normalizes outer CCW/holes CW; rejects all touch/nesting.
/// Height must be positive. Use the vector/frame APIs for signed or skew extrusion.
pub fn extrude_arc_line_region(
    profile: &ArcLineRegion,
    height: f64,
    tol: Tolerance,
) -> Result<Solid> {
    if !height.is_finite() || height <= 0. {
        return Err(Error::InvalidInput(
            "mixed height-only extrusion requires positive finite height",
        ));
    }
    extrude_arc_line_region_along(profile, Vec3::new(0., 0., height), tol)
}
/// Exact XY line/arc region extrusion along a finite vector with resolved normal span.
/// Supports skew and either normal sign; in-plane extrusion is rejected.
pub fn extrude_arc_line_region_along(
    profile: &ArcLineRegion,
    direction: Vec3,
    tol: Tolerance,
) -> Result<Solid> {
    Tolerance::new(tol.linear)?;
    if !direction.finite() || direction.z.abs() <= 10. * tol.linear {
        return Err(Error::InvalidInput(
            "mixed extrusion requires finite direction and normal span exceeding ten tolerances",
        ));
    }
    let mut base = profile.clone();
    let direction = if direction.z < 0. {
        base.origin = base.origin + direction;
        direction * (-1.)
    } else {
        direction
    };
    extrude_mixed_positive(&base, direction, tol)
}
fn extrude_mixed_positive(
    profile: &ArcLineRegion,
    direction: Vec3,
    tol: Tolerance,
) -> Result<Solid> {
    let height = direction.z;
    let input: Vec<_> = std::iter::once(&profile.outer)
        .chain(&profile.holes)
        .cloned()
        .collect();
    validate_mixed_region(&input, tol)?;
    if !profile.origin.finite()
        || !height.is_finite()
        || height <= 10.0 * tol.linear
        || !(profile.origin + direction).finite()
    {
        return Err(Error::InvalidInput(
            "mixed extrusion requires finite origin and positive height exceeding ten tolerances",
        ));
    }
    let rings: Vec<_> = input
        .iter()
        .enumerate()
        .map(|(i, ring)| normalized_ring(ring, i == 0))
        .collect();
    let segments: Vec<_> = rings.iter().flatten().copied().collect();
    let mut next = Vec::new();
    let mut offset = 0;
    for ring in &rings {
        for i in 0..ring.len() {
            next.push(offset + (i + 1) % ring.len());
        }
        offset += ring.len();
    }
    let n = segments.len();
    let mut s = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    for z in [0.0, height] {
        for segment in &segments {
            s.vertices.push(Vertex {
                point: profile.origin + p3(segment.evaluate(0.0)) + direction * (z / height),
            });
        }
    }
    for (i, &segment) in segments.iter().enumerate() {
        let j = next[i];
        let canonical = segment.canonical();
        for (offset, z) in [(0, 0.0), (n, height)] {
            let curve = match canonical {
                PlanarSegment::Line { a, b } => Curve::Line {
                    a: profile.origin + p3(a) + direction * (z / height),
                    b: profile.origin + p3(b) + direction * (z / height),
                },
                PlanarSegment::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } => Curve::Arc {
                    frame: arc_frame(
                        profile.origin + direction * (z / height),
                        center,
                        start_angle,
                        tol,
                    )?,
                    radius,
                    sweep,
                },
            };
            let vertices = if segment.forward() {
                [i + offset, j + offset]
            } else {
                [j + offset, i + offset]
            };
            s.edges.push(Edge { vertices, curve });
        }
        s.edges.push(Edge {
            vertices: [i, i + n],
            curve: Curve::Line {
                a: s.vertices[i].point,
                b: s.vertices[i + n].point,
            },
        });
    }
    for (top, orientation) in [(false, -1), (true, 1)] {
        let mut offset = 0;
        let mut wires = Vec::new();
        for ring in &rings {
            wires.push(Wire {
                coedges: ring
                    .iter()
                    .enumerate()
                    .map(|(i, segment)| Coedge {
                        edge: 3 * (i + offset) + usize::from(top),
                        forward: segment.forward(),
                        pcurve: segment.pcurve(),
                    })
                    .collect(),
            });
            offset += ring.len();
        }
        s.shell.faces.push(Face {
            surface: Surface::Plane {
                origin: profile.origin + direction * if top { 1. } else { 0. },
                u: Vec3::new(1.0, 0.0, 0.0),
                v: Vec3::new(0.0, 1.0, 0.0),
            },
            orientation,
            wires,
        });
    }
    for (i, &segment) in segments.iter().enumerate() {
        let j = next[i];
        let canonical = segment.canonical();
        let (surface, span) = match canonical {
            PlanarSegment::Line { a, b } => {
                let u = (p3(b) - p3(a)).normalized()?;
                let v = (direction - u * direction.dot(u)).normalized()?;
                (
                    Surface::Plane {
                        origin: profile.origin + p3(a),
                        u,
                        v,
                    },
                    distance(a, b),
                )
            }
            PlanarSegment::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => (
                {
                    let frame = arc_frame(profile.origin, center, start_angle, tol)?;
                    if direction.x == 0. && direction.y == 0. {
                        Surface::FramedCylinder {
                            frame,
                            radius,
                            height,
                        }
                    } else {
                        let g = frame.local_vector(direction);
                        Surface::ExtrudedCircle {
                            frame,
                            radius,
                            height,
                            drift: [g.x / height, g.y / height],
                        }
                    }
                },
                sweep,
            ),
        };
        let shift = if let Surface::Plane { u, v, .. } = surface {
            [direction.dot(u), direction.dot(v)]
        } else {
            [0., height]
        };
        let direction = if matches!(canonical, PlanarSegment::Line { .. }) {
            span
        } else {
            1.0
        };
        let (start, end, orientation) = if segment.forward() {
            (i, j, 1)
        } else {
            (j, i, -1)
        };
        let coedge = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        s.shell.faces.push(Face {
            surface,
            orientation,
            wires: vec![Wire {
                coedges: vec![
                    coedge(3 * i, true, [0.0, 0.0], [direction, 0.0]),
                    coedge(3 * end + 2, true, [span, 0.0], shift),
                    coedge(3 * i + 1, false, shift, [direction, 0.0]),
                    coedge(3 * start + 2, false, [0.0, 0.0], shift),
                ],
            }],
        });
    }
    s.validate(tol)?;
    Ok(s)
}

/// Four exact quarter-circle corners joined tangentially to four straight sides.
pub fn rounded_rectangle_profile(
    origin: Point3,
    width: f64,
    depth: f64,
    radius: f64,
    tol: Tolerance,
) -> Result<ArcLineProfile> {
    Tolerance::new(tol.linear)?;
    if !origin.finite()
        || [width, depth, radius]
            .iter()
            .any(|x| !x.is_finite() || *x <= 10.0 * tol.linear)
        || width - 2.0 * radius <= 10.0 * tol.linear
        || depth - 2.0 * radius <= 10.0 * tol.linear
    {
        return Err(Error::InvalidInput(
            "rounded rectangle requires finite positive dimensions and resolved straight sides",
        ));
    }
    let (x, y) = (width * 0.5, depth * 0.5);
    let line = |a, b| PlanarSegment::Line { a, b };
    let arc = |center, start_angle| PlanarSegment::Arc {
        center,
        radius,
        start_angle,
        sweep: PI * 0.5,
    };
    let profile = ArcLineProfile {
        origin,
        segments: vec![
            line([-x + radius, -y], [x - radius, -y]),
            arc([x - radius, -y + radius], -PI * 0.5),
            line([x, -y + radius], [x, y - radius]),
            arc([x - radius, y - radius], 0.0),
            line([x - radius, y], [-x + radius, y]),
            arc([-x + radius, y - radius], PI * 0.5),
            line([-x, y - radius], [-x, -y + radius]),
            arc([-x + radius, -y + radius], PI),
        ],
    };
    validate_mixed(&profile.segments, tol)?;
    Ok(profile)
}

pub(crate) fn notched_demo_profile(radius: f64, tol: Tolerance) -> Result<ArcLineRegion> {
    if !radius.is_finite() || radius <= 10.0 * tol.linear || radius >= 30.0 - 10.0 * tol.linear {
        return Err(Error::InvalidInput(
            "notch radius must be positive and less than 30 model units",
        ));
    }
    let line = |a, b| PlanarSegment::Line { a, b };
    let mut hole =
        rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 20.0, 10.0, 3.0, tol)?.segments;
    for segment in &mut hole {
        match segment {
            PlanarSegment::Line { a, b } => {
                a[1] -= 10.0;
                b[1] -= 10.0;
            }
            PlanarSegment::Arc { center, .. } => center[1] -= 10.0,
        }
    }
    Ok(ArcLineRegion {
        origin: Point3::new(0.0, 0.0, -12.0),
        outer: vec![
            line([-40.0, -30.0], [40.0, -30.0]),
            line([40.0, -30.0], [40.0, 30.0]),
            line([40.0, 30.0], [radius, 30.0]),
            PlanarSegment::Arc {
                center: [0.0, 30.0],
                radius,
                start_angle: 0.0,
                sweep: -PI,
            },
            line([-radius, 30.0], [-40.0, 30.0]),
            line([-40.0, 30.0], [-40.0, -30.0]),
        ],
        holes: vec![hole],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn finite_arc_distance_does_not_extend_endpoint_by_angular_slack() {
        let arc = super::PlanarSegment::Arc {
            center: [0., 0.],
            radius: 1.,
            start_angle: 0.,
            sweep: std::f64::consts::FRAC_PI_2,
        };
        let angle: f64 = -5e-11;
        let distance = super::point_distance([angle.cos(), angle.sin()], arc).unwrap();
        assert!((distance - 5e-11).abs() < 1e-20);
    }

    use super::*;
    #[test]
    fn analytic_line_arc_distance_checks_interior_extrema_and_intersections() {
        let arc = PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 1.0,
            start_angle: 0.0,
            sweep: PI,
        };
        for (y, expected) in [(2.0, 1.0), (1.0, 0.0), (0.5, 0.0), (1.0 + 1e-7, 1e-7)] {
            let line = PlanarSegment::Line {
                a: [-2.0, y],
                b: [2.0, y],
            };
            assert!((boundary_distance(line, arc).unwrap() - expected).abs() < 1e-14);
            assert!((boundary_distance(arc, line).unwrap() - expected).abs() < 1e-14);
        }
    }
    #[test]
    fn analytic_arc_arc_distance_checks_crossing_concentricity_and_radial_extrema() {
        let a = PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 2.0,
            start_angle: -PI * 0.5,
            sweep: PI,
        };
        let arc = |x, r| PlanarSegment::Arc {
            center: [x, 0.0],
            radius: r,
            start_angle: PI * 0.5,
            sweep: PI,
        };
        assert_eq!(boundary_distance(a, arc(2.0, 2.0)).unwrap(), 0.0);
        assert!((boundary_distance(a, arc(6.0, 1.0)).unwrap() - 3.0).abs() < 1e-14);
        let b = PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 3.0,
            start_angle: 0.0,
            sweep: PI * 0.5,
        };
        assert!((boundary_distance(a, b).unwrap() - 1.0).abs() < 1e-14);
        let b = PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 2.0,
            start_angle: 0.0,
            sweep: PI * 0.5,
        };
        assert_eq!(boundary_distance(a, b).unwrap(), 0.0);
    }
}
