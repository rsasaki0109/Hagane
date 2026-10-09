//! Checked ellipse annuli and chord-closed minor arcs; arbitrary mixed loops are unsupported.
use crate::*;
use std::f64::consts::{PI, TAU};
type P2 = [f64; 2];
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn norm(a: P2) -> f64 {
    a[0].hypot(a[1])
}
// The retained disk lies on the arc side of its closing chord. Keep the
// exact semicircle plane to preserve existing half-ellipse arithmetic.
fn chord_plane(sweep: f64) -> (P2, f64) {
    if sweep == PI {
        ([0., 1.], 0.)
    } else {
        let half = sweep / 2.;
        ([half.cos(), half.sin()], half.cos())
    }
}
#[derive(Clone)]
pub(crate) struct EllipseRing {
    center: P2,
    cosine: P2,
    sine: P2,
    coherence_error: f64,
    segment_sweep: Option<f64>,
}
impl EllipseRing {
    fn scale(&self) -> f64 {
        norm(self.cosine).max(norm(self.sine))
    }
    fn inverse(&self, p: P2) -> Result<P2> {
        self.inverse_vector(sub(p, self.center))
    }
    fn inverse_vector(&self, delta: P2) -> Result<P2> {
        let scale = self.scale();
        let a = self.cosine.map(|x| x / scale);
        let b = self.sine.map(|x| x / scale);
        let q = delta.map(|x| x / scale);
        let det = cross(a, b);
        if !det.is_finite() || det.abs() <= 1e-10 {
            return Err(Error::Unsupported(
                "planar ellipse axes are ill-conditioned",
            ));
        }
        let uv = [cross(q, b) / det, cross(a, q) / det];
        if uv.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput(
                "ellipse coordinates exceed finite range",
            ));
        }
        Ok(uv)
    }
    fn point(&self, u: f64) -> P2 {
        std::array::from_fn(|i| self.center[i] + self.cosine[i] * u.cos() + self.sine[i] * u.sin())
    }
    pub(crate) fn location(&self, p: P2) -> Result<PointLocation> {
        let uv = self.inverse(p)?;
        let r = norm(uv);
        if let Some(sweep) = self.segment_sweep {
            let (n, level) = chord_plane(sweep);
            let side = dot(n, uv) - level;
            if side < 0. {
                return Ok(PointLocation::Outside);
            }
            if side == 0. && r <= 1. {
                return Ok(PointLocation::Boundary);
            }
        }
        Ok(if r == 1. {
            PointLocation::Boundary
        } else if r < 1. {
            PointLocation::Inside
        } else {
            PointLocation::Outside
        })
    }
    /// Bound physical distance with chord interpolation; no display mesh is read.
    pub(crate) fn within_boundary(&self, p: P2, budget: f64) -> Result<bool> {
        let curvature = norm(self.cosine) + norm(self.sine);
        let roundoff = 512. * f64::EPSILON * (self.scale() + norm(self.center) + norm(p))
            + self.coherence_error;
        if !budget.is_finite() || roundoff >= budget {
            return Err(Error::Unsupported(
                "planar ellipse distance has insufficient precision",
            ));
        }
        let uv = self.inverse(p)?;
        let angle = uv[1].atan2(uv[0]).rem_euclid(TAU);
        let mut chord_unresolved = false;
        if let Some(sweep) = self.segment_sweep {
            let a = self.point(0.);
            let b = self.point(sweep);
            let e = sub(b, a);
            let length = norm(e);
            let unit = e.map(|x| x / length);
            let t = dot(sub(p, a), unit).clamp(0., length);
            let distance = norm(sub(p, [a[0] + unit[0] * t, a[1] + unit[1] * t]));
            if distance + roundoff <= budget {
                return Ok(true);
            }
            chord_unresolved = (distance - budget).abs() <= roundoff;
        }
        let sweep = self.segment_sweep.unwrap_or(TAU);
        if angle <= sweep && norm(sub(p, self.point(angle))) + roundoff <= budget {
            return Ok(true);
        }
        let patches = (sweep / (PI / 2.)).ceil() as usize;
        let mut pending: Vec<_> = (0..patches)
            .map(|i| {
                (
                    sweep * i as f64 / patches as f64,
                    sweep * (i + 1) as f64 / patches as f64,
                    0,
                )
            })
            .collect();
        let mut visits = 0;
        while let Some((lo, hi, depth)) = pending.pop() {
            visits += 1;
            if visits > 16384 || depth > 48 {
                return Err(Error::Unsupported(
                    "planar ellipse distance remains unresolved",
                ));
            }
            let a = self.point(lo);
            let b = self.point(hi);
            let e = sub(b, a);
            let length = norm(e);
            if !length.is_finite() || length == 0. {
                return Err(Error::Unsupported("ellipse chord loses angular resolution"));
            }
            let unit = e.map(|x| x / length);
            let t = dot(sub(p, a), unit).clamp(0., length);
            let q = [a[0] + unit[0] * t, a[1] + unit[1] * t];
            let error = curvature * (hi - lo).powi(2) / 8.;
            if norm(sub(p, q)) - error - roundoff > budget {
                continue;
            }
            let middle = lo + (hi - lo) / 2.;
            for u in [lo, hi, middle, angle.clamp(lo, hi)] {
                if norm(sub(p, self.point(u))) + roundoff <= budget {
                    return Ok(true);
                }
            }
            if middle == lo || middle == hi {
                return Err(Error::Unsupported(
                    "ellipse distance refinement loses angular resolution",
                ));
            }
            pending.push((lo, middle, depth + 1));
            pending.push((middle, hi, depth + 1));
        }
        if chord_unresolved {
            return Err(Error::Unsupported("ellipse chord distance is unresolved"));
        }
        Ok(false)
    }
}
pub(crate) fn has_ellipse(f: &Face) -> bool {
    f.wires
        .iter()
        .flat_map(|w| &w.coedges)
        .any(|c| matches!(c.pcurve, PCurve::EllipseArc { .. }))
}
/// Validated ellipse material with holes certified by normalized enclosing circles.
pub(crate) struct EllipseRegion {
    rings: Vec<EllipseRing>,
}
impl EllipseRegion {
    pub(crate) fn location(&self, p: P2) -> Result<PointLocation> {
        let outer = self.rings[0].location(p)?;
        if outer != PointLocation::Inside {
            return Ok(outer);
        }
        for hole in &self.rings[1..] {
            match hole.location(p)? {
                PointLocation::Inside => return Ok(PointLocation::Outside),
                PointLocation::Boundary => return Ok(PointLocation::Boundary),
                PointLocation::Outside => (),
            }
        }
        Ok(PointLocation::Inside)
    }
    pub(crate) fn within_boundary(&self, p: P2, budget: f64) -> Result<bool> {
        let mut unresolved = None;
        for ring in &self.rings {
            match ring.within_boundary(p, budget) {
                Ok(true) => return Ok(true),
                Ok(false) => (),
                Err(error) => unresolved = Some(error),
            }
        }
        if let Some(error) = unresolved {
            return Err(error);
        }
        Ok(false)
    }
}
pub(crate) fn ring(face: &Face, tol: Tolerance) -> Result<EllipseRegion> {
    if !matches!(face.surface, Surface::Plane { .. })
        || face.wires.is_empty()
        || face.wires.len() > 17
    {
        return Err(Error::Unsupported(
            "ellipse trims require an outer wire and at most sixteen supported holes",
        ));
    }
    let rings = (0..face.wires.len())
        .map(|wire| single_ring(face, wire, tol))
        .collect::<Result<Vec<_>>>()?;
    let mut normalized_holes = Vec::new();
    for hole in &rings[1..] {
        let outer = &rings[0];
        if outer.segment_sweep.is_some() || hole.segment_sweep.is_some() {
            return Err(Error::Unsupported(
                "ellipse holes require complete ellipse wires",
            ));
        }
        // The largest singular value encloses any affine ellipse in a circle
        // in outer normalized coordinates, including rotated/nonorthogonal axes.
        let x = outer.inverse_vector(hole.cosine)?;
        let y = outer.inverse_vector(hole.sine)?;
        let matrix_scale = norm(x).max(norm(y));
        if !matrix_scale.is_finite() || matrix_scale == 0. {
            return Err(Error::Unsupported("ellipse hole envelope is unresolved"));
        }
        let x = x.map(|v| v / matrix_scale);
        let y = y.map(|v| v / matrix_scale);
        let a = dot(x, x);
        let b = dot(x, y);
        let c = dot(y, y);
        let ratio = matrix_scale
            * ((a + c + (a - c).hypot(2. * b)) / 2.).sqrt()
            * (1. + 512. * f64::EPSILON);
        if !ratio.is_finite() || ratio >= 1. {
            return Err(Error::Unsupported(
                "ellipse hole envelope is not strictly smaller than the outer ellipse",
            ));
        }
        let error = hole.coherence_error + outer.coherence_error;
        let scale = outer.scale();
        let a = outer.cosine.map(|x| x / scale);
        let b = outer.sine.map(|x| x / scale);
        let center_offset = outer.inverse(hole.center)?;
        let sigma_lower = scale * cross(a, b).abs() / (norm(a) + norm(b));
        let clearance = (1. - ratio - norm(center_offset)) * sigma_lower;
        let arithmetic = 512. * f64::EPSILON * (scale + norm(outer.center) + norm(hole.center));
        if !clearance.is_finite() || clearance <= 10. * tol.linear + error + arithmetic {
            return Err(Error::Unsupported("ellipse hole clearance is unresolved"));
        }
        normalized_holes.push((ratio, center_offset, sigma_lower, error + arithmetic));
    }
    for (i, &(radius, center, sigma, error)) in normalized_holes.iter().enumerate() {
        for (j, &(other_radius, other_center, _, other_error)) in
            normalized_holes[..i].iter().enumerate()
        {
            let clearance = (norm(sub(center, other_center)) - radius - other_radius) * sigma;
            let budget = 10. * tol.linear + error + other_error;
            if !clearance.is_finite() || clearance <= budget {
                let first = &rings[i + 1];
                // Each direction supplies an analytic supporting-line certificate.
                // Trying finitely many directions can miss separation, but cannot
                // accept overlapping ellipses when the guarded gap is positive.
                let separated = separated_ellipses(first, &rings[j + 1], budget);
                if !separated {
                    return Err(Error::Unsupported(
                        "ellipse holes have no certified separating line",
                    ));
                }
            }
        }
    }
    Ok(EllipseRegion { rings })
}
fn separated_ellipses(a: &EllipseRing, b: &EllipseRing, budget: f64) -> bool {
    let delta = sub(b.center, a.center);
    let length = norm(delta);
    if !length.is_finite() || length == 0. {
        return false;
    }
    let initial = delta[1].atan2(delta[0]);
    let arithmetic =
        2048. * f64::EPSILON * (norm(a.center) + norm(b.center) + a.scale() + b.scale());
    (0..64).any(|i| {
        let angle = initial + PI * i as f64 / 64.;
        let n = [angle.cos(), angle.sin()];
        let ra = dot(n, a.cosine).hypot(dot(n, a.sine));
        let rb = dot(n, b.cosine).hypot(dot(n, b.sine));
        let gap = dot(n, delta).abs() - ra - rb;
        gap.is_finite() && gap > budget + arithmetic
    })
}
/// One full ellipse, or a minor ellipse arc followed by its closing chord.
fn single_ring(face: &Face, wire: usize, tol: Tolerance) -> Result<EllipseRing> {
    if !matches!(face.surface, Surface::Plane { .. }) || face.wires[wire].coedges.len() != 2 {
        return Err(Error::Unsupported(
            "planar ellipse trims require one supported two-coedge wire",
        ));
    }
    let coedges = &face.wires[wire].coedges;
    let PCurve::EllipseArc {
        center,
        cosine,
        sine,
        sweep,
    } = coedges[0].pcurve
    else {
        return Err(Error::Unsupported(
            "mixed ellipse planar wires are unsupported",
        ));
    };
    let mut ellipse = EllipseRing {
        center,
        cosine,
        sine,
        coherence_error: 0.,
        segment_sweep: None,
    };
    if let PCurve::Affine { origin, direction } = coedges[1].pcurve {
        if !sweep.is_finite() || sweep <= 0. || sweep > PI {
            return Err(Error::Unsupported(
                "ellipse/chord trim requires a sweep in (0, pi]",
            ));
        }
        if [center, cosine, sine, origin, direction]
            .iter()
            .flatten()
            .any(|x| !x.is_finite())
            || norm(cosine) <= tol.linear
            || norm(sine) <= tol.linear
        {
            return Err(Error::InvalidTopology("invalid ellipse/chord coefficients"));
        }
        let first = ellipse.point(if coedges[0].forward { sweep } else { 0. });
        let last = ellipse.point(if coedges[0].forward { 0. } else { sweep });
        let start = coedges[1]
            .pcurve
            .evaluate(if coedges[1].forward { 0. } else { 1. });
        let end = coedges[1]
            .pcurve
            .evaluate(if coedges[1].forward { 1. } else { 0. });
        let error = norm(sub(first, start)) + norm(sub(last, end));
        if error > tol.linear {
            return Err(Error::InvalidTopology(
                "ellipse and chord endpoints disagree",
            ));
        }
        if error > 512. * f64::EPSILON * (ellipse.scale() + norm(center)) {
            return Err(Error::Unsupported(
                "ellipse chord differs beyond checked arithmetic precision",
            ));
        }
        ellipse.coherence_error = error;
        ellipse.segment_sweep = Some(sweep);
        ellipse.inverse(center)?;
        let scale = ellipse.scale();
        let a = cosine.map(|x| x / scale);
        let b = sine.map(|x| x / scale);
        let sigma_lower = scale * cross(a, b).abs() / (norm(a) + norm(b));
        let sagitta = 2. * (sweep / 4.).sin().powi(2) * sigma_lower;
        if !sagitta.is_finite() || sagitta <= 10. * tol.linear {
            return Err(Error::Unsupported(
                "ellipse segment thickness is unresolved",
            ));
        }
        return Ok(ellipse);
    }
    let PCurve::EllipseArc {
        center: c,
        cosine: a,
        sine: b,
        sweep: second,
    } = coedges[1].pcurve
    else {
        return Err(Error::Unsupported(
            "mixed ellipse planar wires are unsupported",
        ));
    };
    if sweep != PI || second != PI || coedges[0].forward != coedges[1].forward {
        return Err(Error::Unsupported(
            "ellipse wire must contain two equally oriented half arcs",
        ));
    }
    if [center, cosine, sine, c, a, b]
        .iter()
        .flatten()
        .any(|x| !x.is_finite())
        || norm(cosine) <= tol.linear
        || norm(sine) <= tol.linear
        || norm(sub(center, c))
            + norm([a[0] + cosine[0], a[1] + cosine[1]])
            + norm([b[0] + sine[0], b[1] + sine[1]])
            > tol.linear
    {
        return Err(Error::InvalidTopology(
            "ellipse wire axes/centers disagree or are degenerate",
        ));
    }
    let coherence = norm(sub(center, c))
        + norm([a[0] + cosine[0], a[1] + cosine[1]])
        + norm([b[0] + sine[0], b[1] + sine[1]]);
    if coherence > 512. * f64::EPSILON * (ellipse.scale() + norm(center)) {
        return Err(Error::Unsupported(
            "half-ellipse conics differ beyond checked arithmetic precision",
        ));
    }
    ellipse.coherence_error = coherence;
    ellipse.inverse(center)?;
    Ok(ellipse)
}
pub(crate) fn clip(
    solid: &Solid,
    face: &Face,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarLineClip> {
    let region = ring(face, tol.absolute())?;
    let mut outer = clip_ring(solid, face, (0, &region.rings[0]), anchor, direction, tol)?;
    if outer.events.is_empty() || region.rings.len() == 1 {
        return Ok(outer);
    }
    for (wire, hole) in region.rings.iter().enumerate().skip(1) {
        outer
            .events
            .extend(clip_ring(solid, face, (wire, hole), anchor, direction, tol)?.events);
    }
    outer
        .events
        .sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
    if outer.events.first().is_none_or(|event| event.wire != 0)
        || outer.events.last().is_none_or(|event| event.wire != 0)
        || outer.events[1..outer.events.len() - 1]
            .chunks(2)
            .any(|pair| pair.len() != 2 || pair[0].wire == 0 || pair[0].wire != pair[1].wire)
    {
        return Err(Error::Unsupported("ellipse hole event order is unresolved"));
    }
    let budget = tol.length_at_scale(region.rings[0].scale())?;
    for pair in outer.events.windows(2) {
        if pair[1].parameter <= pair[0].parameter
            || (pair[1].point - pair[0].point).norm() <= budget
        {
            return Err(Error::Unsupported(
                "ellipse annulus crossings are too close to resolve",
            ));
        }
    }
    outer.intervals = (0..outer.events.len() - 1)
        .step_by(2)
        .map(|i| PlanarClipInterval {
            parameter_range: [outer.events[i].parameter, outer.events[i + 1].parameter],
            start: outer.events[i].clone(),
            end: outer.events[i + 1].clone(),
        })
        .collect();
    Ok(outer)
}
fn clip_ring(
    solid: &Solid,
    face: &Face,
    trim: (usize, &EllipseRing),
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarLineClip> {
    let (wire, ellipse) = trim;
    let Surface::Plane { origin, u, v } = face.surface else {
        unreachable!()
    };
    if !anchor.finite() {
        return Err(Error::InvalidInput(
            "ellipse clipping requires finite anchor",
        ));
    }
    let unit = direction.normalized()?;
    let normal = u.cross(v).normalized()?;
    let offset = anchor - origin;
    let scale = ellipse.scale();
    let budget = tol.length_at_scale(scale)?;
    let roundoff =
        512. * f64::EPSILON * (anchor.norm() + origin.norm() + scale) + ellipse.coherence_error;
    if !roundoff.is_finite() || roundoff >= budget {
        return Err(Error::Unsupported(
            "ellipse clipping loses coordinate precision",
        ));
    }
    if !offset.finite()
        || offset.dot(normal).abs() > budget.min(64. * f64::EPSILON * scale.max(offset.norm()))
        || unit.dot(normal).abs() > 64. * f64::EPSILON
    {
        return Err(Error::InvalidInput(
            "ellipse clipping line must lie in supporting plane",
        ));
    }
    let a = [offset.dot(u), offset.dot(v)];
    let d = [unit.dot(u), unit.dot(v)];
    let q = ellipse.inverse(a)?;
    let transformed = ellipse.inverse_vector(d)?;
    let speed = norm(transformed);
    if !speed.is_finite() || speed == 0. {
        return Err(Error::Unsupported("ellipse line speed is unresolved"));
    }
    let h = transformed.map(|x| x / speed);
    let along = -dot(q, h);
    let signed = cross(h, q);
    let axes_a = ellipse.cosine.map(|x| x / scale);
    let axes_b = ellipse.sine.map(|x| x / scale);
    let sigma_lower = scale * cross(axes_a, axes_b).abs() / (norm(axes_a) + norm(axes_b));
    let guard = budget / sigma_lower + 256. * f64::EPSILON * norm(q);
    if !guard.is_finite() || guard >= 0.1 {
        return Err(Error::Unsupported("ellipse tangent guard is unresolved"));
    }
    let gap = signed.abs() - 1.;
    if gap > guard {
        return Ok(PlanarLineClip {
            events: vec![],
            intervals: vec![],
        });
    }
    if gap.abs() <= guard {
        return Err(Error::Unsupported(
            "tangent or near-tangent ellipse trim cut",
        ));
    }
    let half = ((1. - signed.abs()) * (1. + signed.abs())).sqrt();
    let mut roots = Vec::new();
    for travel in [along - half, along + half] {
        let circle = [q[0] + h[0] * travel, q[1] + h[1] * travel];
        let angle = circle[1].atan2(circle[0]).rem_euclid(TAU);
        if ellipse.segment_sweep.is_some_and(|sweep| angle > sweep) {
            continue;
        }
        let coedge = if ellipse.segment_sweep.is_some() {
            0
        } else {
            usize::from(angle >= PI)
        };
        roots.push((travel, coedge, angle - coedge as f64 * PI));
    }
    if let Some(sweep) = ellipse.segment_sweep {
        let (n, level) = chord_plane(sweep);
        let denominator = dot(n, h);
        let distance = dot(n, q) - level;
        if denominator.abs() <= tol.angular().sin() {
            if distance.abs() <= guard {
                return Err(Error::Unsupported(
                    "line overlaps or nearly overlaps ellipse chord",
                ));
            }
        } else {
            let travel = -distance / denominator;
            let circle = [q[0] + h[0] * travel, q[1] + h[1] * travel];
            if norm(sub(circle, [1., 0.])).min(norm(sub(circle, [sweep.cos(), sweep.sin()])))
                <= guard
            {
                return Err(Error::Unsupported(
                    "ellipse cut is unresolved near a chord vertex",
                ));
            }
            let tangent = [n[1], -n[0]];
            if dot(tangent, circle).abs() < (sweep / 2.).sin() {
                let a = face.wires[wire].coedges[1].pcurve.evaluate(0.);
                let b = face.wires[wire].coedges[1].pcurve.evaluate(1.);
                let target: P2 = std::array::from_fn(|i| {
                    ellipse.center[i] + ellipse.cosine[i] * circle[0] + ellipse.sine[i] * circle[1]
                });
                let delta = sub(b, a);
                let coordinate = usize::from(delta[1].abs() > delta[0].abs());
                let t = (target[coordinate] - a[coordinate]) / delta[coordinate];
                roots.push((travel, 1, t));
            }
        }
    }
    roots.sort_by(|a, b| a.0.total_cmp(&b.0));
    if roots.is_empty() {
        return Ok(PlanarLineClip {
            events: vec![],
            intervals: vec![],
        });
    }
    if roots.len() != 2 {
        return Err(Error::Unsupported(
            "ellipse/chord crossing count is unresolved",
        ));
    }
    let mut events = Vec::new();
    for (travel, coedge, t) in roots {
        let c = &face.wires[wire].coedges[coedge];
        let edge = &solid.edges[c.edge];
        let point = anchor + unit * (travel / speed);
        let parameter = crate::intersections::line_parameter(travel / speed, direction)?;
        let pc = c.pcurve.evaluate(t);
        if !parameter.is_finite()
            || !point.finite()
            || !tol.absolute().coincident(point, edge.curve.evaluate(t))
            || !tol
                .absolute()
                .coincident(point, face.surface.evaluate(pc[0], pc[1]))
            || !tol
                .absolute()
                .coincident(point, anchor + direction * parameter)
        {
            return Err(Error::InvalidInput(
                "ellipse boundary event loses edge/line/surface agreement",
            ));
        }
        if edge
            .curve
            .range()
            .iter()
            .any(|r| (point - edge.curve.evaluate(*r)).norm() <= budget)
        {
            return Err(Error::Unsupported(
                "ellipse cut passes through or near a trim vertex",
            ));
        }
        events.push(PlanarBoundaryEvent {
            parameter,
            point,
            wire,
            coedge,
            edge: c.edge,
            edge_parameter: t,
        });
    }
    if events[1].parameter <= events[0].parameter
        || (events[1].point - events[0].point).norm() <= budget
    {
        return Err(Error::Unsupported(
            "ellipse clip interval is unrepresentable",
        ));
    }
    Ok(PlanarLineClip {
        intervals: vec![PlanarClipInterval {
            parameter_range: [events[0].parameter, events[1].parameter],
            start: events[0].clone(),
            end: events[1].clone(),
        }],
        events,
    })
}
/// Actual closed B-rep fixture: lower part of a two-arc cylinder, capped by a
/// transverse plane with a full ellipse wire. This is not a general partition API.
pub fn ellipse_planar_demo_solid(
    radius: f64,
    height: f64,
    slope: f64,
    tol: GeometryTolerance,
) -> Result<Solid> {
    ellipse_cap_fixture(radius, height, slope, tol, None, &[])
}
/// Closed half-cylinder fixture with a half-ellipse/straight-diameter cap.
pub fn half_ellipse_planar_demo_solid(
    radius: f64,
    height: f64,
    slope: f64,
    tol: GeometryTolerance,
) -> Result<Solid> {
    ellipse_segment_planar_demo_solid(radius, height, slope, PI, tol)
}
/// Closed circular-segment fixture capped by an oblique ellipse arc and chord.
/// Positive sweeps up to pi are supported; the profile is symmetric about +Y.
/// The cutting plane must remain strictly between both source rims.
pub fn ellipse_segment_planar_demo_solid(
    radius: f64,
    height: f64,
    slope: f64,
    sweep: f64,
    tol: GeometryTolerance,
) -> Result<Solid> {
    if !sweep.is_finite() || sweep <= 0. || sweep > PI {
        return Err(Error::InvalidInput(
            "ellipse segment fixture requires sweep in (0, pi]",
        ));
    }
    ellipse_cap_fixture(radius, height, slope, tol, Some(sweep), &[])
}
/// Closed tube fixture with a complete ellipse outer boundary and concentric hole.
/// The transverse plane must stay strictly between both source rims.
pub fn ellipse_annulus_planar_demo_solid(
    radius: f64,
    inner_radius: f64,
    height: f64,
    slope: f64,
    tol: GeometryTolerance,
) -> Result<Solid> {
    ellipse_eccentric_planar_demo_solid(radius, inner_radius, height, slope, [0., 0.], tol)
}
/// Closed eccentric tube fixture: the hole center is in the source XY profile.
/// Both circular rims must stay strictly across the transverse cutting plane.
pub fn ellipse_eccentric_planar_demo_solid(
    radius: f64,
    inner_radius: f64,
    height: f64,
    slope: f64,
    center: [f64; 2],
    tol: GeometryTolerance,
) -> Result<Solid> {
    ellipse_multi_hole_planar_demo_solid(
        radius,
        height,
        slope,
        &[EllipseCapHole {
            radius: inner_radius,
            center,
        }],
        tol,
    )
}
/// Source XY circular hole; its oblique section becomes a homothetic ellipse.
#[derive(Clone, Copy, Debug)]
pub struct EllipseCapHole {
    pub radius: f64,
    pub center: [f64; 2],
}
/// Exact oblique cap fixture with up to sixteen disjoint homothetic ellipse holes.
pub fn ellipse_multi_hole_planar_demo_solid(
    radius: f64,
    height: f64,
    slope: f64,
    holes: &[EllipseCapHole],
    tol: GeometryTolerance,
) -> Result<Solid> {
    if holes.len() > 16 {
        return Err(Error::Unsupported(
            "ellipse cap fixture supports at most sixteen holes",
        ));
    }
    for hole in holes {
        if !hole.radius.is_finite()
            || hole.radius <= tol.linear()
            || !radius.is_finite()
            || hole.center.iter().any(|x| !x.is_finite())
            || radius - hole.radius - norm(hole.center) <= 10. * tol.linear()
        {
            return Err(Error::InvalidInput(
                "ellipse holes require resolved radii, finite centers and strict containment",
            ));
        }
    }
    ellipse_cap_fixture(radius, height, slope, tol, None, holes)
}

fn ellipse_cap_fixture(
    radius: f64,
    height: f64,
    slope: f64,
    tol: GeometryTolerance,
    segment_sweep: Option<f64>,
    holes: &[EllipseCapHole],
) -> Result<Solid> {
    if !radius.is_finite()
        || !height.is_finite()
        || !slope.is_finite()
        || radius <= tol.linear()
        || height <= tol.linear()
    {
        return Err(Error::InvalidInput(
            "ellipse cap demo requires resolved positive dimensions and finite slope",
        ));
    }
    let source = extrude_arc_line_region_along(
        &ArcLineRegion {
            origin: Point3::new(0., 0., -height / 2.),
            holes: holes
                .iter()
                .map(|hole| {
                    [0., PI]
                        .into_iter()
                        .map(|start_angle| PlanarSegment::Arc {
                            center: hole.center,
                            radius: hole.radius,
                            start_angle,
                            sweep: PI,
                        })
                        .collect()
                })
                .collect(),
            outer: if let Some(sweep) = segment_sweep {
                vec![
                    PlanarSegment::Arc {
                        center: [0., 0.],
                        radius,
                        start_angle: (PI - sweep) / 2.,
                        sweep,
                    },
                    PlanarSegment::Line {
                        a: [-radius * (sweep / 2.).sin(), radius * chord_plane(sweep).1],
                        b: [radius * (sweep / 2.).sin(), radius * chord_plane(sweep).1],
                    },
                ]
            } else {
                [0., PI]
                    .map(|start_angle| PlanarSegment::Arc {
                        center: [0., 0.],
                        radius,
                        start_angle,
                        sweep: PI,
                    })
                    .to_vec()
            },
        },
        Vec3::new(0., 0., height),
        tol.absolute(),
    )?;
    let cut = subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(0., 0., 0.),
        Vec3::new(slope, 0., 1.),
        tol,
    )?;
    let mut solid = cut.solid;
    let sides: Vec<_> = cut
        .split_faces
        .iter()
        .filter(|pair| pair[0] >= 2)
        .map(|pair| {
            let index = pair
                .iter()
                .find(|&&fi| {
                    solid.shell.faces[fi]
                        .wires
                        .iter()
                        .flat_map(|w| &w.coedges)
                        .any(|c| {
                            let edge = &solid.edges[c.edge];
                            edge.vertices.iter().any(|&index| {
                                Vec3::new(slope, 0., 1.).dot(solid.vertices[index].point)
                                    < -tol.linear()
                            })
                        })
                })
                .copied()
                .ok_or(Error::InvalidTopology(
                    "ellipse fixture has no resolved lower wall",
                ))?;
            Ok(solid.shell.faces[index].clone())
        })
        .collect::<Result<_>>()?;
    if sides.len() != 2 * (1 + holes.len()) {
        return Err(Error::InvalidTopology(
            "ellipse fixture has an unexpected side wall count",
        ));
    }
    let boundaries: Vec<_> = sides
        .iter()
        .map(|f| {
            f.wires[0]
                .coedges
                .iter()
                .find(|c| cut.section_edges.contains(&c.edge))
                .cloned()
                .ok_or(Error::InvalidTopology(
                    "section edge missing from fixture side",
                ))
        })
        .collect::<Result<_>>()?;
    let edges: Vec<_> = boundaries.iter().map(|c| c.edge).collect();
    let Curve::EllipseArc { center, cosine, .. } = solid.edges[edges[0]].curve else {
        return Err(Error::InvalidTopology("ellipse section required"));
    };
    let u = cosine.normalized()?;
    let normal = Vec3::new(slope, 0., 1.).normalized()?;
    let v = normal.cross(u).normalized()?;
    let surface = Surface::Plane {
        origin: center,
        u,
        v,
    };
    let coedges: Vec<_> = edges
        .iter()
        .enumerate()
        .map(|(index, &edge)| {
            let pcurve = match solid.edges[edge].curve {
                Curve::EllipseArc {
                    center: c,
                    cosine: a,
                    sine: b,
                    sweep,
                } => PCurve::EllipseArc {
                    center: surface.parameters(c),
                    cosine: [a.dot(u), a.dot(v)],
                    sine: [b.dot(u), b.dot(v)],
                    sweep,
                },
                Curve::Line { a, b } => {
                    let first = surface.parameters(a);
                    let last = surface.parameters(b);
                    PCurve::Affine {
                        origin: first,
                        direction: sub(last, first),
                    }
                }
                _ => unreachable!(),
            };
            Coedge {
                edge,
                // Shared edge uses oppose after applying each face's orientation;
                // inward-facing tube walls therefore keep the coedge direction.
                forward: if sides[index].orientation == 1 {
                    !boundaries[index].forward
                } else {
                    boundaries[index].forward
                },
                pcurve,
            }
        })
        .collect();
    let mut faces = vec![solid.shell.faces[0].clone()];
    faces.extend(sides);
    faces.push(Face {
        surface,
        orientation: 1,
        wires: coedges
            .chunks(2)
            .map(|coedges| Wire {
                coedges: coedges.to_vec(),
            })
            .collect(),
    });
    solid.shell.faces = faces;
    let mut edge_map = vec![usize::MAX; solid.edges.len()];
    let mut used = Vec::new();
    for c in solid
        .shell
        .faces
        .iter_mut()
        .flat_map(|f| &mut f.wires)
        .flat_map(|w| &mut w.coedges)
    {
        if edge_map[c.edge] == usize::MAX {
            edge_map[c.edge] = used.len();
            used.push(solid.edges[c.edge].clone());
        }
        c.edge = edge_map[c.edge];
    }
    solid.edges = used;
    let mut vertex_map = vec![usize::MAX; solid.vertices.len()];
    let mut vertices = Vec::new();
    for edge in &mut solid.edges {
        for index in &mut edge.vertices {
            if vertex_map[*index] == usize::MAX {
                vertex_map[*index] = vertices.len();
                vertices.push(solid.vertices[*index].clone());
            }
            *index = vertex_map[*index];
        }
    }
    solid.vertices = vertices;
    solid.validate(tol.absolute())?;
    Ok(solid)
}
/// Native/WASM full-ellipse planar clipping fixture. Offset is along plane V;
/// placement is a rigid rotation in radians. Tangencies and trim vertices fail.
pub fn ellipse_planar_demo_json(offset: f64, placement: f64) -> Result<String> {
    if !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "ellipse planar demo requires finite values",
        ));
    }
    let t = GeometryTolerance::default();
    let solid = ellipse_planar_demo_solid(24., 24., 0.25, t)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        t.absolute(),
    )?;
    let face = 3;
    let Surface::Plane { origin, u, v } = solid.shell.faces[face].surface else {
        unreachable!()
    };
    let anchor = origin - u * 34. + v * offset;
    let direction = u * 2.;
    ellipse_planar_query_json(&solid, 3, anchor, direction)
}
/// Mixed half-ellipse/diameter fixture: modes 0 across arc, 1 diameter-to-arc,
/// 2 reversed diameter crossing. Offset is the transverse in-plane coordinate.
pub fn half_ellipse_planar_demo_json(mode: u32, offset: f64, placement: f64) -> Result<String> {
    if mode > 2 || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "half ellipse demo requires mode 0..2 and finite values",
        ));
    }
    let t = GeometryTolerance::default();
    let solid = half_ellipse_planar_demo_solid(24., 24., 0.25, t)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        t.absolute(),
    )?;
    let Surface::Plane { origin, u, v } = solid.shell.faces[3].surface else {
        unreachable!()
    };
    let (a, d) = match mode {
        0 => (origin - u * 34. + v * offset, u * 2.),
        1 => (origin + u * offset - v * 34., v * 2.),
        _ => (origin + u * offset + v * 34., v * (-2.)),
    };
    ellipse_planar_query_json(&solid, 3, a, d)
}
/// Native/WASM segment fixture; query axes are physical X/Z and Y in the cap.
pub fn ellipse_segment_planar_demo_json(
    sweep: f64,
    mode: u32,
    offset: f64,
    placement: f64,
) -> Result<String> {
    if mode > 2 || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "ellipse segment demo requires mode 0..2 and finite values",
        ));
    }
    let t = GeometryTolerance::default();
    let transform = Transform::rotation(Vec3::new(1., 2., 3.), placement)?;
    let solid = ellipse_segment_planar_demo_solid(24., 24., 0.25, sweep, t)?
        .transformed(transform, t.absolute())?;
    let u = Vec3::new(1., 0., -0.25).normalized()?;
    let v = Vec3::new(0., 1., 0.);
    let (a, d) = match mode {
        0 => (u * (-34.) + v * offset, u * 2.),
        1 => (u * offset - v * 34., v * 2.),
        _ => (u * offset + v * 34., v * (-2.)),
    };
    ellipse_planar_query_json(&solid, 3, transform.point(a), transform.vector(d))
}
/// Native/WASM annular ellipse cap: offset along V, rigid placement in radians.
pub fn ellipse_annulus_planar_demo_json(offset: f64, placement: f64) -> Result<String> {
    if !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "ellipse annulus demo requires finite values",
        ));
    }
    let t = GeometryTolerance::default();
    let solid = ellipse_annulus_planar_demo_solid(24., 12., 24., 0.25, t)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        t.absolute(),
    )?;
    let Surface::Plane { origin, u, v } = solid.shell.faces[5].surface else {
        unreachable!()
    };
    ellipse_planar_query_json(&solid, 5, origin - u * 34. + v * offset, u * 2.)
}
/// Eccentric hole center X and line V offset in mm; placement in radians.
pub fn ellipse_eccentric_planar_demo_json(
    center_x: f64,
    offset: f64,
    placement: f64,
) -> Result<String> {
    if !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "eccentric ellipse demo requires finite query values",
        ));
    }
    let t = GeometryTolerance::default();
    let solid = ellipse_eccentric_planar_demo_solid(24., 8., 24., 0.25, [center_x, 0.], t)?
        .transformed(
            Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
            t.absolute(),
        )?;
    let Surface::Plane { origin, u, v } = solid.shell.faces[5].surface else {
        unreachable!()
    };
    ellipse_planar_query_json(&solid, 5, origin - u * 34. + v * offset, u * 2.)
}
/// Two differently sized holes at X=-spread and X=+spread; line V offset in mm.
pub fn ellipse_multi_hole_planar_demo_json(
    spread: f64,
    offset: f64,
    placement: f64,
) -> Result<String> {
    if !spread.is_finite() || spread <= 0. || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "multiple ellipse hole demo requires positive spread and finite values",
        ));
    }
    let t = GeometryTolerance::default();
    let holes = [
        EllipseCapHole {
            radius: 5.,
            center: [-spread, 0.],
        },
        EllipseCapHole {
            radius: 6.,
            center: [spread, 0.],
        },
    ];
    let solid = ellipse_multi_hole_planar_demo_solid(24., 24., 0.25, &holes, t)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        t.absolute(),
    )?;
    let face = 7;
    let Surface::Plane { origin, u, v } = solid.shell.faces[face].surface else {
        unreachable!()
    };
    ellipse_planar_query_json(&solid, face, origin - u * 34. + v * offset, u * 2.)
}
pub(crate) fn ellipse_planar_query_json(
    solid: &Solid,
    face: usize,
    anchor: Point3,
    direction: Vec3,
) -> Result<String> {
    let t = GeometryTolerance::default();
    let Surface::Plane { u, v, .. } = solid.shell.faces[face].surface else {
        unreachable!()
    };
    let result = clip_line_to_planar_face(solid, face, anchor, direction, t)?;
    let xyz = |p: Vec3| format!("[{},{},{}]", p.x, p.y, p.z);
    let intervals: Vec<_> = result
        .intervals
        .iter()
        .map(|x| format!("[{},{}]", x.parameter_range[0], x.parameter_range[1]))
        .collect();
    let hits:Vec<_>=result.events.iter().map(|p|format!("{{\"point\":{},\"normal\":{},\"parameter\":{},\"contact\":\"crossing\",\"boundaries\":[{{\"wire\":{},\"coedge\":{},\"edge\":{},\"edge_parameter\":{}}}]}}",xyz(p.point),xyz(u.cross(v)),p.parameter,p.wire,p.coedge,p.edge,p.edge_parameter)).collect();
    let mesh = solid.tessellate(0.05, t.absolute())?;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for (i, tri) in mesh.triangles.iter().enumerate() {
        if mesh.face_ids[i] == face {
            for &index in tri {
                positions.extend([
                    mesh.positions[index].x,
                    mesh.positions[index].y,
                    mesh.positions[index].z,
                ]);
                normals.extend([
                    mesh.normals[index].x,
                    mesh.normals[index].y,
                    mesh.normals[index].z,
                ]);
            }
        }
    }
    let flatten = |v: Vec<f64>| {
        v.into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    Ok(format!("{{\"intersection\":{{\"kind\":\"{}\",\"hits\":[{}],\"intervals\":[{}]}},\"line\":{{\"anchor\":{},\"direction\":{}}},\"face\":{},\"display_mesh\":{{\"positions\":[{}],\"normals\":[{}]}},\"mesh\":{}}}",if hits.is_empty(){"empty"}else{"points"},hits.join(","),intervals.join(","),xyz(anchor),xyz(direction),face,flatten(positions),flatten(normals),solid.mesh_json(0.05,t.absolute())?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn homothetic_nonorthogonal_hole_membership_and_domain_checks() {
        let outer = EllipseRing {
            center: [10., -2.],
            cosine: [2., 0.],
            sine: [1., 1.],
            coherence_error: 0.,
            segment_sweep: None,
        };
        let coedges = |ratio: f64, forward: bool, offset: f64| {
            [1., -1.]
                .into_iter()
                .map(|sign| Coedge {
                    edge: 0,
                    forward,
                    pcurve: PCurve::EllipseArc {
                        center: [10. + offset, -2.],
                        cosine: outer.cosine.map(|x| x * ratio * sign),
                        sine: outer.sine.map(|x| x * ratio * sign),
                        sweep: PI,
                    },
                })
                .collect()
        };
        let face = |ratio, offset| Face {
            surface: Surface::Plane {
                origin: Point3::new(0., 0., 0.),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.),
            },
            orientation: 1,
            wires: vec![
                Wire {
                    coedges: coedges(1., true, 0.),
                },
                Wire {
                    coedges: coedges(ratio, false, offset),
                },
            ],
        };
        let tolerance = Tolerance::new(1e-8).unwrap();
        let region = ring(&face(0.5, 0.), tolerance).unwrap();
        assert_eq!(
            region.location(outer.center).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(region.location([11.5, -2.]).unwrap(), PointLocation::Inside);
        assert!(region.within_boundary([11., -2.], 1e-5).unwrap());
        assert!(!region.within_boundary(outer.center, 1e-5).unwrap());
        let shifted = ring(&face(0.5, 0.6), tolerance).unwrap();
        assert_eq!(
            shifted.location([10.6, -2.]).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(shifted.location([8.7, -2.]).unwrap(), PointLocation::Inside);
        assert!(shifted.within_boundary([9.6, -2.], 1e-5).unwrap());
        let mut multiple = face(0.2, -0.6);
        multiple.wires.push(Wire {
            coedges: coedges(0.2, false, 0.6),
        });
        let region = ring(&multiple, tolerance).unwrap();
        assert_eq!(
            region.location(outer.center).unwrap(),
            PointLocation::Inside
        );
        for x in [9.4, 10.6] {
            assert_eq!(region.location([x, -2.]).unwrap(), PointLocation::Outside);
        }
        multiple.wires[2].coedges = coedges(0.6, false, 0.6);
        assert!(ring(&multiple, tolerance).is_err());
        for (ratio, offset) in [(0.5, 2.), (1., 0.), (1. - 1e-9, 0.), (1.1, 0.)] {
            assert!(ring(&face(ratio, offset), tolerance).is_err());
        }
    }
    #[test]
    fn unequal_rotated_axes_and_conservative_circle_rejection() {
        let wire = |center, cosine: P2, sine: P2, forward| Wire {
            coedges: [1., -1.]
                .into_iter()
                .map(|sign| Coedge {
                    edge: 0,
                    forward,
                    pcurve: PCurve::EllipseArc {
                        center,
                        cosine: cosine.map(|v| v * sign),
                        sine: sine.map(|v| v * sign),
                        sweep: PI,
                    },
                })
                .collect(),
        };
        let mut face = Face {
            surface: Surface::Plane {
                origin: Point3::new(0., 0., 0.),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.),
            },
            orientation: 1,
            wires: vec![
                wire([10., -2.], [2., 0.], [1., 1.], true),
                wire([10.5, -2.], [0.4, 0.1], [-0.1, 0.2], false),
            ],
        };
        let tol = Tolerance::new(1e-8).unwrap();
        let region = ring(&face, tol).unwrap();
        assert_eq!(
            region.location([10.5, -2.]).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(region.location([10., -2.]).unwrap(), PointLocation::Inside);
        assert!(region.within_boundary([10.9, -1.9], 1e-5).unwrap());
        // Actual skinny ellipses are disjoint, but their enclosing circles
        // overlap. Analytic support lines certify their physical separation.
        face.wires = vec![
            wire([0., 0.], [3., 0.], [0., 3.], true),
            wire([0., -0.3], [0.6, 0.], [0., 0.1], false),
            wire([0., 0.3], [0.6, 0.], [0., 0.1], false),
        ];
        let region = ring(&face, tol).unwrap();
        let first = EllipseRing {
            center: [0., 0.],
            cosine: [0.6, 0.],
            sine: [0., 0.1],
            coherence_error: 0.,
            segment_sweep: None,
        };
        let second = EllipseRing {
            center: [0.4, 0.3],
            ..first.clone()
        };
        // The center direction fails; an oblique search direction succeeds.
        let n = [0.8, 0.6];
        assert!(
            dot(n, sub(second.center, first.center))
                - 2. * dot(n, first.cosine).hypot(dot(n, first.sine))
                < 0.
        );
        assert!(separated_ellipses(&first, &second, 1e-7));
        assert!(!separated_ellipses(&first, &second, 1.));
        assert_eq!(region.location([0., 0.]).unwrap(), PointLocation::Inside);
        assert_eq!(region.location([0., 0.3]).unwrap(), PointLocation::Outside);
        face.wires[2] = wire([0., -0.1], [0.6, 0.], [0., 0.1], false);
        assert!(matches!(ring(&face, tol), Err(Error::Unsupported(_))));
        face.wires[2] = wire([0., -0.1 + 1e-9], [0.6, 0.], [0., 0.1], false);
        assert!(matches!(ring(&face, tol), Err(Error::Unsupported(_))));
        face.wires[2] = wire([0., -0.2], [0.6, 0.], [0., 0.1], false);
        assert!(matches!(ring(&face, tol), Err(Error::Unsupported(_))));
    }
    #[test]
    fn minor_segment_nonorthogonal_membership_and_physical_chord_bands() {
        for sweep in [0.6, PI / 2., 2.5] {
            let ellipse = EllipseRing {
                center: [10., -2.],
                cosine: [2., 0.],
                sine: [1., 1.],
                coherence_error: 0.,
                segment_sweep: Some(sweep),
            };
            let (n, level) = chord_plane(sweep);
            let map = |q: P2| {
                std::array::from_fn(|i| {
                    ellipse.center[i] + ellipse.cosine[i] * q[0] + ellipse.sine[i] * q[1]
                })
            };
            assert_eq!(
                ellipse
                    .location(map(n.map(|x| x * (1. + level) / 2.)))
                    .unwrap(),
                PointLocation::Inside
            );
            assert_eq!(
                ellipse.location(map(n.map(|x| x * (level - 0.1)))).unwrap(),
                PointLocation::Outside
            );
            let midpoint = map(n.map(|x| x * level));
            let e = sub(ellipse.point(sweep), ellipse.point(0.));
            let normal = [-e[1] / norm(e), e[0] / norm(e)];
            let budget = 1e-5;
            assert!(ellipse.within_boundary(midpoint, budget).unwrap());
            for distance in [-2. * budget, 2. * budget] {
                assert!(!ellipse
                    .within_boundary(
                        std::array::from_fn(|i| midpoint[i] + normal[i] * distance),
                        budget
                    )
                    .unwrap());
            }
            assert!(ellipse
                .within_boundary(
                    std::array::from_fn(|i| midpoint[i] + normal[i] * budget),
                    budget
                )
                .is_err());
            assert!(!ellipse
                .within_boundary(ellipse.point(sweep + 0.2), budget)
                .unwrap());
        }
    }
    #[test]
    fn nonorthogonal_axes_inverse_distance_and_conditioning() {
        let ellipse = EllipseRing {
            center: [100., -20.],
            cosine: [2., 0.],
            sine: [1., 1.],
            coherence_error: 0.,
            segment_sweep: None,
        };
        let uv = ellipse.inverse(ellipse.point(0.37)).unwrap();
        assert!((uv[0] - 0.37f64.cos()).abs() < 1e-13);
        assert!((uv[1] - 0.37f64.sin()).abs() < 1e-13);
        assert_eq!(
            ellipse.location(ellipse.center).unwrap(),
            PointLocation::Inside
        );
        let budget = 1e-5;
        let p = ellipse.point(0.37);
        let tangent = [-2. * 0.37f64.sin() + 0.37f64.cos(), 0.37f64.cos()];
        let n = [tangent[1] / norm(tangent), -tangent[0] / norm(tangent)];
        assert!(ellipse.within_boundary(p, budget).unwrap());
        for distance in [-2. * budget, 2. * budget] {
            assert!(!ellipse
                .within_boundary([p[0] + n[0] * distance, p[1] + n[1] * distance], budget)
                .unwrap());
        }
        assert!(matches!(
            ellipse.within_boundary([p[0] + n[0] * budget, p[1] + n[1] * budget], budget),
            Err(Error::Unsupported(_))
        ));
        let bad = EllipseRing {
            center: [0., 0.],
            cosine: [1., 0.],
            sine: [1., 1e-12],
            coherence_error: 0.,
            segment_sweep: None,
        };
        assert!(bad.inverse([0., 0.]).is_err());
    }
}
