//! Scoped convex, tangent line/arc profiles and exact normal extrusion.
use crate::*;
use std::f64::consts::{PI, TAU};
type P2 = [f64; 2];
#[derive(Clone, Copy, Debug)]
pub enum PlanarSegment {
    Line {
        a: P2,
        b: P2,
    },
    /// Counterclockwise circular arc; angles are radians in the profile XY plane.
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
                Ok(Vec3::new(-a.sin(), a.cos(), 0.0))
            }
        }
    }
    fn pcurve(&self) -> PCurve {
        match *self {
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
    /// Local XY origin; ring coordinates are offsets. No holes in this first API.
    pub origin: Point3,
    pub segments: Vec<PlanarSegment>,
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
    let t = (angle - start_angle).rem_euclid(TAU);
    t <= sweep + 1e-10 || TAU - t <= 1e-10
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
fn point_distance(p: P2, segment: PlanarSegment) -> Result<f64> {
    match segment {
        PlanarSegment::Line { a, b } => crate::planar::segment_distance(p, a, b),
        PlanarSegment::Arc { center, radius, .. } => {
            let delta = [p[0] - center[0], p[1] - center[1]];
            let mut result =
                distance(p, segment.evaluate(0.0)).min(distance(p, segment.evaluate(1.0)));
            if has_angle(segment, delta[1].atan2(delta[0])) {
                result = result.min((delta[0].hypot(delta[1]) - radius).abs());
            }
            if !result.is_finite() {
                return Err(Error::InvalidInput("arc distance exceeds finite range"));
            }
            Ok(result)
        }
    }
}
// Analytic candidate extrema/intersections; never validate from a display polygon.
fn boundary_distance(a: PlanarSegment, b: PlanarSegment) -> Result<f64> {
    let mut minimum = f64::INFINITY;
    for t in [0.0, 1.0] {
        minimum = minimum
            .min(point_distance(a.evaluate(t), b)?)
            .min(point_distance(b.evaluate(t), a)?);
    }
    match (a, b) {
        (PlanarSegment::Line { a, b }, PlanarSegment::Line { a: c, b: d }) => {
            if segments_intersect2d(a, b, c, d)? {
                minimum = 0.0;
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
                    minimum = minimum.min(point_distance(radial(arc, angle), line)?);
                }
            }
            if signed.abs() <= radius {
                let half = radius * (1.0 - (signed / radius).powi(2)).max(0.0).sqrt();
                for t in [along - half, along + half] {
                    if t >= 0.0 && t <= len {
                        let hit = [p[0] + t * u[0], p[1] + t * u[1]];
                        if has_angle(arc, (hit[1] - center[1]).atan2(hit[0] - center[0])) {
                            minimum = 0.0;
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
                radius: s,
                ..
            },
        ) => {
            let span = distance(c, d);
            if !span.is_finite() || !(r + s).is_finite() {
                return Err(Error::InvalidInput(
                    "arc/arc separation exceeds finite range",
                ));
            }
            if span > 0.0 {
                let angle = (d[1] - c[1]).atan2(d[0] - c[0]);
                for x in [angle, angle + PI] {
                    for y in [angle, angle + PI] {
                        if has_angle(a, x) && has_angle(b, y) {
                            minimum = minimum.min(distance(radial(a, x), radial(b, y)));
                        }
                    }
                }
                if span >= (r - s).abs() && span <= r + s {
                    let x = span * 0.5 + ((r - s) / span) * ((r + s) * 0.5);
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
                            minimum = 0.0;
                        }
                    }
                }
            }
        }
    }
    if !minimum.is_finite() {
        return Err(Error::InvalidInput(
            "mixed boundary distance exceeds finite range",
        ));
    }
    Ok(minimum)
}
pub(crate) fn validate_mixed(segments: &[PlanarSegment], tol: Tolerance) -> Result<()> {
    Tolerance::new(tol.linear)?;
    if segments.len() < 2 || segments.len() > 1024 {
        return Err(Error::Unsupported(
            "mixed profiles require 2..1024 segments",
        ));
    }
    let mut turn = 0.0;
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
                    || sweep <= 0.0
                    || sweep > PI
                    || !((radius * sweep).is_finite())
                    || radius * sweep <= 10.0 * tol.linear
                {
                    return Err(Error::InvalidInput("arc requires finite center/radius, start in [-2pi,2pi], CCW sweep in (0,pi], and size exceeding ten tolerances"));
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
                turn += sweep;
            }
        }
    }
    if (turn - TAU).abs() > 64.0 * f64::EPSILON * TAU * segments.len() as f64 {
        return Err(Error::Unsupported(
            "convex tangent profile arcs must turn exactly once counterclockwise",
        ));
    }
    for i in 0..segments.len() {
        let a = segments[i];
        let b = segments[(i + 1) % segments.len()];
        if distance(a.evaluate(1.0), b.evaluate(0.0)) > tol.linear {
            return Err(Error::InvalidInput("mixed profile is not closed"));
        }
        if (a.tangent(true)? - b.tangent(false)?).norm() > 1e-10 {
            return Err(Error::Unsupported(
                "mixed profile joins must be tangent and consistently oriented",
            ));
        }
        if matches!(
            (a, b),
            (PlanarSegment::Line { .. }, PlanarSegment::Line { .. })
        ) {
            return Err(Error::Unsupported(
                "merge consecutive straight segments before extrusion",
            ));
        }
        for j in i + 1..segments.len() {
            if j == i + 1 || (i == 0 && j == segments.len() - 1) {
                continue;
            }
            if boundary_distance(a, segments[j])? <= tol.linear {
                return Err(Error::InvalidInput(
                    "mixed profile crosses, touches or nearly touches itself",
                ));
            }
        }
    }
    let wire = Wire {
        coedges: segments
            .iter()
            .enumerate()
            .map(|(edge, s)| Coedge {
                edge,
                forward: true,
                pcurve: s.pcurve(),
            })
            .collect(),
    };
    let area = crate::topology::wire_area(&wire);
    if !area.is_finite() || area <= tol.linear * tol.linear {
        return Err(Error::InvalidInput(
            "mixed profile has no finite positive area",
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
/// Exact positive-normal extrusion of one convex, CCW, tangent line/arc loop.
/// No holes, concavity, clockwise arcs, corner joins or skew extrusion. Place
/// the resulting solid with `Solid::transformed` for arbitrary planes.
pub fn extrude_arc_line(profile: &ArcLineProfile, height: f64, tol: Tolerance) -> Result<Solid> {
    validate_mixed(&profile.segments, tol)?;
    if !profile.origin.finite()
        || !height.is_finite()
        || height <= 10.0 * tol.linear
        || !(profile.origin + Vec3::new(0.0, 0.0, height)).finite()
    {
        return Err(Error::InvalidInput(
            "mixed extrusion requires finite origin and positive height exceeding ten tolerances",
        ));
    }
    let n = profile.segments.len();
    let mut s = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    for z in [0.0, height] {
        for segment in &profile.segments {
            s.vertices.push(Vertex {
                point: profile.origin + p3(segment.evaluate(0.0)) + Vec3::new(0.0, 0.0, z),
            });
        }
    }
    for (i, segment) in profile.segments.iter().enumerate() {
        let j = (i + 1) % n;
        for (offset, z) in [(0, 0.0), (n, height)] {
            let curve = match *segment {
                PlanarSegment::Line { a, b } => Curve::Line {
                    a: profile.origin + p3(a) + Vec3::new(0.0, 0.0, z),
                    b: profile.origin + p3(b) + Vec3::new(0.0, 0.0, z),
                },
                PlanarSegment::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } => Curve::Arc {
                    frame: arc_frame(
                        profile.origin + Vec3::new(0.0, 0.0, z),
                        center,
                        start_angle,
                        tol,
                    )?,
                    radius,
                    sweep,
                },
            };
            s.edges.push(Edge {
                vertices: [i + offset, j + offset],
                curve,
            });
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
        s.shell.faces.push(Face {
            surface: Surface::Plane {
                origin: profile.origin + Vec3::new(0.0, 0.0, if top { height } else { 0.0 }),
                u: Vec3::new(1.0, 0.0, 0.0),
                v: Vec3::new(0.0, 1.0, 0.0),
            },
            orientation,
            wires: vec![Wire {
                coedges: profile
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(i, segment)| Coedge {
                        edge: 3 * i + usize::from(top),
                        forward: true,
                        pcurve: segment.pcurve(),
                    })
                    .collect(),
            }],
        });
    }
    for (i, segment) in profile.segments.iter().enumerate() {
        let next = (i + 1) % n;
        let (surface, span) = match *segment {
            PlanarSegment::Line { a, b } => (
                Surface::Plane {
                    origin: profile.origin + p3(a),
                    u: (p3(b) - p3(a)).normalized()?,
                    v: Vec3::new(0.0, 0.0, 1.0),
                },
                distance(a, b),
            ),
            PlanarSegment::Arc {
                center,
                radius,
                start_angle,
                sweep,
            } => (
                Surface::FramedCylinder {
                    frame: arc_frame(profile.origin, center, start_angle, tol)?,
                    radius,
                    height,
                },
                sweep,
            ),
        };
        let bottom_direction = if matches!(segment, PlanarSegment::Line { .. }) {
            span
        } else {
            1.0
        };
        let coedge = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        s.shell.faces.push(Face {
            surface,
            orientation: 1,
            wires: vec![Wire {
                coedges: vec![
                    coedge(3 * i, true, [0.0, 0.0], [bottom_direction, 0.0]),
                    coedge(3 * next + 2, true, [span, 0.0], [0.0, height]),
                    coedge(3 * i + 1, false, [0.0, height], [bottom_direction, 0.0]),
                    coedge(3 * i + 2, false, [0.0, 0.0], [0.0, height]),
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

#[cfg(test)]
mod tests {
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
