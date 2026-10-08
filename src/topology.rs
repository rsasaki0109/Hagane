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
    // Precisely delimit the trim domains covered by this first kernel milestone.
    fn validate_supported_trim(&self, tol: Tolerance) -> Result<()> {
        match self.surface {
            Surface::Plane { .. } => {
                if self.wires.len() > 2 {
                    return Err(Error::Unsupported(
                        "multiple planar holes are not supported yet",
                    ));
                }
                let outer = &self.wires[0];
                if outer.coedges.len() == 1
                    && matches!(outer.coedges[0].pcurve, PCurve::Circle { .. })
                {
                    if self.wires.len() != 1 {
                        return Err(Error::Unsupported(
                            "annular disk faces are not supported yet",
                        ));
                    }
                    return Ok(());
                }
                if outer.coedges.len() != 4
                    || outer
                        .coedges
                        .iter()
                        .any(|c| !matches!(c.pcurve, PCurve::Affine { .. }))
                {
                    return Err(Error::Unsupported(
                        "planar outer trim must be a rectangle or disk",
                    ));
                }
                let points: Vec<_> = outer
                    .coedges
                    .iter()
                    .map(|c| c.pcurve.evaluate(if c.forward { 0.0 } else { 1.0 }))
                    .collect();
                for i in 0..4 {
                    let a = points[i];
                    let b = points[(i + 1) % 4];
                    let d = [b[0] - a[0], b[1] - a[1]];
                    if (d[0].abs() > tol.linear && d[1].abs() > tol.linear)
                        || d[0].hypot(d[1]) <= tol.linear
                    {
                        return Err(Error::Unsupported(
                            "planar trim must be axis-aligned in its own surface parameters",
                        ));
                    }
                    let c = points[(i + 2) % 4];
                    let next = [c[0] - b[0], c[1] - b[1]];
                    if (d[0] * next[0] + d[1] * next[1]).abs()
                        > tol.linear * d[0].hypot(d[1]) * next[0].hypot(next[1])
                    {
                        return Err(Error::InvalidTopology("nonrectangular trim"));
                    }
                }
                if self.wires.len() == 2 {
                    let hole = &self.wires[1];
                    if hole.coedges.len() != 1 {
                        return Err(Error::Unsupported(
                            "only a single circular planar hole is supported",
                        ));
                    }
                    let PCurve::Circle { center, radius } = hole.coedges[0].pcurve else {
                        return Err(Error::Unsupported(
                            "only circular planar holes are supported",
                        ));
                    };
                    for axis in 0..2 {
                        let lo = points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
                        let hi = points
                            .iter()
                            .map(|p| p[axis])
                            .fold(f64::NEG_INFINITY, f64::max);
                        if center[axis] - radius <= lo + tol.linear
                            || center[axis] + radius >= hi - tol.linear
                        {
                            return Err(Error::InvalidTopology(
                                "circular trim touches or leaves its outer rectangle",
                            ));
                        }
                    }
                }
            }
            Surface::Cylinder { height, .. } => {
                if self.wires.len() != 1 || self.wires[0].coedges.len() != 4 {
                    return Err(Error::Unsupported(
                        "only complete cylindrical faces are supported",
                    ));
                }
                let expected = [
                    ([0.0, 0.0], [1.0, 0.0], true),
                    ([TAU, 0.0], [0.0, height], true),
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
                            "partial or reordered cylindrical trim is unsupported",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
impl Solid {
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
                Curve::Circle { radius, .. } if !radius.is_finite() || radius <= tol.linear => {
                    return Err(Error::InvalidTopology("invalid circle"))
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
                        || !u.finite()
                        || !v.finite()
                        || (u.norm() - 1.0).abs() > tol.linear
                        || (v.norm() - 1.0).abs() > tol.linear
                        || u.dot(v).abs() > tol.linear
                    {
                        return Err(Error::InvalidTopology("invalid plane basis"));
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
                Surface::Cylinder { radius, height, .. } => {
                    if f.wires.len() != 1 || f.wires[0].coedges.len() != 4 {
                        return Err(Error::Unsupported(
                            "volume requires a complete cylindrical face",
                        ));
                    }
                    TAU * radius * radius * height / 3.0
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
            if let Curve::Circle { center, radius } = e.curve {
                add(center + Vec3::new(radius, radius, 0.0));
                add(center - Vec3::new(radius, radius, 0.0));
            }
        }
        Bounds { min: lo, max: hi }
    }
}
pub(crate) fn wire_area(w: &Wire) -> f64 {
    w.coedges
        .iter()
        .map(|c| {
            let a = match c.pcurve {
                PCurve::Affine { origin, direction } => {
                    0.5 * (origin[0] * direction[1] - origin[1] * direction[0])
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
