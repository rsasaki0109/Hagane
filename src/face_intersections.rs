//! Transverse clipping against validated analytic planar B-rep trims.
use crate::*;
use std::f64::consts::{PI, TAU};
type P2 = [f64; 2];
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn norm(a: P2) -> f64 {
    a[0].hypot(a[1])
}
/// A proper boundary crossing; edge parameter follows the owning 3D edge,
/// independent of its coedge traversal. Line parameter uses the input direction.
#[derive(Clone, Debug)]
pub struct PlanarBoundaryEvent {
    pub parameter: f64,
    pub point: Point3,
    pub wire: usize,
    pub coedge: usize,
    pub edge: usize,
    pub edge_parameter: f64,
}
#[derive(Clone, Debug)]
pub struct PlanarClipInterval {
    pub parameter_range: [f64; 2],
    pub start: PlanarBoundaryEvent,
    pub end: PlanarBoundaryEvent,
}
#[derive(Clone, Debug)]
pub struct PlanarLineClip {
    /// All proper crossings, sorted by line parameter, including hole boundaries.
    pub events: Vec<PlanarBoundaryEvent>,
    /// Strict interior intervals, with boundary endpoints included.
    pub intervals: Vec<PlanarClipInterval>,
}
/// Finite line edge and two pcurves share normalized parameter [0,1].
#[derive(Clone, Debug)]
pub struct PlanarFaceIntersectionSegment {
    /// Interval on the unit-speed infinite plane intersection line.
    pub parameter_range: [f64; 2],
    pub curve: Curve,
    pub first: PCurve,
    pub second: PCurve,
}
#[derive(Clone, Debug)]
pub enum PlanarFacesIntersection {
    /// Supporting planes are parallel under the angular policy.
    Parallel,
    /// Empty means disjoint trimmed faces; multiple segments preserve holes.
    Segments(Vec<PlanarFaceIntersectionSegment>),
}
pub(crate) fn rings(face: &Face) -> Result<Vec<Vec<PlanarSegment>>> {
    face.wires
        .iter()
        .map(|wire| {
            let mut ring = Vec::new();
            for c in &wire.coedges {
                let s = match c.pcurve {
                    PCurve::HeightGraph { .. } => {
                        return Err(Error::Unsupported(
                            "planar trim routines do not support harmonic height graphs",
                        ))
                    }
                    PCurve::Affine { .. } => PlanarSegment::Line {
                        a: c.pcurve.evaluate(0.0),
                        b: c.pcurve.evaluate(1.0),
                    },
                    PCurve::Arc {
                        center,
                        radius,
                        start_angle,
                        sweep,
                    } => PlanarSegment::Arc {
                        center,
                        radius,
                        start_angle,
                        sweep,
                    },
                    PCurve::Circle { center, radius } => {
                        let arcs = [0.0, PI].map(|start_angle| PlanarSegment::Arc {
                            center,
                            radius,
                            start_angle,
                            sweep: PI,
                        });
                        if c.forward {
                            ring.extend(arcs);
                        } else {
                            ring.extend(arcs.into_iter().rev().map(PlanarSegment::reversed));
                        }
                        continue;
                    }
                };
                ring.push(if c.forward { s } else { s.reversed() });
            }
            Ok(ring)
        })
        .collect()
}
fn angle(x: f64, y: f64) -> f64 {
    let a = y.atan2(x).rem_euclid(TAU);
    if a >= TAU {
        0.0
    } else {
        a
    }
}
fn arc_parameter(pc: &PCurve, a: f64) -> Option<f64> {
    match *pc {
        PCurve::Circle { .. } => Some(a),
        PCurve::Arc {
            start_angle, sweep, ..
        } => {
            let mut t = (a - start_angle).rem_euclid(TAU);
            if t >= TAU - 1e-10 {
                t = 0.0;
            }
            if t <= sweep + 1e-10 {
                Some(t.min(sweep))
            } else {
                None
            }
        }
        _ => None,
    }
}
fn planar_face(solid: &Solid, index: usize) -> Result<&Face> {
    let f = solid
        .shell
        .faces
        .get(index)
        .ok_or(Error::InvalidInput("planar face index out of range"))?;
    if !matches!(f.surface, Surface::Plane { .. }) {
        return Err(Error::Unsupported("trim clipping requires a planar face"));
    }
    Ok(f)
}
/// Clips an infinite line lying in a planar face's supporting plane. Validates
/// the entire owning solid before reading topology. Proper transverse crossings
/// only: tangencies, vertex/near-vertex hits and boundary overlaps are errors.
/// Uses analytic trims, never tessellation. Direction need not be unit length.
pub fn clip_line_to_planar_face(
    solid: &Solid,
    face_index: usize,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarLineClip> {
    solid.validate(tol.absolute())?;
    clip_validated(
        solid,
        planar_face(solid, face_index)?,
        anchor,
        direction,
        tol,
    )
}
fn clip_validated(
    solid: &Solid,
    face: &Face,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarLineClip> {
    let Surface::Plane { origin, u, v } = face.surface else {
        unreachable!()
    };
    if !anchor.finite() {
        return Err(Error::InvalidInput(
            "planar clipping requires a finite anchor",
        ));
    }
    let unit = direction.normalized()?;
    let loops = rings(face)?;
    let scale = loops
        .iter()
        .flatten()
        .map(|s| match *s {
            PlanarSegment::Line { a, b } => norm(sub(a, b)),
            PlanarSegment::Arc { radius, .. } => radius,
        })
        .fold(tol.linear(), f64::max);
    let budget = tol.length_at_scale(scale)?;
    let n = u.cross(v).normalized()?;
    let offset = anchor - origin;
    if !offset.finite()
        || !offset.norm().is_finite()
        || offset.dot(n).abs() > budget.min(64.0 * f64::EPSILON * scale.max(offset.norm()))
        || unit.dot(n).abs() > 64.0 * f64::EPSILON
    {
        return Err(Error::InvalidInput(
            "clipping line must lie in the supporting plane",
        ));
    }
    let a = [offset.dot(u), offset.dot(v)];
    let d = [unit.dot(u), unit.dot(v)];
    let speed = norm(d);
    if !speed.is_finite() || (speed - 1.0).abs() > 1e-10 {
        return Err(Error::InvalidInput("unresolved planar clipping direction"));
    }
    let d = [d[0] / speed, d[1] / speed];
    let mut events: Vec<(f64, PlanarBoundaryEvent)> = Vec::new();
    for (wi, wire) in face.wires.iter().enumerate() {
        for (ci, c) in wire.coedges.iter().enumerate() {
            let edge = &solid.edges[c.edge];
            let range = edge.curve.range();
            if !matches!(c.pcurve, PCurve::Circle { .. }) {
                for t in range {
                    let p = c.pcurve.evaluate(t);
                    if cross(sub(p, a), d).abs() <= budget {
                        return Err(Error::Unsupported(
                            "planar cut passes through or near a trim vertex",
                        ));
                    }
                }
            }
            let mut roots = Vec::new();
            match c.pcurve {
                PCurve::HeightGraph { .. } => {
                    return Err(Error::Unsupported(
                        "planar clipping does not support harmonic height graphs",
                    ))
                }
                PCurve::Affine {
                    origin: p,
                    direction: e,
                } => {
                    let len = norm(e);
                    let eu = [e[0] / len, e[1] / len];
                    let den = cross(d, eu);
                    let b = [p[0] + e[0], p[1] + e[1]];
                    let sa = cross(sub(p, a), d);
                    let sb = cross(sub(b, a), d);
                    if !sa.is_finite() || !sb.is_finite() {
                        return Err(Error::InvalidInput(
                            "trim signed distance exceeds finite range",
                        ));
                    }
                    if den.abs() <= tol.angular().sin() {
                        if (sa > 0.0) != (sb > 0.0) || sa.abs().min(sb.abs()) <= budget {
                            return Err(Error::Unsupported(
                                "unresolved parallel or overlapping planar cut",
                            ));
                        }
                        continue;
                    }
                    let delta = sub(p, a);
                    let travel = cross(delta, eu) / den;
                    let t = cross(delta, d) / den / len;
                    if !travel.is_finite() || !t.is_finite() {
                        return Err(Error::InvalidInput("linear trim roots exceed finite range"));
                    }
                    if (0.0..=1.0).contains(&t) {
                        roots.push((travel, t));
                    }
                }
                PCurve::Circle { center, radius } | PCurve::Arc { center, radius, .. } => {
                    let delta = sub(center, a);
                    let along = dot(delta, d);
                    let signed = cross(d, delta);
                    if !along.is_finite() || !signed.is_finite() {
                        return Err(Error::InvalidInput(
                            "trim root projection exceeds finite range",
                        ));
                    }
                    let gap = signed.abs() - radius;
                    if gap.abs() <= budget {
                        let near = [
                            a[0] + d[0] * along - center[0],
                            a[1] + d[1] * along - center[1],
                        ];
                        if arc_parameter(&c.pcurve, angle(near[0], near[1])).is_some() {
                            return Err(Error::Unsupported(
                                "tangent or near-tangent planar trim cut",
                            ));
                        }
                        continue;
                    }
                    if gap > 0.0 {
                        continue;
                    }
                    let half = radius * (1.0 - (signed / radius).powi(2)).max(0.0).sqrt();
                    for travel in [along - half, along + half] {
                        let p = [
                            a[0] + d[0] * travel - center[0],
                            a[1] + d[1] * travel - center[1],
                        ];
                        if !travel.is_finite() || p.iter().any(|x| !x.is_finite()) {
                            return Err(Error::InvalidInput(
                                "circular trim roots exceed finite range",
                            ));
                        }
                        if let Some(t) = arc_parameter(&c.pcurve, angle(p[0], p[1])) {
                            roots.push((travel, t));
                        }
                    }
                }
            }
            for (travel, t) in roots {
                let point = anchor + unit * (travel / speed);
                let parameter = crate::intersections::line_parameter(travel / speed, direction)?;
                let uv = c.pcurve.evaluate(t);
                if !travel.is_finite()
                    || !t.is_finite()
                    || !point.finite()
                    || !tol.absolute().coincident(point, edge.curve.evaluate(t))
                    || !tol
                        .absolute()
                        .coincident(point, face.surface.evaluate(uv[0], uv[1]))
                    || !tol
                        .absolute()
                        .coincident(point, anchor + direction * parameter)
                {
                    return Err(Error::InvalidInput(
                        "planar boundary event loses edge/line/surface agreement",
                    ));
                }
                if !matches!(c.pcurve, PCurve::Circle { .. })
                    && range
                        .into_iter()
                        .any(|t| (point - edge.curve.evaluate(t)).norm() <= budget)
                {
                    return Err(Error::Unsupported(
                        "unresolved near-vertex planar trim event",
                    ));
                }
                events.push((
                    travel / speed,
                    PlanarBoundaryEvent {
                        parameter,
                        point,
                        wire: wi,
                        coedge: ci,
                        edge: c.edge,
                        edge_parameter: t,
                    },
                ));
            }
        }
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut intervals = Vec::new();
    for pair in events.windows(2) {
        let [start, end] = [&pair[0], &pair[1]];
        let width = end.0 - start.0;
        let guard = budget.max(64.0 * f64::EPSILON * start.0.abs().max(end.0.abs()));
        if !width.is_finite() || width <= guard {
            return Err(Error::Unsupported(
                "planar boundary events are unresolved or coincident",
            ));
        }
        let middle = start.0 + width * 0.5;
        let p = [
            a[0] + d[0] * (middle * speed),
            a[1] + d[1] * (middle * speed),
        ];
        let mut inside = false;
        for (i, ring) in loops.iter().enumerate() {
            let location = crate::mixed::point_location(p, ring, Tolerance::new(budget)?)?;
            if location == PointLocation::Boundary {
                return Err(Error::Unsupported(
                    "planar clipped interval is unresolved at model tolerance",
                ));
            }
            if i == 0 {
                inside = location == PointLocation::Inside;
            } else if location == PointLocation::Inside {
                inside = false;
            }
        }
        if inside {
            if end.1.parameter <= start.1.parameter {
                return Err(Error::InvalidInput(
                    "planar clip parameter interval is unrepresentable",
                ));
            }
            intervals.push(PlanarClipInterval {
                parameter_range: [start.1.parameter, end.1.parameter],
                start: start.1.clone(),
                end: end.1.clone(),
            });
        }
    }
    if !events.len().is_multiple_of(2) {
        return Err(Error::InvalidInput(
            "unresolved odd planar boundary crossing count",
        ));
    }
    Ok(PlanarLineClip {
        events: events.into_iter().map(|(_, e)| e).collect(),
        intervals,
    })
}
/// Intersects two validated planar faces and clips the shared supporting-plane
/// line against both exact trim regions. Coplanar overlays/contact-only events
/// are unsupported. Does not change topology, split faces or perform a Boolean.
pub fn intersect_planar_faces(
    first: &Solid,
    first_face: usize,
    second: &Solid,
    second_face: usize,
    tol: GeometryTolerance,
) -> Result<PlanarFacesIntersection> {
    first.validate(tol.absolute())?;
    second.validate(tol.absolute())?;
    let a = planar_face(first, first_face)?;
    let b = planar_face(second, second_face)?;
    let line = match intersect_plane_plane(&a.surface, &b.surface, tol)? {
        PlanePlaneIntersection::Parallel => return Ok(PlanarFacesIntersection::Parallel),
        PlanePlaneIntersection::Coincident => {
            return Err(Error::Unsupported(
                "coplanar trimmed face overlay is not implemented",
            ))
        }
        PlanePlaneIntersection::Line(line) => line,
    };
    let ac = clip_validated(first, a, line.origin, line.direction, tol)?;
    let bc = clip_validated(second, b, line.origin, line.direction, tol)?;
    let mut segments = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < ac.intervals.len() && j < bc.intervals.len() {
        let ar = ac.intervals[i].parameter_range;
        let br = bc.intervals[j].parameter_range;
        let lo = ar[0].max(br[0]);
        let hi = ar[1].min(br[1]);
        let budget = tol.length_at_scale((ar[1] - ar[0]).max(br[1] - br[0]))?;
        if hi - lo >= -budget && hi - lo <= budget {
            return Err(Error::Unsupported(
                "planar faces touch or their overlap interval is unresolved",
            ));
        }
        if hi > lo {
            let begin = line.evaluate(lo)?;
            let end = line.evaluate(hi)?;
            let pc = |p: &PCurve| {
                let s = p.evaluate(lo);
                let e = p.evaluate(hi);
                PCurve::Affine {
                    origin: s,
                    direction: sub(e, s),
                }
            };
            let first_pc = pc(&line.first);
            let second_pc = pc(&line.second);
            for t in [0.0, 0.5, 1.0] {
                let p = begin + (end - begin) * t;
                for (face, uv) in [(a, first_pc.evaluate(t)), (b, second_pc.evaluate(t))] {
                    if !tol
                        .absolute()
                        .coincident(p, face.surface.evaluate(uv[0], uv[1]))
                    {
                        return Err(Error::InvalidInput(
                            "finite face intersection loses shared pcurve agreement",
                        ));
                    }
                }
            }
            if (end - begin).norm() <= budget {
                return Err(Error::InvalidInput(
                    "face intersection endpoints collapse at coordinate magnitude",
                ));
            }
            segments.push(PlanarFaceIntersectionSegment {
                parameter_range: [lo, hi],
                curve: Curve::Line { a: begin, b: end },
                first: first_pc,
                second: second_pc,
            });
        }
        if ar[1] < br[1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Ok(PlanarFacesIntersection::Segments(segments))
}

/// Native/WASM demo of clipping a through-bore cap and intersecting it with a
/// vertical box face. `offset` controls both cut locations; placement is radians.
pub fn face_clipping_demo_json(offset: f64, placement: f64) -> Result<String> {
    if !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "face clipping demo requires finite values",
        ));
    }
    let tol = GeometryTolerance::default();
    let tr = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), placement)?;
    let first = subtract_through_cylinder(
        BoxSpec {
            min: Point3::new(-4.0, -3.0, 0.0),
            size: Vec3::new(8.0, 6.0, 2.0),
        },
        CylinderSpec {
            base: Point3::new(0.0, 0.0, -1.0),
            radius: 1.0,
            height: 4.0,
        },
        tol.absolute(),
    )?
    .transformed(tr, tol.absolute())?;
    let second = make_box(
        BoxSpec {
            min: Point3::new(offset, -2.0, -1.0),
            size: Vec3::new(1.0, 4.0, 2.0),
        },
        tol.absolute(),
    )?
    .transformed(tr, tol.absolute())?;
    let clipped = clip_line_to_planar_face(
        &first,
        0,
        tr.point(Point3::new(-10.0, offset, 0.0)),
        tr.vector(Vec3::new(2.0, 0.0, 0.0)),
        tol,
    )?;
    let PlanarFacesIntersection::Segments(segments) =
        intersect_planar_faces(&first, 0, &second, 5, tol)?
    else {
        return Err(Error::InvalidInput("expected intersecting demo planes"));
    };
    let xyz = |p: Vec3| format!("[{},{},{}]", p.x, p.y, p.z);
    let event = |e: &PlanarBoundaryEvent| {
        format!("{{\"parameter\":{},\"point\":{},\"wire\":{},\"coedge\":{},\"edge\":{},\"edge_parameter\":{}}}",e.parameter,xyz(e.point),e.wire,e.coedge,e.edge,e.edge_parameter)
    };
    let events: Vec<_> = clipped.events.iter().map(event).collect();
    let intervals: Vec<_> = clipped
        .intervals
        .iter()
        .map(|i| format!("[{},{}]", i.parameter_range[0], i.parameter_range[1]))
        .collect();
    let pc = |c: &PCurve| -> Result<String> {
        let PCurve::Affine { origin, direction } = c else {
            return Err(Error::InvalidInput(
                "expected affine finite intersection pcurve",
            ));
        };
        Ok(format!(
            "{{\"origin\":[{},{}],\"direction\":[{},{}]}}",
            origin[0], origin[1], direction[0], direction[1]
        ))
    };
    let mut cuts = Vec::new();
    for s in segments {
        cuts.push(format!(
            "{{\"range\":[{},{}],\"start\":{},\"end\":{},\"first_uv\":{},\"second_uv\":{}}}",
            s.parameter_range[0],
            s.parameter_range[1],
            xyz(s.curve.evaluate(0.0)),
            xyz(s.curve.evaluate(1.0)),
            pc(&s.first)?,
            pc(&s.second)?
        ));
    }
    Ok(format!(
        "{{\"events\":[{}],\"intervals\":[{}],\"segments\":[{}],\"volume\":{}}}",
        events.join(","),
        intervals.join(","),
        cuts.join(","),
        first.volume()?
    ))
}
