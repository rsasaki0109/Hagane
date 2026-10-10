//! Read-only certificate for one quarter-arc blind cavity in a normal prism.
use crate::*;
use std::collections::BTreeSet;

const DOMAIN: Error =
    Error::Unsupported("bounded STEP blind cavity is outside the certified domain");
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
fn face_edges(face: &Face) -> BTreeSet<usize> {
    face.wires
        .iter()
        .flat_map(|w| &w.coedges)
        .map(|c| c.edge)
        .collect()
}
fn circle(edge: &Edge) -> Result<(Frame3, f64, f64)> {
    let Curve::Arc {
        frame,
        radius,
        sweep,
    } = edge.curve
    else {
        return Err(DOMAIN);
    };
    if !radius.is_finite()
        || radius <= 0.
        || (sweep - std::f64::consts::FRAC_PI_2).abs() > 64. * f64::EPSILON
    {
        return Err(DOMAIN);
    }
    Ok((frame, radius, sweep))
}
// Affine lines and trigonometric circle coefficients certify every parameter.
fn pcurve_identity(surface: &Surface, curve: &Curve, pcurve: &PCurve, budget: f64) -> Result<()> {
    match (surface, curve, pcurve) {
        (
            Surface::Plane { origin, u, v },
            Curve::Arc {
                frame,
                radius,
                sweep,
            },
            PCurve::Arc {
                center,
                radius: r,
                start_angle,
                sweep: psweep,
            },
        ) => {
            checked(
                4096.
                    * f64::EPSILON
                    * (r * start_angle.abs().max(sweep.abs())
                        + length(*u) * center[0].abs()
                        + length(*v) * center[1].abs()),
                budget / 8.,
            )?;
            let (sin, cos) = start_angle.sin_cos();
            checked(
                length(*origin + *u * center[0] + *v * center[1] - frame.origin())
                    + length((*u * cos + *v * sin) * *r - frame.axes()[0] * *radius)
                    + length((*v * cos - *u * sin) * *r - frame.axes()[1] * *radius)
                    + (psweep - sweep).abs() * r.max(*radius),
                budget,
            )
        }
        (
            Surface::Cylinder { .. } | Surface::FramedCylinder { .. },
            Curve::Arc {
                frame: arc,
                radius,
                sweep,
            },
            PCurve::Affine { origin, direction },
        ) => {
            let (frame, r, _, _) = crate::circular_trims::surface_data(surface)?;
            checked(
                4096.
                    * f64::EPSILON
                    * (r * origin[0].abs().max(sweep.abs())
                        + origin[1].abs()
                        + direction[1].abs() * sweep.abs()),
                budget / 8.,
            )?;
            if direction[0] != 1. && direction[0] != -1. {
                return Err(DOMAIN);
            }
            let (sin, cos) = origin[0].sin_cos();
            checked(
                length(frame.origin() + frame.axes()[2] * origin[1] - arc.origin())
                    + length(
                        (frame.axes()[0] * cos + frame.axes()[1] * sin) * r
                            - arc.axes()[0] * *radius,
                    )
                    + length(
                        (frame.axes()[1] * cos - frame.axes()[0] * sin) * (r * direction[0])
                            - arc.axes()[1] * *radius,
                    )
                    + direction[1].abs() * sweep.abs(),
                budget,
            )
        }
        (
            Surface::Cylinder { .. } | Surface::FramedCylinder { .. },
            Curve::Line { a, b },
            PCurve::Affine { origin, direction },
        ) => {
            let (frame, r, _, _) = crate::circular_trims::surface_data(surface)?;
            checked(
                4096. * f64::EPSILON * (r * origin[0].abs() + origin[1].abs() + direction[1].abs()),
                budget / 8.,
            )?;
            if direction[0] != 0. {
                return Err(DOMAIN);
            }
            checked(
                length(surface.try_evaluate(origin[0], origin[1])? - *a)
                    + length(frame.axes()[2] * direction[1] - (*b - *a)),
                budget,
            )
        }
        _ => Err(DOMAIN),
    }
}
fn curve_identity(a: &Curve, b: &Curve, reverse: bool, budget: f64) -> Result<()> {
    match (a, b) {
        (Curve::Line { a: a0, b: a1 }, Curve::Line { a: b0, b: b1 }) => {
            let (b0, b1) = if reverse { (*b1, *b0) } else { (*b0, *b1) };
            checked(length(*a0 - b0) + length(*a1 - b1), budget)
        }
        (
            Curve::Arc {
                frame: a,
                radius: ar,
                sweep: asw,
            },
            Curve::Arc {
                frame: b,
                radius: br,
                sweep: bsw,
            },
        ) => {
            checked((asw - bsw).abs() * ar.max(*br), budget)?;
            let (bc, bs) = if reverse {
                let (sin, cos) = bsw.sin_cos();
                (
                    b.axes()[0] * cos + b.axes()[1] * sin,
                    b.axes()[0] * sin - b.axes()[1] * cos,
                )
            } else {
                (b.axes()[0], b.axes()[1])
            };
            checked(
                length(a.origin() - b.origin())
                    + length(a.axes()[0] * *ar - bc * *br)
                    + length(a.axes()[1] * *ar - bs * *br),
                budget,
            )
        }
        _ => Err(DOMAIN),
    }
}

/// Certify actual imported geometry without replacing it with rebuilt geometry.
pub(crate) fn certify_validated_normal_prism_blind(s: &Solid, tol: Tolerance) -> Result<()> {
    Tolerance::new(tol.linear)?;
    s.validate(tol)?;
    if s.shell.faces.len() > 135 || s.edges.len() > 396 || s.vertices.len() > 264 {
        return Err(DOMAIN);
    }
    let mut admitted = 0;
    for (floor, face) in s.shell.faces.iter().enumerate() {
        if !matches!(face.surface, Surface::Plane { .. })
            || face.wires.len() != 1
            || face.wires[0].coedges.len() != 4
        {
            continue;
        }
        if face.wires[0]
            .coedges
            .iter()
            .any(|c| circle(&s.edges[c.edge]).is_err())
        {
            continue;
        }
        if certify_candidate(s, tol, floor).is_ok() {
            admitted += 1;
        }
    }
    if admitted == 1 {
        Ok(())
    } else {
        Err(DOMAIN)
    }
}
fn certify_candidate(s: &Solid, tol: Tolerance, floor: usize) -> Result<()> {
    let floor_edges = face_edges(&s.shell.faces[floor]);
    if floor_edges.len() != 4 {
        return Err(DOMAIN);
    }
    let mut walls = BTreeSet::new();
    for edge in &floor_edges {
        let uses: Vec<_> = s
            .shell
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| face_edges(f).contains(edge))
            .map(|(i, _)| i)
            .collect();
        if uses.len() != 2 {
            return Err(DOMAIN);
        }
        let wall = *uses.iter().find(|i| **i != floor).ok_or(DOMAIN)?;
        if !matches!(
            s.shell.faces[wall].surface,
            Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
        ) || s.shell.faces[wall].wires.len() != 1
            || s.shell.faces[wall].wires[0].coedges.len() != 4
        {
            return Err(DOMAIN);
        }
        walls.insert(wall);
    }
    if walls.len() != 4 {
        return Err(DOMAIN);
    }
    let mut deleted_faces = walls.clone();
    deleted_faces.insert(floor);
    let deleted_edges: BTreeSet<_> = deleted_faces
        .iter()
        .flat_map(|i| face_edges(&s.shell.faces[*i]))
        .collect();
    let deleted_vertices: BTreeSet<_> = deleted_edges
        .iter()
        .flat_map(|e| s.edges[*e].vertices)
        .collect();
    if deleted_edges.len() != 12 || deleted_vertices.len() != 8 {
        return Err(DOMAIN);
    }
    let mut entries = Vec::new();
    for (f, face) in s.shell.faces.iter().enumerate() {
        if deleted_faces.contains(&f) {
            continue;
        }
        for (w, wire) in face.wires.iter().enumerate() {
            let intersect: Vec<_> = wire
                .coedges
                .iter()
                .filter(|c| deleted_edges.contains(&c.edge))
                .collect();
            if intersect.is_empty() {
                continue;
            }
            if w == 0
                || wire.coedges.len() != 4
                || intersect.len() != 4
                || !matches!(face.surface, Surface::Plane { .. })
                || intersect.iter().any(|c| circle(&s.edges[c.edge]).is_err())
            {
                return Err(DOMAIN);
            }
            entries.push((f, w));
        }
    }
    if entries.len() != 1 {
        return Err(DOMAIN);
    }
    let (entry, wire) = entries[0];
    for (i, e) in s.edges.iter().enumerate() {
        if !deleted_edges.contains(&i) && e.vertices.iter().any(|v| deleted_vertices.contains(v)) {
            return Err(DOMAIN);
        }
    }
    let mut restored = s.clone();
    let mut vertex_map = vec![usize::MAX; s.vertices.len()];
    restored.vertices = s
        .vertices
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            if deleted_vertices.contains(&i) {
                None
            } else {
                Some((i, v.clone()))
            }
        })
        .enumerate()
        .map(|(n, (i, v))| {
            vertex_map[i] = n;
            v
        })
        .collect();
    let mut edge_map = vec![usize::MAX; s.edges.len()];
    restored.edges = s
        .edges
        .iter()
        .enumerate()
        .filter(|(i, _)| !deleted_edges.contains(i))
        .enumerate()
        .map(|(n, (i, e))| {
            edge_map[i] = n;
            let mut e = e.clone();
            e.vertices = e.vertices.map(|v| vertex_map[v]);
            e
        })
        .collect();
    restored.shell.faces = s
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| !deleted_faces.contains(i))
        .map(|(i, f)| {
            let mut f = f.clone();
            if i == entry {
                f.wires.remove(wire);
            }
            for w in &mut f.wires {
                for c in &mut w.coedges {
                    c.edge = edge_map[c.edge];
                }
            }
            f
        })
        .collect();
    restored.validate(tol)?;
    let Surface::Plane { u, v, .. } = s.shell.faces[entry].surface else {
        return Err(DOMAIN);
    };
    let axis = u.cross(v).normalized()? * f64::from(s.shell.faces[entry].orientation);
    crate::arc_line_prism_validation::recognize_validated_normal_prism(&restored, axis, tol)?;
    let first = s.shell.faces[entry].wires[wire].coedges[0].edge;
    let (entry_frame, radius, _) = circle(&s.edges[first])?;
    let (floor_frame, _, _) = circle(&s.edges[*floor_edges.first().ok_or(DOMAIN)?])?;
    let depth = (entry_frame.origin() - floor_frame.origin()).dot(axis);
    let witness = blind_bore_normal_prism(
        &restored,
        entry_frame.origin(),
        radius,
        depth,
        axis,
        NormalPrismBoreEntry::Positive,
        GeometryTolerance::new(tol.linear, 1e-10, 0.)?,
    )?;
    let body = witness.kept();
    let mut world = length(s.bounds().max - s.bounds().min);
    for e in &s.edges {
        match e.curve {
            Curve::Line { a, b } => {
                for p in [a, b] {
                    world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
                }
            }
            Curve::Arc { frame, radius, .. } => {
                let p = frame.origin();
                world = world
                    .max(p.x.abs())
                    .max(p.y.abs())
                    .max(p.z.abs())
                    .max(radius);
            }
            _ => return Err(DOMAIN),
        }
    }
    for f in &s.shell.faces {
        let p = match f.surface {
            Surface::Plane { origin, .. } => origin,
            Surface::Cylinder { center, .. } => center,
            Surface::FramedCylinder { frame, .. } => frame.origin(),
            _ => return Err(DOMAIN),
        };
        world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
    }
    let arithmetic = 4096. * f64::EPSILON * world;
    checked(arithmetic, tol.linear / 256.)?;
    let budget = tol.linear / 32. - arithmetic;
    let mut matching = vec![usize::MAX; s.edges.len()];
    let mut reversals = vec![false; s.edges.len()];
    let mut vertex_matching = vec![usize::MAX; s.vertices.len()];
    let mut witness_vertices = BTreeSet::new();
    let mut used = BTreeSet::new();
    for old in &deleted_edges {
        let mut candidates = Vec::new();
        for new in restored.edges.len()..body.edges.len() {
            if used.contains(&new) {
                continue;
            }
            for reverse in [false, true] {
                if curve_identity(
                    &s.edges[*old].curve,
                    &body.edges[new].curve,
                    reverse,
                    budget,
                )
                .is_ok()
                {
                    candidates.push((new, reverse));
                }
            }
        }
        if candidates.len() != 1 {
            return Err(DOMAIN);
        }
        let (new, reverse) = candidates[0];
        matching[*old] = new;
        reversals[*old] = reverse;
        used.insert(new);
        let mut target_vertices = body.edges[new].vertices;
        if reverse {
            target_vertices.reverse();
        }
        for (original, target) in s.edges[*old].vertices.into_iter().zip(target_vertices) {
            if vertex_matching[original] == usize::MAX {
                if !witness_vertices.insert(target) {
                    return Err(DOMAIN);
                }
                vertex_matching[original] = target;
            } else if vertex_matching[original] != target {
                return Err(DOMAIN);
            }
        }
    }
    for old in &deleted_faces {
        let edges: BTreeSet<_> = face_edges(&s.shell.faces[*old])
            .iter()
            .map(|e| matching[*e])
            .collect();
        let targets: Vec<_> = body
            .shell
            .faces
            .iter()
            .enumerate()
            .skip(restored.shell.faces.len())
            .filter(|(_, f)| face_edges(f) == edges)
            .collect();
        if targets.len() != 1 {
            return Err(DOMAIN);
        }
        let (_, target) = targets[0];
        let original = &s.shell.faces[*old];
        for c in original.wires.iter().flat_map(|w| &w.coedges) {
            let target_coedge = target
                .wires
                .iter()
                .flat_map(|w| &w.coedges)
                .find(|t| t.edge == matching[c.edge])
                .ok_or(DOMAIN)?;
            let original_forward = c.forward == (original.orientation > 0);
            let witness_forward = target_coedge.forward == (target.orientation > 0);
            if original_forward != (witness_forward != reversals[c.edge]) {
                return Err(DOMAIN);
            }
        }
        match (&original.surface, &target.surface) {
            (
                Surface::Plane {
                    origin: a,
                    u: au,
                    v: av,
                },
                Surface::Plane {
                    origin: b,
                    u: bu,
                    v: bv,
                },
            ) => {
                let an = au.cross(*av).normalized()? * f64::from(original.orientation);
                let bn = bu.cross(*bv).normalized()? * f64::from(target.orientation);
                checked(length(an - bn) * world + (*a - *b).dot(bn).abs(), budget)?;
            }
            (
                Surface::Cylinder { .. } | Surface::FramedCylinder { .. },
                Surface::Cylinder { .. } | Surface::FramedCylinder { .. },
            ) => {
                let (a, ar, ah, _) = crate::circular_trims::surface_data(&original.surface)?;
                let (b, br, bh, _) = crate::circular_trims::surface_data(&target.surface)?;
                checked(
                    (ar - br).abs()
                        + (ah - bh).abs()
                        + length(a.axes()[2].cross(b.axes()[2])) * world,
                    budget,
                )?;
                let a0 = a.origin();
                let a1 = a0 + a.axes()[2] * ah;
                let b0 = b.origin();
                let b1 = b0 + b.axes()[2] * bh;
                checked(
                    (length(a0 - b0) + length(a1 - b1)).min(length(a0 - b1) + length(a1 - b0)),
                    budget,
                )?;
                for c in &original.wires[0].coedges {
                    if let Curve::Arc { sweep, .. } = s.edges[c.edge].curve {
                        let p = s.edges[c.edge].curve.try_evaluate(sweep / 2.)?;
                        let radial =
                            p - a.origin() - a.axes()[2] * (p - a.origin()).dot(a.axes()[2]);
                        let uv = original.surface.try_parameters(p)?;
                        let normal =
                            original.surface.normal(uv[0]) * f64::from(original.orientation);
                        checked(length(normal + radial.normalized()?) * ar, budget)?;
                    }
                }
            }
            _ => return Err(DOMAIN),
        }
        for w in &original.wires {
            for c in &w.coedges {
                pcurve_identity(&original.surface, &s.edges[c.edge].curve, &c.pcurve, budget)?;
            }
        }
    }
    for c in &s.shell.faces[entry].wires[wire].coedges {
        pcurve_identity(
            &s.shell.faces[entry].surface,
            &s.edges[c.edge].curve,
            &c.pcurve,
            budget,
        )?;
    }
    let actual_volume = s.volume()?;
    let witness_volume = body.volume()?;
    if !actual_volume.is_finite()
        || actual_volume <= 0.
        || !witness_volume.is_finite()
        || witness_volume <= 0.
    {
        return Err(DOMAIN);
    }
    checked(
        (actual_volume - witness_volume).abs(),
        8192. * f64::EPSILON * actual_volume.max(witness_volume),
    )?;
    Ok(())
}
