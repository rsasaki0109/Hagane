//! Checked full-ellipse planar trims; arbitrary mixed ellipse loops are unsupported.
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
#[derive(Clone)]
pub(crate) struct EllipseRing {
    center: P2,
    cosine: P2,
    sine: P2,
    coherence_error: f64,
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
        if norm(sub(p, self.point(angle))) + roundoff <= budget {
            return Ok(true);
        }
        let mut pending: Vec<_> = (0..4)
            .map(|i| (i as f64 * PI / 2., (i + 1) as f64 * PI / 2., 0))
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
        Ok(false)
    }
}
pub(crate) fn has_ellipse(f: &Face) -> bool {
    f.wires
        .iter()
        .flat_map(|w| &w.coedges)
        .any(|c| matches!(c.pcurve, PCurve::EllipseArc { .. }))
}
/// Initial domain: one complete ellipse encoded by two opposite semicircle parameters.
pub(crate) fn ring(face: &Face, tol: Tolerance) -> Result<EllipseRing> {
    if !matches!(face.surface, Surface::Plane { .. })
        || face.wires.len() != 1
        || face.wires[0].coedges.len() != 2
    {
        return Err(Error::Unsupported(
            "planar ellipse trims require one two-half-arc ellipse wire",
        ));
    }
    let coedges = &face.wires[0].coedges;
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
    };
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
    let ellipse = ring(face, tol.absolute())?;
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
    let mut events = Vec::new();
    for travel in [along - half, along + half] {
        let circle = [q[0] + h[0] * travel, q[1] + h[1] * travel];
        let angle = circle[1].atan2(circle[0]).rem_euclid(TAU);
        let coedge = usize::from(angle >= PI);
        let t = angle - coedge as f64 * PI;
        let c = &face.wires[0].coedges[coedge];
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
            wire: 0,
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
            holes: vec![],
            outer: [0., PI]
                .map(|start_angle| PlanarSegment::Arc {
                    center: [0., 0.],
                    radius,
                    start_angle,
                    sweep: PI,
                })
                .to_vec(),
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
        .map(|pair| solid.shell.faces[pair[0]].clone())
        .collect();
    if sides.len() != 2 {
        return Err(Error::InvalidTopology(
            "ellipse fixture requires two circular walls",
        ));
    }
    let edges: Vec<_> = sides.iter().map(|f| f.wires[0].coedges[2].edge).collect();
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
    let coedges = edges
        .iter()
        .map(|&edge| {
            let Curve::EllipseArc {
                center: c,
                cosine: a,
                sine: b,
                sweep,
            } = solid.edges[edge].curve
            else {
                unreachable!()
            };
            Coedge {
                edge,
                forward: true,
                pcurve: PCurve::EllipseArc {
                    center: surface.parameters(c),
                    cosine: [a.dot(u), a.dot(v)],
                    sine: [b.dot(u), b.dot(v)],
                    sweep,
                },
            }
        })
        .collect();
    let mut faces = vec![solid.shell.faces[0].clone()];
    faces.extend(sides);
    faces.push(Face {
        surface,
        orientation: 1,
        wires: vec![Wire { coedges }],
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
    let result = clip_line_to_planar_face(&solid, face, anchor, direction, t)?;
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
    fn nonorthogonal_axes_inverse_distance_and_conditioning() {
        let ellipse = EllipseRing {
            center: [100., -20.],
            cosine: [2., 0.],
            sine: [1., 1.],
            coherence_error: 0.,
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
        };
        assert!(bad.inverse([0., 0.]).is_err());
    }
}
