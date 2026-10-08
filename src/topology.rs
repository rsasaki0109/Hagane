use crate::*;
use std::f64::consts::TAU;
#[derive(Clone, Debug)]
pub struct Vertex {
    pub point: Point3,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub vertices: [usize; 2],
    pub curve: Curve,
}
/// Evaluated at the SAME parameter as the owning 3D edge, including reversed uses.
#[derive(Clone, Debug)]
pub enum PCurve {
    Arc {
        center: [f64; 2],
        radius: f64,
        start_angle: f64,
        sweep: f64,
    },
    Affine {
        origin: [f64; 2],
        direction: [f64; 2],
    },
    Circle {
        center: [f64; 2],
        radius: f64,
    },
}
impl PCurve {
    pub fn evaluate(&self, t: f64) -> [f64; 2] {
        match *self {
            Self::Affine { origin, direction } => {
                [origin[0] + direction[0] * t, origin[1] + direction[1] * t]
            }
            Self::Arc {
                center,
                radius,
                start_angle,
                ..
            } => [
                center[0] + radius * (start_angle + t).cos(),
                center[1] + radius * (start_angle + t).sin(),
            ],
            Self::Circle { center, radius } => {
                [center[0] + radius * t.cos(), center[1] + radius * t.sin()]
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct Coedge {
    pub edge: usize,
    pub forward: bool,
    pub pcurve: PCurve,
}
#[derive(Clone, Debug)]
pub struct Wire {
    pub coedges: Vec<Coedge>,
}
/// Wires follow the positive parameter-domain orientation: outer CCW, holes CW.
/// Orientation reverses the complete face, including all its coedges.
#[derive(Clone, Debug)]
pub struct Face {
    pub surface: Surface,
    pub wires: Vec<Wire>,
    pub orientation: i8,
}
#[derive(Clone, Debug)]
pub struct Shell {
    pub faces: Vec<Face>,
}
#[derive(Clone, Debug)]
pub struct Solid {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub shell: Shell,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Point3,
    pub max: Point3,
}
impl Face {
    pub(crate) fn cylinder_span(&self) -> Result<f64> {
        if self.wires.len() != 1 || self.wires[0].coedges.len() != 4 {
            return Err(Error::Unsupported(
                "cylindrical trim requires a four-coedge rectangle",
            ));
        }
        let PCurve::Affine { origin, .. } = self.wires[0].coedges[1].pcurve else {
            return Err(Error::Unsupported("nonrectangular cylindrical trim"));
        };
        let span = origin[0];
        if !span.is_finite() || span <= 0.0 || span > TAU {
            return Err(Error::Unsupported(
                "cylindrical angular span must be in (0, 2pi]",
            ));
        }
        Ok(span)
    }
    // Precisely delimit the trim domains covered by this first kernel milestone.
    fn validate_supported_trim(&self, tol: Tolerance) -> Result<()> {
        match self.surface {
            Surface::Plane { .. } => {
                if self
                    .wires
                    .iter()
                    .flat_map(|w| &w.coedges)
                    .any(|c| matches!(c.pcurve, PCurve::Arc { .. }))
                {
                    if self.wires.len() != 1 {
                        return Err(Error::Unsupported("mixed arc-line planar trims currently support one convex outer wire, without holes"));
                    }
                    let mut segments = Vec::new();
                    for c in &self.wires[0].coedges {
                        segments.push(match c.pcurve {
                            PCurve::Affine { .. } => PlanarSegment::Line {
                                a: c.pcurve.evaluate(if c.forward { 0.0 } else { 1.0 }),
                                b: c.pcurve.evaluate(if c.forward { 1.0 } else { 0.0 }),
                            },
                            PCurve::Arc { center, radius, start_angle, sweep } if c.forward => {
                                PlanarSegment::Arc { center, radius, start_angle, sweep }
                            },
                            _ => return Err(Error::Unsupported("mixed trim arcs must be counterclockwise and cannot use full-circle coedges")),
                        });
                    }
                    crate::mixed::validate_mixed(&segments, tol)?;
                    return Ok(());
                }
                use crate::planar::{validate_polygon, validate_region, PlanarLoop};
                let mut loops = Vec::new();
                for w in &self.wires {
                    if w.coedges.len() == 1 {
                        let PCurve::Circle { center, radius } = w.coedges[0].pcurve else {
                            return Err(Error::Unsupported(
                                "one-edge planar wire must be circular",
                            ));
                        };
                        if !radius.is_finite() || radius <= tol.linear {
                            return Err(Error::InvalidTopology("invalid circular trim"));
                        }
                        loops.push(PlanarLoop::Circle { center, radius });
                    } else {
                        if w.coedges
                            .iter()
                            .any(|c| !matches!(c.pcurve, PCurve::Affine { .. }))
                        {
                            return Err(Error::Unsupported(
                                "full-circle coedges cannot be combined with other planar trim segments",
                            ));
                        }
                        let points: Vec<_> = w
                            .coedges
                            .iter()
                            .map(|c| c.pcurve.evaluate(if c.forward { 0.0 } else { 1.0 }))
                            .collect();
                        validate_polygon(&points, tol)?;
                        loops.push(PlanarLoop::Polygon(points));
                    }
                }
                validate_region(&loops[0], &loops[1..], tol)?;
            }

            Surface::Cylinder { height, .. } | Surface::FramedCylinder { height, .. } => {
                if self.wires.len() != 1 || self.wires[0].coedges.len() != 4 {
                    return Err(Error::Unsupported(
                        "cylindrical faces require one rectangular four-coedge wire",
                    ));
                }
                let span = self.cylinder_span()?;
                let expected = [
                    ([0.0, 0.0], [1.0, 0.0], true),
                    ([span, 0.0], [0.0, height], true),
                    ([0.0, height], [1.0, 0.0], false),
                    ([0.0, 0.0], [0.0, height], false),
                ];
                for (c, (o, d, forward)) in self.wires[0].coedges.iter().zip(expected) {
                    let PCurve::Affine { origin, direction } = c.pcurve else {
                        return Err(Error::Unsupported("nonrectangular cylindrical trim"));
                    };
                    if c.forward != forward
                        || (0..2).any(|i| {
                            (origin[i] - o[i]).abs() > tol.linear
                                || (direction[i] - d[i]).abs() > tol.linear
                        })
                    {
                        return Err(Error::Unsupported(
                            "only cylindrical rectangles from u=0, v=0 to u=span, v=height are supported",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
impl Solid {
    /// Copies exact geometry through a rigid transform. Shared indices, edge
    /// parameters, pcurves and orientations are unchanged; both solids are checked.
    pub fn transformed(&self, transform: Transform, tol: Tolerance) -> Result<Self> {
        self.validate(tol)?;
        let mut result = self.clone();
        for v in &mut result.vertices {
            v.point = transform.point(v.point);
        }
        for e in &mut result.edges {
            e.curve = e.curve.transformed(transform)?;
        }
        for f in &mut result.shell.faces {
            f.surface = f.surface.transformed(transform)?;
        }
        result.validate(tol)?;
        let original_volume = self.volume()?;
        if (result.volume()? - original_volume).abs() > original_volume * 1e-10 {
            return Err(Error::InvalidInput(
                "rigid placement loses volume at this coordinate magnitude",
            ));
        }
        Ok(result)
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        if self.shell.faces.is_empty() || self.vertices.is_empty() {
            return Err(Error::InvalidTopology("empty solid"));
        }
        if self.vertices.iter().any(|v| !v.point.finite()) {
            return Err(Error::InvalidTopology("nonfinite vertex"));
        }
        for e in &self.edges {
            if e.vertices.iter().any(|&i| i >= self.vertices.len()) {
                return Err(Error::InvalidTopology("invalid vertex index"));
            }
            let [a, b] = e.curve.range();
            if !tol.coincident(self.vertices[e.vertices[0]].point, e.curve.evaluate(a))
                || !tol.coincident(self.vertices[e.vertices[1]].point, e.curve.evaluate(b))
            {
                return Err(Error::InvalidTopology("edge endpoints disagree with curve"));
            }
            match e.curve {
                Curve::Line { a, b } if (a - b).norm() <= tol.linear => {
                    return Err(Error::InvalidTopology("degenerate edge"))
                }
                Curve::Circle { radius, .. } | Curve::FramedCircle { radius, .. }
                    if !radius.is_finite() || radius <= tol.linear =>
                {
                    return Err(Error::InvalidTopology("invalid circle"))
                }
                Curve::Arc { radius, sweep, .. }
                    if !radius.is_finite()
                        || radius <= tol.linear
                        || !sweep.is_finite()
                        || sweep <= 0.0
                        || sweep > std::f64::consts::PI =>
                {
                    return Err(Error::InvalidTopology("invalid bounded arc"));
                }
                _ => {}
            }
        }
        let mut uses = vec![Vec::new(); self.edges.len()];
        let mut vertex_used = vec![false; self.vertices.len()];
        for (fi, f) in self.shell.faces.iter().enumerate() {
            if ![-1, 1].contains(&f.orientation) || f.wires.is_empty() {
                return Err(Error::InvalidTopology(
                    "invalid face orientation or empty boundary",
                ));
            }
            match f.surface {
                Surface::Plane { origin, u, v } => {
                    if !origin.finite()
                        || !crate::geometry::plane_basis_valid(
                            u,
                            v,
                            GeometryTolerance::try_from(tol)?,
                        )
                    {
                        return Err(Error::InvalidTopology("invalid plane basis"));
                    }
                }
                Surface::FramedCylinder { radius, height, .. } => {
                    if !radius.is_finite()
                        || !height.is_finite()
                        || radius <= tol.linear
                        || height <= tol.linear
                    {
                        return Err(Error::InvalidTopology("invalid framed cylinder surface"));
                    }
                }
                Surface::Cylinder {
                    center,
                    radius,
                    height,
                } => {
                    if !center.finite()
                        || !radius.is_finite()
                        || !height.is_finite()
                        || radius <= tol.linear
                        || height <= tol.linear
                    {
                        return Err(Error::InvalidTopology("invalid cylinder surface"));
                    }
                }
            }
            for w in &f.wires {
                if w.coedges.is_empty() {
                    return Err(Error::InvalidTopology("empty wire"));
                }
                for (i, c) in w.coedges.iter().enumerate() {
                    let e = self
                        .edges
                        .get(c.edge)
                        .ok_or(Error::InvalidTopology("invalid edge index"))?;
                    let next = &w.coedges[(i + 1) % w.coedges.len()];
                    let ne = self
                        .edges
                        .get(next.edge)
                        .ok_or(Error::InvalidTopology("invalid edge index"))?;
                    let end = e.vertices[usize::from(c.forward)];
                    let start = ne.vertices[usize::from(!next.forward)];
                    if end != start {
                        return Err(Error::InvalidTopology("wire is not topologically closed"));
                    }
                    match (&e.curve, &c.pcurve) {
                        (Curve::Line { .. }, PCurve::Circle { .. } | PCurve::Arc { .. }) => {
                            return Err(Error::InvalidTopology(
                                "line edge cannot have a circular pcurve",
                            ))
                        }
                        (
                            Curve::Circle { .. } | Curve::FramedCircle { .. } | Curve::Arc { .. },
                            PCurve::Affine { .. },
                        ) if matches!(f.surface, Surface::Plane { .. }) => {
                            return Err(Error::InvalidTopology(
                                "planar circular edge requires a circular pcurve",
                            ))
                        }
                        (
                            Curve::Arc { sweep, .. },
                            PCurve::Arc {
                                sweep: pc_sweep, ..
                            },
                        ) if (sweep - pc_sweep).abs() > 1e-10 => {
                            return Err(Error::InvalidTopology(
                                "arc and pcurve angular spans disagree",
                            ))
                        }
                        (Curve::Arc { .. }, PCurve::Circle { .. })
                        | (Curve::Circle { .. } | Curve::FramedCircle { .. }, PCurve::Arc { .. }) => {
                            return Err(Error::InvalidTopology(
                                "bounded arcs and full-circle pcurves cannot be interchanged",
                            ))
                        }
                        _ => {}
                    }
                    let range = e.curve.range();
                    for k in 0..=8 {
                        let t = range[0] + (range[1] - range[0]) * k as f64 / 8.0;
                        let uv = c.pcurve.evaluate(t);
                        if !uv.iter().all(|v| v.is_finite())
                            || !tol
                                .coincident(e.curve.evaluate(t), f.surface.evaluate(uv[0], uv[1]))
                        {
                            return Err(Error::InvalidTopology("pcurve disagrees with 3D edge"));
                        }
                    }
                    let uv = c.pcurve.evaluate(range[usize::from(c.forward)]);
                    let nr = ne.curve.range();
                    let nv = next.pcurve.evaluate(nr[usize::from(!next.forward)]);
                    if (uv[0] - nv[0]).hypot(uv[1] - nv[1]) > tol.linear {
                        return Err(Error::InvalidTopology(
                            "wire is not closed in surface parameters",
                        ));
                    }
                    vertex_used[e.vertices[0]] = true;
                    vertex_used[e.vertices[1]] = true;
                    uses[c.edge].push((
                        fi,
                        if c.forward {
                            f.orientation
                        } else {
                            -f.orientation
                        },
                    ));
                }
            }
            if matches!(f.surface, Surface::Plane { .. }) {
                for (i, w) in f.wires.iter().enumerate() {
                    let a = wire_area(w);
                    if !a.is_finite()
                        || a.abs() <= tol.linear * tol.linear
                        || (i == 0 && a < 0.0)
                        || (i > 0 && a > 0.0)
                    {
                        return Err(Error::InvalidTopology(
                            "invalid planar wire winding or area",
                        ));
                    }
                }
            }
        }
        for f in &self.shell.faces {
            f.validate_supported_trim(tol)?;
        }
        if vertex_used.iter().any(|v| !*v)
            || uses.iter().any(|u| u.len() != 2 || u[0].1 + u[1].1 != 0)
        {
            return Err(Error::InvalidTopology(
                "shell is not closed, shared edges must have opposite orientations",
            ));
        }
        // The link of every vertex must be one cycle. Edge endpoints are distinct
        // link nodes even for closed circles; this also handles periodic seams.
        for vertex in 0..self.vertices.len() {
            let mut links = std::collections::BTreeMap::<usize, Vec<usize>>::new();
            for f in &self.shell.faces {
                for w in &f.wires {
                    for i in 0..w.coedges.len() {
                        let c = &w.coedges[i];
                        let prev = &w.coedges[(i + w.coedges.len() - 1) % w.coedges.len()];
                        let start = usize::from(!c.forward);
                        let end = usize::from(prev.forward);
                        if self.edges[c.edge].vertices[start] == vertex {
                            let a = 2 * c.edge + start;
                            let b = 2 * prev.edge + end;
                            links.entry(a).or_default().push(b);
                            links.entry(b).or_default().push(a);
                        }
                    }
                }
            }
            if links.values().any(|adj| adj.len() != 2) {
                return Err(Error::InvalidTopology("nonmanifold vertex link"));
            }
            let mut pending = vec![*links
                .keys()
                .next()
                .ok_or(Error::InvalidTopology("unused vertex"))?];
            let mut reached = std::collections::BTreeSet::new();
            while let Some(node) = pending.pop() {
                if reached.insert(node) {
                    pending.extend(&links[&node]);
                }
            }
            if reached.len() != links.len() {
                return Err(Error::InvalidTopology("disconnected vertex link"));
            }
        }
        let mut seen = vec![false; self.shell.faces.len()];
        let mut pending = vec![0];
        while let Some(i) = pending.pop() {
            if seen[i] {
                continue;
            }
            seen[i] = true;
            for u in &uses {
                if u[0].0 == i {
                    pending.push(u[1].0);
                }
                if u[1].0 == i {
                    pending.push(u[0].0);
                }
            }
        }
        if seen.iter().any(|s| !*s) {
            return Err(Error::InvalidTopology("disconnected shell"));
        }
        if self.volume()? <= tol.linear.powi(3) {
            return Err(Error::InvalidTopology("nonpositive volume"));
        }
        Ok(())
    }
    /// Divergence theorem, integrated analytically over the supported exact surfaces.
    pub fn volume(&self) -> Result<f64> {
        let reference = self
            .vertices
            .first()
            .ok_or(Error::InvalidTopology("empty solid"))?
            .point;
        let mut sum = 0.0;
        for f in &self.shell.faces {
            let term = match f.surface {
                Surface::Plane { origin, u, v } => {
                    (origin - reference).dot(u.cross(v))
                        * f.wires.iter().map(wire_area).sum::<f64>()
                        / 3.0
                }
                Surface::Cylinder { radius, height, .. }
                | Surface::FramedCylinder { radius, height, .. } => {
                    let span = f.cylinder_span()?;
                    if span == TAU {
                        TAU * radius * radius * height / 3.0
                    } else {
                        let (center, u, v) = match f.surface {
                            Surface::Cylinder { center, .. } => {
                                (center, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
                            }
                            Surface::FramedCylinder { frame, .. } => {
                                (frame.origin(), frame.axes()[0], frame.axes()[1])
                            }
                            _ => unreachable!(),
                        };
                        let delta = center - reference;
                        radius * height / 3.0
                            * (radius * span
                                + delta.dot(u) * span.sin()
                                + delta.dot(v) * (1.0 - span.cos()))
                    }
                }
            };
            sum += term * f.orientation as f64;
        }
        if !sum.is_finite() {
            return Err(Error::InvalidInput("nonfinite volume"));
        }
        Ok(sum)
    }
    pub fn bounds(&self) -> Bounds {
        let mut lo = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut hi = lo * (-1.0);
        let mut add = |p: Point3| {
            lo.x = lo.x.min(p.x);
            lo.y = lo.y.min(p.y);
            lo.z = lo.z.min(p.z);
            hi.x = hi.x.max(p.x);
            hi.y = hi.y.max(p.y);
            hi.z = hi.z.max(p.z);
        };
        for v in &self.vertices {
            add(v.point);
        }
        for e in &self.edges {
            let circle = match e.curve {
                Curve::Circle { center, radius } => Some((
                    center,
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    radius,
                )),
                Curve::FramedCircle { frame, radius } => {
                    Some((frame.origin(), frame.axes()[0], frame.axes()[1], radius))
                }
                Curve::Arc {
                    frame,
                    radius,
                    sweep,
                } => {
                    for angle in [0.0, sweep] {
                        add(frame.point(Vec3::new(
                            radius * angle.cos(),
                            radius * angle.sin(),
                            0.0,
                        )));
                    }
                    let [u, v, _] = frame.axes();
                    for (a, b) in [(u.x, v.x), (u.y, v.y), (u.z, v.z)] {
                        let angle = b.atan2(a).rem_euclid(TAU);
                        for t in [angle, (angle + std::f64::consts::PI).rem_euclid(TAU)] {
                            if t <= sweep {
                                add(frame.point(Vec3::new(
                                    radius * t.cos(),
                                    radius * t.sin(),
                                    0.0,
                                )));
                            }
                        }
                    }
                    None
                }
                Curve::Line { .. } => None,
            };
            if let Some((center, u, v, radius)) = circle {
                let extent = Vec3::new(u.x.hypot(v.x), u.y.hypot(v.y), u.z.hypot(v.z)) * radius;
                add(center + extent);
                add(center - extent);
            }
        }
        Bounds { min: lo, max: hi }
    }
}
pub(crate) fn wire_area(w: &Wire) -> f64 {
    let reference = w
        .coedges
        .first()
        .map(|c| c.pcurve.evaluate(0.0))
        .unwrap_or([0.0, 0.0]);
    w.coedges
        .iter()
        .map(|c| {
            let a = match c.pcurve {
                PCurve::Affine { origin, direction } => {
                    0.5 * ((origin[0] - reference[0]) * direction[1]
                        - (origin[1] - reference[1]) * direction[0])
                }
                PCurve::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } => {
                    let end = start_angle + sweep;
                    0.5 * (radius
                        * ((center[0] - reference[0]) * (end.sin() - start_angle.sin())
                            - (center[1] - reference[1]) * (end.cos() - start_angle.cos()))
                        + radius * radius * sweep)
                }
                PCurve::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
            };
            if c.forward {
                a
            } else {
                -a
            }
        })
        .sum()
}
