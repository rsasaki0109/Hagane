//! Structural certification of a simple normal extrusion with bounded circular rims.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
const DOMAIN: Error = Error::Unsupported(
    "bounded STEP requires a structurally certified simple line/arc normal prism",
);
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn translated_curve(a: &Curve, b: &Curve, delta: Vec3, reverse: bool, budget: f64) -> Result<()> {
    match (a, b) {
        (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => {
            let (c, d) = if reverse { (*d, *c) } else { (*c, *d) };
            if length(*a + delta - c) > budget || length(*b + delta - d) > budget {
                return Err(DOMAIN);
            }
        }
        (
            Curve::Arc {
                frame: a,
                radius: ra,
                sweep: sa,
            },
            Curve::Arc {
                frame: b,
                radius: rb,
                sweep: sb,
            },
        ) => {
            let aa = a.axes();
            let bb = b.axes();
            let (u, v) = if reverse {
                let (sin, cos) = sb.sin_cos();
                (bb[0] * cos + bb[1] * sin, bb[0] * sin - bb[1] * cos)
            } else {
                (bb[0], bb[1])
            };
            let bound = length(a.origin() + delta - b.origin())
                + (ra - rb).abs()
                + ra.max(*rb) * (length(aa[0] - u) + length(aa[1] - v) + (sa - sb).abs());
            if !bound.is_finite() || bound > budget {
                return Err(DOMAIN);
            }
        }
        _ => return Err(DOMAIN),
    }
    Ok(())
}
pub(crate) fn certify_validated_arc_line_prism(s: &Solid, tol: Tolerance) -> Result<()> {
    Tolerance::new(tol.linear)?;
    if s.shell.faces.len() > 130 || s.edges.len() > 384 || s.vertices.len() > 256 {
        return Err(DOMAIN);
    }
    let caps: Vec<_> = s
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            matches!(f.surface, Surface::Plane { .. })
                && f.wires
                    .iter()
                    .flat_map(|w| &w.coedges)
                    .any(|c| matches!(s.edges[c.edge].curve, Curve::Arc { .. }))
        })
        .map(|(i, _)| i)
        .collect();
    if caps.len() != 2 {
        return Err(DOMAIN);
    }
    let base = &s.shell.faces[caps[0]];
    let top = &s.shell.faces[caps[1]];
    if !(1..=17).contains(&base.wires.len()) || top.wires.len() != base.wires.len() {
        return Err(DOMAIN);
    }
    if base.wires.iter().chain(&top.wires).any(|w| {
        w.coedges.len() < 2
            || w.coedges.iter().any(|c| {
                !matches!(
                    s.edges[c.edge].curve,
                    Curve::Line { .. } | Curve::Arc { .. }
                ) || !matches!(c.pcurve, PCurve::Affine { .. } | PCurve::Arc { .. })
            })
    }) {
        return Err(DOMAIN);
    }
    let count = base.wires.iter().map(|w| w.coedges.len()).sum::<usize>();
    if !(2..=128).contains(&count)
        || top.wires.iter().map(|w| w.coedges.len()).sum::<usize>() != count
        || s.vertices.len() != 2 * count
        || s.edges.len() != 3 * count
        || s.shell.faces.len() != count + 2
    {
        return Err(DOMAIN);
    }
    let Surface::Plane { origin, u, v } = base.surface else {
        return Err(DOMAIN);
    };
    let axis = u.cross(v) * -f64::from(base.orientation);
    let frame = Frame3::new(origin, [u, axis.cross(u), axis], tol)?;
    let Surface::Plane {
        origin: upper,
        u: tu,
        v: tv,
    } = top.surface
    else {
        return Err(DOMAIN);
    };
    let height = (upper - origin).dot(frame.axes()[2]);
    let extent = s.bounds().max - s.bounds().min;
    let scale = length(extent);
    let mut world = s.vertices.iter().fold(scale, |w, p| {
        w.max(p.point.x.abs())
            .max(p.point.y.abs())
            .max(p.point.z.abs())
    });
    for f in &s.shell.faces {
        let o = match f.surface {
            Surface::Plane { origin, .. } => origin,
            Surface::Cylinder { center, .. } => center,
            Surface::FramedCylinder { frame, .. } => frame.origin(),
            _ => return Err(DOMAIN),
        };
        world = world.max(o.x.abs()).max(o.y.abs()).max(o.z.abs());
    }
    for e in &s.edges {
        if let Curve::Arc { frame, .. } = e.curve {
            let o = frame.origin();
            world = world.max(o.x.abs()).max(o.y.abs()).max(o.z.abs());
        }
    }
    let arithmetic = 4096. * f64::EPSILON * world;
    let budget = tol.linear / 32.;
    if !arithmetic.is_finite() || arithmetic >= budget / 8. {
        return Err(Error::Unsupported(
            "bounded line/arc prism coordinate precision is unresolved at caller tolerance",
        ));
    }
    if !height.is_finite()
        || height <= 10. * tol.linear
        || length(tu.cross(tv) * f64::from(top.orientation) - frame.axes()[2]) * scale > budget
    {
        return Err(DOMAIN);
    }
    let raw_axes = [u, axis.cross(u), axis];
    let frame_drift = raw_axes
        .iter()
        .zip(frame.axes())
        .map(|(&a, b)| length(a - b))
        .fold(0.0, f64::max)
        * scale;
    if !frame_drift.is_finite() || frame_drift + arithmetic >= budget / 8.0 {
        return Err(Error::Unsupported(
            "bounded line/arc prism frame reconstruction precision is unresolved",
        ));
    }
    let delta = frame.axes()[2] * height;
    let mut profiles = crate::face_intersections::rings(base)?;
    if base.orientation == 1 {
        for segment in profiles.iter_mut().flatten() {
            match segment {
                PlanarSegment::Line { a, b } => {
                    a[1] = -a[1];
                    b[1] = -b[1];
                }
                PlanarSegment::Arc {
                    center,
                    start_angle,
                    sweep,
                    ..
                } => {
                    center[1] = -center[1];
                    *start_angle = -*start_angle;
                    *sweep = -*sweep;
                }
            }
        }
    }
    // This trusted constructor validates the actual cap's simple region. It is a
    // check only: no imported vertex, curve, surface or pcurve is replaced.
    let outer = profiles.remove(0);
    let expected = extrude_arc_line_region_in_frame(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: profiles,
        },
        delta,
        frame,
        tol,
    )?;
    let bottom_edges: BTreeSet<_> = base
        .wires
        .iter()
        .flat_map(|w| &w.coedges)
        .map(|c| c.edge)
        .collect();
    let top_edges: BTreeSet<_> = top
        .wires
        .iter()
        .flat_map(|w| &w.coedges)
        .map(|c| c.edge)
        .collect();
    if bottom_edges.len() != count
        || top_edges.len() != count
        || !bottom_edges.is_disjoint(&top_edges)
    {
        return Err(DOMAIN);
    }
    let bottom_vertices: BTreeSet<_> = bottom_edges
        .iter()
        .flat_map(|&i| s.edges[i].vertices)
        .collect();
    let top_vertices: BTreeSet<_> = top_edges
        .iter()
        .flat_map(|&i| s.edges[i].vertices)
        .collect();
    if bottom_vertices.len() != count
        || top_vertices.len() != count
        || !bottom_vertices.is_disjoint(&top_vertices)
    {
        return Err(DOMAIN);
    }
    // The simple-profile check must describe the actual 3D cap boundary, not
    // merely a different positive-area pcurve region within generic tolerance.
    let mut expected_rims = BTreeSet::new();
    for &index in &bottom_edges {
        let actual = &s.edges[index];
        let start = s.vertices[actual.vertices[0]].point;
        let end = s.vertices[actual.vertices[1]].point;
        let mut matches = Vec::new();
        for (i, candidate) in expected.edges.iter().enumerate() {
            let [a, b] = candidate.vertices.map(|j| expected.vertices[j].point);
            let reverse = length(start - b) <= budget && length(end - a) <= budget;
            if ((length(start - a) <= budget && length(end - b) <= budget) || reverse)
                && translated_curve(
                    &actual.curve,
                    &candidate.curve,
                    Vec3::new(0., 0., 0.),
                    reverse,
                    budget,
                )
                .is_ok()
            {
                matches.push(i);
            }
        }
        if matches.len() != 1 || !expected_rims.insert(matches[0]) {
            return Err(DOMAIN);
        }
    }
    let mut vertex_map = BTreeMap::new();
    let mut matched = BTreeSet::new();
    for &i in &bottom_vertices {
        let p = s.vertices[i].point + delta;
        let hits: Vec<_> = top_vertices
            .iter()
            .copied()
            .filter(|&j| length(s.vertices[j].point - p) <= budget)
            .collect();
        if hits.len() != 1 || !matched.insert(hits[0]) {
            return Err(DOMAIN);
        }
        vertex_map.insert(i, hits[0]);
    }
    let mut edge_map = BTreeMap::new();
    let mut top_used = BTreeSet::new();
    let mut wall_used = BTreeSet::new();
    let mut vertical_used = BTreeSet::new();
    for coedge in base.wires.iter().flat_map(|w| &w.coedges) {
        let edge = &s.edges[coedge.edge];
        let [a, b] = edge.vertices;
        let [ta, tb] = [vertex_map[&a], vertex_map[&b]];
        let candidates: Vec<_> = top_edges
            .iter()
            .copied()
            .filter(|&i| {
                let e = &s.edges[i];
                (e.vertices == [ta, tb] || e.vertices == [tb, ta])
                    && translated_curve(
                        &edge.curve,
                        &e.curve,
                        delta,
                        e.vertices != [ta, tb],
                        budget,
                    )
                    .is_ok()
            })
            .collect();
        if candidates.len() != 1 {
            return Err(DOMAIN);
        }
        let top_edge = candidates[0];
        edge_map.insert(coedge.edge, top_edge);
        if !top_used.insert(top_edge) {
            return Err(DOMAIN);
        }
        translated_curve(
            &edge.curve,
            &s.edges[top_edge].curve,
            delta,
            s.edges[top_edge].vertices != [ta, tb],
            budget,
        )?;
        let walls: Vec<_> = s
            .shell
            .faces
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                !caps.contains(i)
                    && f.wires.len() == 1
                    && f.wires[0].coedges.iter().any(|c| c.edge == coedge.edge)
            })
            .map(|(i, _)| i)
            .collect();
        if walls.len() != 1 || !wall_used.insert(walls[0]) {
            return Err(DOMAIN);
        }
        let wall = &s.shell.faces[walls[0]];
        let wall_edges: BTreeSet<_> = wall.wires[0].coedges.iter().map(|c| c.edge).collect();
        if wall.wires[0].coedges.len() != 4
            || wall_edges.len() != 4
            || !wall_edges.contains(&top_edge)
        {
            return Err(DOMAIN);
        }
        for (lo, hi) in [(a, ta), (b, tb)] {
            let vertical: Vec<_> = wall_edges
                .iter()
                .copied()
                .filter(|&i| {
                    let e = &s.edges[i];
                    e.vertices == [lo, hi] || e.vertices == [hi, lo]
                })
                .collect();
            if vertical.len() != 1 {
                return Err(DOMAIN);
            }
            let id = vertical[0];
            let Curve::Line { a: pa, b: pb } = s.edges[id].curve else {
                return Err(DOMAIN);
            };
            let [va, vb] = s.edges[id].vertices;
            if length(pa - s.vertices[va].point) > budget
                || length(pb - s.vertices[vb].point) > budget
            {
                return Err(DOMAIN);
            }
            vertical_used.insert(id);
        }
        let tangent = match edge.curve {
            Curve::Line { a, b } => b - a,
            Curve::Arc {
                frame,
                radius,
                sweep,
            } => {
                let (sin, cos) = (sweep / 2.).sin_cos();
                (frame.axes()[1] * cos - frame.axes()[0] * sin) * radius
            }
            _ => return Err(DOMAIN),
        };
        let outward = (tangent
            * if coedge.forward {
                -f64::from(base.orientation)
            } else {
                f64::from(base.orientation)
            })
        .cross(frame.axes()[2])
        .normalized()?;
        match (&edge.curve, &wall.surface) {
            (Curve::Line { .. }, Surface::Plane { origin, u, v }) => {
                if length(u.cross(*v) * f64::from(wall.orientation) - outward) * scale > budget {
                    return Err(DOMAIN);
                }
                for i in [a, b, ta, tb] {
                    let d = (s.vertices[i].point - *origin).dot(u.cross(*v));
                    if !d.is_finite() || d.abs() > budget {
                        return Err(DOMAIN);
                    }
                }
            }
            (
                Curve::Arc {
                    frame: arc,
                    radius,
                    sweep,
                },
                Surface::Cylinder { .. } | Surface::FramedCylinder { .. },
            ) => {
                let (cylinder, r, h, _) = crate::circular_trims::surface_data(&wall.surface)?;
                let span = wall.cylinder_span()?;
                if (r - radius).abs() > budget
                    || (h - height).abs() > budget
                    || (span - sweep).abs() * r > budget
                    || length(cylinder.axes()[2].cross(frame.axes()[2])) * scale > budget
                {
                    return Err(DOMAIN);
                }
                let Curve::Arc { frame: other, .. } = s.edges[top_edge].curve else {
                    return Err(DOMAIN);
                };
                let mut levels = Vec::new();
                for center in [arc.origin(), other.origin()] {
                    let p = cylinder.local_point(center);
                    if !p.finite() || p.x.hypot(p.y) > budget {
                        return Err(DOMAIN);
                    }
                    levels.push(p.z);
                }
                levels.sort_by(f64::total_cmp);
                if levels[0].abs() > budget || (levels[1] - h).abs() > budget {
                    return Err(DOMAIN);
                }
                let mid = edge.curve.evaluate(*sweep / 2.) + delta * 0.5;
                let uv = wall.surface.parameters(mid);
                let normal = wall.surface.normal(uv[0]) * f64::from(wall.orientation);
                if length(normal - outward) * r > budget {
                    return Err(DOMAIN);
                }
            }
            _ => return Err(DOMAIN),
        }
    }
    if top_used.len() != count || wall_used.len() != count || vertical_used.len() != count {
        return Err(DOMAIN);
    }
    let mut matched_wires = BTreeSet::new();
    for (index, wire) in base.wires.iter().enumerate() {
        let edges: BTreeSet<_> = wire.coedges.iter().map(|c| edge_map[&c.edge]).collect();
        let matches: Vec<_> = top
            .wires
            .iter()
            .enumerate()
            .filter(|(_, w)| w.coedges.iter().map(|c| c.edge).collect::<BTreeSet<_>>() == edges)
            .map(|(i, _)| i)
            .collect();
        if matches.len() != 1
            || !matched_wires.insert(matches[0])
            || ((index == 0) != (matches[0] == 0))
        {
            return Err(DOMAIN);
        }
    }
    let volume = s.volume()?;
    let expected_volume = expected.volume()?;
    if !volume.is_finite()
        || volume <= 0.
        || !expected_volume.is_finite()
        || (volume - expected_volume).abs() > expected_volume.abs() * 1e-10
    {
        return Err(DOMAIN);
    }
    Ok(())
}
