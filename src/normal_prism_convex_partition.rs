//! Analytic finite line/circle arrangements for normal prismatic partitions.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Debug)]
pub struct NormalPrismConvexPartition {
    difference: Vec<Solid>,
    intersection: Vec<Solid>,
}
impl NormalPrismConvexPartition {
    pub fn difference(&self) -> &[Solid] {
        &self.difference
    }
    pub fn intersection(&self) -> &[Solid] {
        &self.intersection
    }
    pub fn into_solids(self) -> (Vec<Solid>, Vec<Solid>) {
        (self.difference, self.intersection)
    }
}
type P2 = [f64; 2];
const DOMAIN: Error =
    Error::Unsupported("normal prism convex partition contact or arithmetic is unresolved");
fn distance(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn checked(value: f64, budget: f64) -> Result<()> {
    if value.is_finite() && value < budget {
        Ok(())
    } else {
        Err(DOMAIN)
    }
}
fn segment_length(s: PlanarSegment) -> f64 {
    match s {
        PlanarSegment::Line { a, b } => distance(a, b),
        PlanarSegment::Arc { radius, sweep, .. } => radius * sweep.abs(),
    }
}
fn area(ring: &[PlanarSegment]) -> Result<f64> {
    let anchor = ring.first().ok_or(DOMAIN)?.evaluate(0.);
    let mut sum = 0.;
    let mut correction = 0.;
    let mut absolute = 0.;
    for s in ring {
        let terms = match *s {
            PlanarSegment::Line { a, b } => {
                let a = sub(a, anchor);
                let b = sub(b, anchor);
                [a[0] * b[1] * 0.5, -a[1] * b[0] * 0.5, 0.]
            }
            PlanarSegment::Arc {
                center: c,
                radius: r,
                start_angle: a,
                sweep: w,
            } => {
                let c = sub(c, anchor);
                let half_sine = (w * 0.5).sin();
                let middle = a + w * 0.5;
                [
                    r * c[0] * middle.cos() * half_sine,
                    r * c[1] * middle.sin() * half_sine,
                    0.5 * r * r * w,
                ]
            }
        };
        for value in terms {
            let adjusted = value - correction;
            let next = sum + adjusted;
            correction = (next - sum) - adjusted;
            sum = next;
            absolute += value.abs();
        }
    }
    if !sum.is_finite() || !absolute.is_finite() || sum.abs() <= 8192. * f64::EPSILON * absolute {
        return Err(DOMAIN);
    }
    Ok(sum)
}
fn orient(ring: &[PlanarSegment], positive: bool) -> Result<Vec<PlanarSegment>> {
    if (area(ring)? > 0.) == positive {
        Ok(ring.to_vec())
    } else {
        Ok(ring.iter().rev().map(|s| s.reversed()).collect())
    }
}
fn arc_fraction(s: PlanarSegment, p: P2) -> Option<f64> {
    let PlanarSegment::Arc {
        center,
        radius: _,
        start_angle,
        sweep,
    } = s
    else {
        return None;
    };
    let delta = ((p[1] - center[1]).atan2(p[0] - center[0]) - start_angle) * sweep.signum();
    let angle = delta.rem_euclid(TAU);
    if angle <= sweep.abs() {
        Some(angle / sweep.abs())
    } else {
        None
    }
}
fn endpoint_clearance(a: PlanarSegment, b: PlanarSegment, margin: f64) -> Result<()> {
    for t in [0., 1.] {
        if crate::mixed::point_distance(a.evaluate(t), b)? <= margin
            || crate::mixed::point_distance(b.evaluate(t), a)? <= margin
        {
            return Err(DOMAIN);
        }
    }
    Ok(())
}
fn line_intersections(
    source: PlanarSegment,
    tool: PlanarSegment,
    margin: f64,
    reserve: f64,
    angular: f64,
) -> Result<Vec<(f64, f64, P2)>> {
    let PlanarSegment::Line { a, b } = tool else {
        return Err(DOMAIN);
    };
    endpoint_clearance(source, tool, margin)?;
    let delta = sub(b, a);
    let len = delta[0].hypot(delta[1]);
    let unit = [delta[0] / len, delta[1] / len];
    let mut result = Vec::new();
    match source {
        PlanarSegment::Line { a: c, b: d } => {
            let sd = sub(d, c);
            let sl = sd[0].hypot(sd[1]);
            let su = [sd[0] / sl, sd[1] / sl];
            let denominator = cross(su, unit);
            if denominator.abs() <= 64. * f64::EPSILON {
                return Ok(result);
            }
            let ca = sub(a, c);
            let t = cross(ca, unit) / denominator / sl;
            let u = cross(ca, su) / denominator / len;
            if t > 0. && t < 1. && u > 0. && u < 1. {
                if denominator.abs() <= angular.sin() || denominator.abs() * sl.min(len) <= margin {
                    return Err(DOMAIN);
                }
                let p = source.evaluate(t);
                checked(distance(p, tool.evaluate(u)), reserve)?;
                result.push((t, u, p));
            }
        }
        PlanarSegment::Arc { center, radius, .. } => {
            let ca = sub(center, a);
            let projection = dot(ca, unit);
            let perpendicular = cross(ca, unit);
            let scale = radius.max(perpendicular.abs());
            let disc = (radius / scale).powi(2) - (perpendicular / scale).powi(2);
            let foot = [a[0] + unit[0] * projection, a[1] + unit[1] * projection];
            // A near tangent only matters on BOTH finite restrictions.
            if (radius - perpendicular.abs()).abs() <= margin {
                for sign in [-1., 1.] {
                    let p = [
                        center[0] - unit[1] * radius * sign,
                        center[1] + unit[0] * radius * sign,
                    ];
                    if arc_fraction(source, p).is_some()
                        && crate::mixed::point_distance(p, tool)? <= margin
                    {
                        return Err(DOMAIN);
                    }
                }
            }
            if !disc.is_finite() {
                return Err(DOMAIN);
            }
            if disc <= 0. {
                return Ok(result);
            }
            let root = disc.sqrt() * scale;
            for sign in [-1., 1.] {
                let along = projection + sign * root;
                let u = along / len;
                if u <= 0. || u >= 1. {
                    continue;
                }
                let p = [
                    foot[0] + unit[0] * sign * root,
                    foot[1] + unit[1] * sign * root,
                ];
                if let Some(t) = arc_fraction(source, p) {
                    if t <= 0. || t >= 1. {
                        return Err(DOMAIN);
                    }
                    let exact = source.evaluate(t);
                    checked(
                        distance(exact, p) + distance(exact, tool.evaluate(u)),
                        reserve,
                    )?;
                    if root <= margin
                        || t.min(1. - t) * segment_length(source) <= margin
                        || u.min(1. - u) * len <= margin
                    {
                        return Err(DOMAIN);
                    }
                    result.push((t, u, exact));
                }
            }
        }
    }
    Ok(result)
}
fn arc_intersections(
    a: PlanarSegment,
    b: PlanarSegment,
    margin: f64,
    reserve: f64,
    angular: f64,
    arithmetic: f64,
) -> Result<Vec<(f64, f64, P2)>> {
    match (a, b) {
        (_, PlanarSegment::Line { .. }) => {
            let events = line_intersections(a, b, margin, reserve, angular)?;
            if let PlanarSegment::Arc { center, radius, .. } = a {
                let PlanarSegment::Line { a: p, b: q } = b else {
                    unreachable!()
                };
                let v = sub(q, p);
                let length = v[0].hypot(v[1]);
                for (_, _, point) in &events {
                    let radial = sub(*point, center);
                    let sine = dot(radial, v).abs() / (radius * length);
                    checked((arithmetic + 512. * f64::EPSILON * radius) / sine, reserve)?;
                    if sine <= angular.sin() {
                        return Err(DOMAIN);
                    }
                }
            }
            Ok(events)
        }
        (PlanarSegment::Line { .. }, PlanarSegment::Arc { .. }) => Ok(arc_intersections(
            b, a, margin, reserve, angular, arithmetic,
        )?
        .into_iter()
        .map(|(t, u, p)| (u, t, p))
        .collect()),
        (
            PlanarSegment::Arc {
                center: c0,
                radius: r0,
                ..
            },
            PlanarSegment::Arc {
                center: c1,
                radius: r1,
                ..
            },
        ) => {
            endpoint_clearance(a, b, margin)?;
            let delta = sub(c1, c0);
            let d = delta[0].hypot(delta[1]);
            if !d.is_finite() {
                return Err(DOMAIN);
            }
            if d <= margin {
                if (r0 - r1).abs() > d + margin {
                    return Ok(Vec::new());
                }
                return Err(DOMAIN);
            }
            let unit = [delta[0] / d, delta[1] / d];
            let outside = d - r0 - r1;
            let inside = d - (r0 - r1).abs();
            if outside > margin || inside < -margin {
                return Ok(Vec::new());
            }
            if outside.abs() <= margin || inside.abs() <= margin {
                for s in [-1., 1.] {
                    for t in [-1., 1.] {
                        let p = [c0[0] + s * r0 * unit[0], c0[1] + s * r0 * unit[1]];
                        let q = [c1[0] + t * r1 * unit[0], c1[1] + t * r1 * unit[1]];
                        if distance(p, q) <= margin
                            && arc_fraction(a, p).is_some()
                            && arc_fraction(b, q).is_some()
                        {
                            return Err(DOMAIN);
                        }
                    }
                }
                // No contact on these finite arcs; nonpositive support factors
                // cannot produce a transverse finite event.
                if outside >= 0. || inside <= 0. {
                    return Ok(Vec::new());
                }
            }
            let scale = d.max(r0).max(r1);
            let nd = d / scale;
            let n0 = r0 / scale;
            let n1 = r1 / scale;
            let factors = [n0 + n1 - nd, n0 + n1 + nd, nd - n0 + n1, nd + n0 - n1];
            if factors.iter().any(|f| !f.is_finite() || *f <= 0.) {
                return Ok(Vec::new());
            }
            let h2 = factors.iter().product::<f64>() / (4. * nd * nd);
            let along = (nd * nd + (n0 - n1) * (n0 + n1)) / (2. * nd) * scale;
            let h = h2.sqrt() * scale;
            if !h.is_finite() || !along.is_finite() {
                return Err(DOMAIN);
            }
            let sine = nd * (h / scale) / (n0 * n1);
            let mut result = Vec::new();
            for sign in [-1., 1.] {
                let p = [
                    c0[0] + unit[0] * along - unit[1] * h * sign,
                    c0[1] + unit[1] * along + unit[0] * h * sign,
                ];
                if let (Some(t), Some(u)) = (arc_fraction(a, p), arc_fraction(b, p)) {
                    if t.min(1. - t) * segment_length(a) <= margin
                        || u.min(1. - u) * segment_length(b) <= margin
                        || sine <= angular.sin()
                    {
                        return Err(DOMAIN);
                    }
                    checked((arithmetic + 4096. * f64::EPSILON * scale) / sine, reserve)?;
                    let exact = a.evaluate(t);
                    checked(
                        distance(exact, p)
                            + distance(exact, b.evaluate(u))
                            + (distance(p, c0) - r0).abs()
                            + (distance(p, c1) - r1).abs(),
                        reserve,
                    )?;
                    result.push((t, u, exact));
                }
            }
            Ok(result)
        }
    }
}
#[derive(Clone)]
struct InputSegment {
    geometry: PlanarSegment,
    breaks: Vec<(f64, usize)>,
}
#[derive(Clone, Copy)]
struct Fragment {
    geometry: PlanarSegment,
    start: usize,
    end: usize,
}
fn inputs(rings: &[Vec<PlanarSegment>], nodes: &mut Vec<P2>) -> Vec<InputSegment> {
    let mut result = Vec::new();
    for ring in rings {
        let first = nodes.len();
        nodes.extend(ring.iter().map(|s| s.evaluate(0.)));
        for (i, s) in ring.iter().enumerate() {
            result.push(InputSegment {
                geometry: *s,
                breaks: vec![(0., first + i), (1., first + (i + 1) % ring.len())],
            });
        }
    }
    result
}
fn fragments(
    inputs: &mut [InputSegment],
    nodes: &[P2],
    margin: f64,
    reserve: f64,
) -> Result<Vec<Fragment>> {
    let mut result = Vec::new();
    for input in inputs {
        input.breaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in input.breaks.windows(2) {
            let (t0, start) = pair[0];
            let (t1, end) = pair[1];
            if start == end
                || !t0.is_finite()
                || !t1.is_finite()
                || (t1 - t0) * segment_length(input.geometry) <= margin
            {
                return Err(DOMAIN);
            }
            checked(
                distance(nodes[start], input.geometry.evaluate(t0))
                    + distance(nodes[end], input.geometry.evaluate(t1)),
                reserve,
            )?;
            let geometry = match input.geometry {
                PlanarSegment::Line { .. } => PlanarSegment::Line {
                    a: nodes[start],
                    b: nodes[end],
                },
                PlanarSegment::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } => {
                    let phase = start_angle + sweep * t0;
                    let wrapped = phase.rem_euclid(TAU);
                    checked(
                        radius
                            * ((phase.cos() - wrapped.cos()).abs()
                                + (phase.sin() - wrapped.sin()).abs()),
                        reserve,
                    )?;
                    PlanarSegment::Arc {
                        center,
                        radius,
                        start_angle: wrapped,
                        sweep: sweep * (t1 - t0),
                    }
                }
            };
            result.push(Fragment {
                geometry,
                start,
                end,
            });
        }
    }
    Ok(result)
}
fn material_location(p: P2, rings: &[Vec<PlanarSegment>], tol: Tolerance) -> Result<PointLocation> {
    let outer = crate::mixed::point_location(p, &rings[0], tol)?;
    if outer != PointLocation::Inside {
        return Ok(outer);
    }
    for ring in &rings[1..] {
        match crate::mixed::point_location(p, ring, tol)? {
            PointLocation::Boundary => return Ok(PointLocation::Boundary),
            PointLocation::Inside => return Ok(PointLocation::Outside),
            PointLocation::Outside => {}
        }
    }
    Ok(PointLocation::Inside)
}
fn cycles(selected: &[Fragment]) -> Result<Vec<Vec<PlanarSegment>>> {
    if selected.len() > 128 {
        return Err(Error::Unsupported(
            "partition result profile limit exceeded",
        ));
    }
    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for (i, f) in selected.iter().enumerate() {
        if outgoing.insert(f.start, i).is_some() || !incoming.insert(f.end) {
            return Err(DOMAIN);
        }
    }
    if outgoing.len() != incoming.len() || outgoing.keys().any(|i| !incoming.contains(i)) {
        return Err(DOMAIN);
    }
    let mut visited = vec![false; selected.len()];
    let mut result = Vec::new();
    for first in 0..selected.len() {
        if visited[first] {
            continue;
        }
        let mut index = first;
        let mut ring = Vec::new();
        loop {
            if visited[index] {
                return Err(DOMAIN);
            }
            visited[index] = true;
            let f = selected[index];
            ring.push(f.geometry);
            index = *outgoing.get(&f.end).ok_or(DOMAIN)?;
            if index == first {
                break;
            }
            if ring.len() > 128 {
                return Err(DOMAIN);
            }
        }
        area(&ring)?;
        result.push(ring);
    }
    Ok(result)
}
fn extrude_cycles(
    rings: Vec<Vec<PlanarSegment>>,
    frame: Frame3,
    height: f64,
    axis: Vec3,
    tol: Tolerance,
    effective: Tolerance,
) -> Result<Vec<Solid>> {
    let mut outers = Vec::new();
    let mut holes = Vec::new();
    for ring in rings {
        if area(&ring)? > 0. {
            outers.push(ring);
        } else {
            holes.push(ring);
        }
    }
    let mut assignments = vec![Vec::new(); outers.len()];
    for hole in holes {
        let point = hole[0].evaluate(0.);
        let mut parent = None;
        for (i, outer) in outers.iter().enumerate() {
            if crate::mixed::point_location(point, outer, effective)? == PointLocation::Inside
                && parent.replace(i).is_some()
            {
                return Err(DOMAIN);
            }
        }
        assignments[parent.ok_or(DOMAIN)?].push(hole);
    }
    let mut result = Vec::new();
    for (outer, holes) in outers.into_iter().zip(assignments) {
        let mut material_area = area(&outer)?;
        let mut absolute_area = material_area.abs();
        let mut area_correction = 0.;
        for hole in &holes {
            let value = area(hole)?;
            absolute_area += value.abs();
            let adjusted = value - area_correction;
            let next = material_area + adjusted;
            area_correction = (next - material_area) - adjusted;
            material_area = next;
        }
        if !material_area.is_finite()
            || !absolute_area.is_finite()
            || material_area <= 8192. * f64::EPSILON * absolute_area
        {
            return Err(DOMAIN);
        }
        let region = ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes,
        };
        let mut loops = vec![region.outer.clone()];
        loops.extend(region.holes.iter().cloned());
        crate::mixed::validate_mixed_region_trim(&loops, effective)?;
        let body = extrude_arc_line_region_in_frame(&region, frame.axes()[2] * height, frame, tol)?;
        body.validate(tol)?;
        crate::arc_line_prism_validation::recognize_validated_normal_prism(&body, axis, tol)?;
        let volume = body.volume()?;
        let expected = material_area * height;
        if !volume.is_finite()
            || volume <= 0.
            || !expected.is_finite()
            || expected < f64::MIN_POSITIVE
            || (volume - expected).abs() > 32768. * f64::EPSILON * volume.max(expected)
        {
            return Err(DOMAIN);
        }
        result.push(body);
    }
    Ok(result)
}

/// Partition certified normal line/quarter-arc stock by a convex all-line normal
/// tool with the same physical cap interval. Proper finite boundary crossings,
/// strict containment and separation are admitted; contact, tangent, vertex,
/// coincident or unresolved arrangements are explicitly unsupported. Every
/// returned component is an exact closed B-rep, with no mesh Boolean operation.
pub fn partition_normal_prism_by_convex_tool(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismConvexPartition> {
    let (difference, intersection, _) = partition_engine(source, tool, axis, tolerance, false)?;
    Ok(NormalPrismConvexPartition {
        difference,
        intersection,
    })
}
pub(crate) type BooleanBodies = (Vec<Solid>, Vec<Solid>, Vec<Solid>);
pub(crate) fn partition_engine(
    source: &Solid,
    tool: &Solid,
    axis: Vec3,
    tolerance: GeometryTolerance,
    curved_tool: bool,
) -> Result<BooleanBodies> {
    let tol = tolerance.absolute();
    source.validate(tol)?;
    tool.validate(tol)?;
    let stock =
        crate::arc_line_prism_validation::recognize_validated_normal_prism(source, axis, tol)?;
    let cutter =
        crate::arc_line_prism_validation::recognize_validated_normal_prism(tool, axis, tol)?;
    if !cutter.region.holes.is_empty()
        || (!curved_tool
            && tool
                .edges
                .iter()
                .any(|e| !matches!(e.curve, Curve::Line { .. })))
    {
        return Err(Error::Unsupported(
            "partition tool must be a convex all-line prism without holes",
        ));
    }
    if !curved_tool {
        crate::booleans::convex_planes(tool, tolerance)?;
    }
    if curved_tool
        && cutter.region.outer.iter().any(
            |s| matches!(s,PlanarSegment::Arc{sweep,..}if sweep.abs()>FRAC_PI_2+64.*f64::EPSILON),
        )
    {
        return Err(Error::Unsupported(
            "arc-line Boolean tool requires quarter or smaller arcs without holes",
        ));
    }
    let mut stock_rings = vec![orient(&stock.region.outer, true)?];
    for hole in &stock.region.holes {
        stock_rings.push(orient(hole, false)?);
    }
    if stock.region.holes.len() > 16
        || stock_rings.iter().flatten().any(
            |s| matches!(s,PlanarSegment::Arc{sweep,..}if sweep.abs()>FRAC_PI_2+64.*f64::EPSILON),
        )
    {
        return Err(Error::Unsupported(
            "partition source requires at most sixteen holes and quarter or smaller arcs",
        ));
    }
    let total = stock_rings.iter().map(Vec::len).sum::<usize>() + cutter.region.outer.len();
    if total > 128 {
        return Err(Error::Unsupported("partition input profile limit exceeded"));
    }
    let scale = length(source.bounds().max - source.bounds().min)
        .max(length(tool.bounds().max - tool.bounds().min));
    let margin = 10. * tolerance.length_at_scale(scale)?;
    let reserve = tol.linear / 64.;
    let mut world = scale;
    for body in [source, tool] {
        for vertex in &body.vertices {
            let p = vertex.point;
            world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
        for face in &body.shell.faces {
            let p = match face.surface {
                Surface::Plane { origin, .. } => origin,
                Surface::FramedCylinder { frame, .. } => frame.origin(),
                Surface::Cylinder { center, .. } => center,
                _ => return Err(DOMAIN),
            };
            world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
        for edge in &body.edges {
            if let Curve::Arc { frame, radius, .. } = edge.curve {
                let p = frame.origin();
                world = world
                    .max(p.x.abs())
                    .max(p.y.abs())
                    .max(p.z.abs())
                    .max(radius);
            }
        }
    }
    let arithmetic = 4096. * f64::EPSILON * world;
    checked(arithmetic, tol.linear / 256.)?;
    let reserve = reserve - arithmetic;
    if reserve <= 0. || !margin.is_finite() {
        return Err(DOMAIN);
    }
    let cb = stock.frame.local_point(cutter.frame.origin());
    let ct = stock
        .frame
        .local_point(cutter.frame.point(Vec3::new(0., 0., cutter.height)));
    checked(
        cb.z.abs().min((cb.z - stock.height).abs())
            + ct.z.abs().min((ct.z - stock.height).abs())
            + ((cb.z - ct.z).abs() - stock.height).abs(),
        reserve,
    )?;
    checked(
        length(stock.frame.axes()[2].cross(cutter.frame.axes()[2])) * scale,
        reserve,
    )?;
    let mut transformed = Vec::new();
    for segment in &cutter.region.outer {
        let (a, b) = match *segment {
            PlanarSegment::Line { a, b } => (a, b),
            PlanarSegment::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => {
                let actual = cutter.frame.point(Vec3::new(center[0], center[1], 0.));
                let local = stock.frame.local_point(actual);
                checked(
                    (local.z - cb.z).abs()
                        + length(actual - stock.frame.point(Vec3::new(local.x, local.y, cb.z))),
                    reserve,
                )?;
                let x = stock.frame.local_vector(cutter.frame.axes()[0]);
                let y = stock.frame.local_vector(cutter.frame.axes()[1]);
                let phase = x.y.atan2(x.x);
                let orientation = if x.x * y.y - x.y * y.x > 0. { 1. } else { -1. };
                let cosine = stock.frame.vector(Vec3::new(phase.cos(), phase.sin(), 0.));
                let sine = stock.frame.vector(Vec3::new(
                    -phase.sin() * orientation,
                    phase.cos() * orientation,
                    0.,
                ));
                checked(
                    radius
                        * (length(cosine - cutter.frame.axes()[0])
                            + length(sine - cutter.frame.axes()[1])),
                    reserve,
                )?;
                let start = phase + orientation * start_angle;
                let wrapped = start.rem_euclid(TAU);
                checked(
                    radius
                        * ((start.cos() - wrapped.cos()).abs()
                            + (start.sin() - wrapped.sin()).abs()),
                    reserve,
                )?;
                transformed.push(PlanarSegment::Arc {
                    center: [local.x, local.y],
                    radius,
                    start_angle: wrapped,
                    sweep: orientation * sweep,
                });
                continue;
            }
        };
        let mut points = Vec::new();
        for p in [a, b] {
            let actual = cutter.frame.point(Vec3::new(p[0], p[1], 0.));
            let local = stock.frame.local_point(actual);
            checked((local.z - cb.z).abs(), reserve)?;
            let represented = stock.frame.point(Vec3::new(local.x, local.y, cb.z));
            checked(length(actual - represented), reserve)?;
            points.push([local.x, local.y]);
        }
        transformed.push(PlanarSegment::Line {
            a: points[0],
            b: points[1],
        });
    }
    let tool_ring = orient(&transformed, true)?;
    crate::mixed::validate_mixed(&tool_ring, tol)?;
    let mut nodes = Vec::new();
    let mut first = inputs(&stock_rings, &mut nodes);
    let mut second = inputs(std::slice::from_ref(&tool_ring), &mut nodes);
    let mut crossings = 0;
    for a in &mut first {
        for b in &mut second {
            let events = if curved_tool {
                arc_intersections(
                    a.geometry,
                    b.geometry,
                    margin + arithmetic,
                    reserve,
                    tolerance.angular(),
                    arithmetic,
                )?
            } else {
                line_intersections(
                    a.geometry,
                    b.geometry,
                    margin + arithmetic,
                    reserve,
                    tolerance.angular(),
                )?
            };
            for (t, u, p) in events {
                if crossings >= 128 {
                    return Err(Error::Unsupported("partition crossing limit exceeded"));
                }
                // Each analytic event owns one shared ID; no proximity welding occurs.
                if curved_tool
                    && nodes[total..]
                        .iter()
                        .any(|q| distance(*q, p) <= margin + arithmetic)
                {
                    return Err(DOMAIN);
                }
                let node = nodes.len();
                nodes.push(p);
                a.breaks.push((t, node));
                b.breaks.push((u, node));
                crossings += 1;
            }
        }
    }
    let source_fragments = fragments(&mut first, &nodes, margin, reserve)?;
    let tool_fragments = fragments(&mut second, &nodes, margin, reserve)?;
    let classify_tol = Tolerance::new(margin + arithmetic)?;
    let mut difference = Vec::new();
    let mut common = Vec::new();
    let mut union = Vec::new();
    for f in source_fragments {
        match crate::mixed::point_location(f.geometry.evaluate(0.5), &tool_ring, classify_tol)? {
            PointLocation::Outside => {
                difference.push(f);
                if curved_tool {
                    union.push(f);
                }
            }
            PointLocation::Inside => common.push(f),
            PointLocation::Boundary => return Err(DOMAIN),
        }
    }
    for f in tool_fragments {
        match material_location(f.geometry.evaluate(0.5), &stock_rings, classify_tol)? {
            PointLocation::Inside => {
                common.push(f);
                difference.push(Fragment {
                    geometry: f.geometry.reversed(),
                    start: f.end,
                    end: f.start,
                });
            }
            PointLocation::Outside => {
                if curved_tool {
                    union.push(f);
                }
            }
            PointLocation::Boundary => return Err(DOMAIN),
        }
    }
    if difference.len() + common.len() + union.len() > 128 {
        return Err(Error::Unsupported(
            "partition combined result profile limit exceeded",
        ));
    }
    let effective = Tolerance::new(tolerance.length_at_scale(scale)?)?;
    let difference = extrude_cycles(
        cycles(&difference)?,
        stock.frame,
        stock.height,
        axis,
        tol,
        effective,
    )?;
    let intersection = extrude_cycles(
        cycles(&common)?,
        stock.frame,
        stock.height,
        axis,
        tol,
        effective,
    )?;
    let union = if curved_tool {
        extrude_cycles(
            cycles(&union)?,
            stock.frame,
            stock.height,
            axis,
            tol,
            effective,
        )?
    } else {
        Vec::new()
    };
    let mut sum = 0.;
    let mut correction = 0.;
    for body in difference.iter().chain(&intersection) {
        let v = body.volume()?;
        let adjusted = v - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    let expected = source.volume()?;
    checked(
        (sum - expected).abs(),
        32768. * f64::EPSILON * sum.abs().max(expected.abs()),
    )?;
    let overlap = intersection
        .iter()
        .try_fold(0., |sum, body| Ok::<_, Error>(sum + body.volume()?))?;
    if !overlap.is_finite()
        || overlap > tool.volume()? + 32768. * f64::EPSILON * tool.volume()?.abs()
    {
        return Err(DOMAIN);
    }
    if curved_tool {
        let mut volume = 0.;
        let mut correction = 0.;
        for body in &union {
            let adjusted = body.volume()? - correction;
            let next = volume + adjusted;
            correction = (next - volume) - adjusted;
            volume = next;
        }
        let expected = source.volume()? + tool.volume()? - overlap;
        checked(
            (volume - expected).abs(),
            32768. * f64::EPSILON * (source.volume()?.abs() + tool.volume()?.abs() + overlap.abs()),
        )?;
    }
    Ok((difference, intersection, union))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn circular_segment_area_guards_internal_cancellation() {
        let cap = |radius: f64, theta: f64, phase: f64| {
            let arc = PlanarSegment::Arc {
                center: [-radius * phase.cos(), -radius * phase.sin()],
                radius,
                start_angle: phase - theta,
                sweep: 2. * theta,
            };
            vec![
                arc,
                PlanarSegment::Line {
                    a: arc.evaluate(1.),
                    b: arc.evaluate(0.),
                },
            ]
        };
        let theta = 0.01_f64;
        let expected = theta - theta.sin() * theta.cos();
        for phase in [0., 0.7, 2.4] {
            assert!(matches!(
                area(&cap(1e6, 1e-6, phase)),
                Err(Error::Unsupported(_))
            ));
            assert!((area(&cap(1., theta, phase)).unwrap() - expected).abs() < 1e-15);
        }
    }
}
